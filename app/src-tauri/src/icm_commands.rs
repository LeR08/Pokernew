//! Commands of the ICM & push/fold module. Study module: every command refuses
//! to run in play mode.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use lps_icm::review::{review, review_many, Review, ReviewModel};
use lps_icm::{bubble_factor, chip_ev, icm, risk_premium, solve, Options, Solution, Spot};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::commands::CommandError;
use crate::hud::HudService;
use crate::state::AppState;

type Res<T> = Result<T, CommandError>;

fn study_guard(hud: &HudService) -> Res<()> {
    if hud.play_mode.load(Ordering::SeqCst) {
        return Err(CommandError::from(
            "study modules are locked while a poker table is open".to_string(),
        ));
    }
    Ok(())
}

fn e(x: impl std::fmt::Display) -> CommandError {
    CommandError::from(x.to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IcmResult {
    pub icm: Vec<f64>,
    pub chip_ev: Vec<f64>,
    /// bubble[i][j]: bubble factor of i against j.
    pub bubble: Vec<Vec<f64>>,
    pub risk: Vec<Vec<f64>>,
    pub prize_pool: f64,
}

pub fn icm_table(stacks: &[f64], payouts: &[f64]) -> Res<IcmResult> {
    if stacks.is_empty()
        || stacks.len() > lps_icm::model::MAX_PLAYERS
        || stacks.iter().any(|s| !s.is_finite() || *s < 0.0)
    {
        return Err(e("1 to 16 players with valid stacks"));
    }
    let n = stacks.len();
    let bubble: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| bubble_factor(stacks, payouts, i, j))
                .collect()
        })
        .collect();
    let risk = bubble
        .iter()
        .map(|r| r.iter().map(|&b| risk_premium(b)).collect())
        .collect();
    Ok(IcmResult {
        icm: icm(stacks, payouts),
        chip_ev: chip_ev(stacks, payouts),
        bubble: bubble
            .iter()
            .map(|r| {
                r.iter()
                    .map(|b| if b.is_finite() { *b } else { 999.0 })
                    .collect()
            })
            .collect(),
        risk,
        prize_pool: payouts.iter().take(n).sum(),
    })
}

#[tauri::command]
pub fn icm_calc(
    hud: State<'_, Arc<HudService>>,
    stacks: Vec<f64>,
    payouts: Vec<f64>,
) -> Res<IcmResult> {
    study_guard(&hud)?;
    icm_table(&stacks, &payouts)
}

#[tauri::command]
pub async fn pushfold_solve(
    hud: State<'_, Arc<HudService>>,
    spot: Spot,
    options: Options,
) -> Res<Solution> {
    study_guard(&hud)?;
    tauri::async_runtime::spawn_blocking(move || solve(&spot, &options).map_err(e))
        .await
        .map_err(e)?
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRequest {
    #[serde(default)]
    pub payouts: Vec<f64>,
    #[serde(default)]
    pub others: Vec<f64>,
    #[serde(default)]
    pub use_bounties: bool,
    pub bounty_share: Option<f64>,
    pub max_bb: Option<f64>,
    #[serde(default)]
    pub fgs_depth: u8,
}

impl ReviewRequest {
    fn model(&self) -> ReviewModel {
        ReviewModel {
            payouts: self.payouts.clone(),
            others: self.others.clone(),
            use_bounties: self.use_bounties,
            bounty_share: self.bounty_share.unwrap_or(0.5).clamp(0.0, 1.0),
            max_bb: self.max_bb.unwrap_or(20.0),
            fgs_depth: self.fgs_depth.min(3),
        }
    }
}

fn svc(s: &AppState) -> Res<&crate::import_service::ImportService> {
    s.import.as_deref().ok_or_else(|| e("database unavailable"))
}

#[tauri::command]
pub async fn icm_review_hand(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    hand_id: i64,
    req: ReviewRequest,
) -> Res<Review> {
    study_guard(&hud)?;
    let hand = svc(&s)?
        .with_reader(|c| lps_db::load::load_hand(c, hand_id).map(|(h, _)| h))
        .map_err(e)?;
    let model = req.model();
    tauri::async_runtime::spawn_blocking(move || review(&hand, &model).map_err(e))
        .await
        .map_err(e)?
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRow {
    pub hand_id: i64,
    pub played_at: i64,
    pub level: Option<u32>,
    pub review: Review,
}

/// Every hero push/fold decision of a tournament (≤ max_bb), sorted by value lost.
#[tauri::command]
pub async fn icm_review_tournament(
    hud: State<'_, Arc<HudService>>,
    s: State<'_, AppState>,
    tournament_id: i64,
    req: ReviewRequest,
) -> Res<Vec<BatchRow>> {
    study_guard(&hud)?;
    let hands = svc(&s)?
        .with_reader(|c| {
            let mut st = c
                .prepare("SELECT id FROM hands WHERE tournament_id = ?1 ORDER BY played_at, id")
                .map_err(lps_db::DbError::Sqlite)?;
            let ids: Vec<i64> = st
                .query_map([tournament_id], |r| r.get(0))
                .map_err(lps_db::DbError::Sqlite)?
                .collect::<Result<_, _>>()
                .map_err(lps_db::DbError::Sqlite)?;
            ids.into_iter()
                .map(|id| lps_db::load::load_hand(c, id).map(|(h, _)| (id, h)))
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(e)?;
    let model = req.model();
    tauri::async_runtime::spawn_blocking(move || {
        let only: Vec<_> = hands.iter().map(|(_, h)| h.clone()).collect();
        let mut rows: Vec<BatchRow> = review_many(&only, &model)
            .into_iter()
            .map(|(i, r)| BatchRow {
                hand_id: hands[i].0,
                played_at: hands[i].1.played_at,
                level: hands[i].1.level,
                review: r,
            })
            .collect();
        let loss = |r: &BatchRow| {
            r.review
                .loss_money
                .map(|m| m * 1e6)
                .unwrap_or(r.review.loss_chips)
        };
        rows.sort_by(|a, b| loss(b).total_cmp(&loss(a)));
        rows
    })
    .await
    .map_err(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icm_table_sums() {
        let r = icm_table(&[5000.0, 3000.0, 2000.0], &[50.0, 30.0, 20.0]).unwrap();
        assert!((r.icm.iter().sum::<f64>() - 100.0).abs() < 1e-9);
        assert_eq!(r.bubble.len(), 3);
        assert!(icm_table(&[], &[1.0]).is_err());
    }
}
