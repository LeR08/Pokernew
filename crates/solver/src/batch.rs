//! Batch solving: one spot over a list of flops, every solution cached, and a
//! report of the first decision (OOP) on each flop.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use lps_equity::texture::{flop_classes, FlopFilter};
use lps_evaluator::card_str;
use serde::Serialize;

use crate::cache;
use crate::job::{solve_now, SolveParams};
use crate::spot::SpotConfig;
use crate::view::{view, ActionInfo};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRow {
    pub flop: String,
    pub hash: String,
    pub exploitability_pct: f64,
    pub actions: Vec<ActionInfo>,
    /// OOP's overall frequency of each first action.
    pub totals: Vec<f32>,
    pub ev_oop: f32,
    pub equity_oop: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgress {
    pub total: usize,
    pub done: usize,
    pub current: Option<String>,
    pub running: bool,
    pub rows: Vec<BatchRow>,
    pub errors: Vec<String>,
}

pub struct Batch {
    progress: Mutex<BatchProgress>,
    cancel: AtomicBool,
}

/// Representative flops (one per class of suit-equivalent flops) matching a filter.
pub fn flops(filter: &FlopFilter) -> Vec<String> {
    flop_classes()
        .iter()
        .filter(|f| filter.matches(&f.cards))
        .map(|f| f.cards.iter().map(|&c| card_str(c)).collect())
        .collect()
}

impl Batch {
    pub fn start(
        spot: SpotConfig,
        params: SolveParams,
        flops: Vec<String>,
        name: String,
        dir: PathBuf,
        now: i64,
    ) -> Arc<Batch> {
        let b = Arc::new(Batch {
            progress: Mutex::new(BatchProgress {
                total: flops.len(),
                done: 0,
                current: None,
                running: true,
                rows: vec![],
                errors: vec![],
            }),
            cancel: AtomicBool::new(false),
        });
        let me = b.clone();
        std::thread::Builder::new()
            .name("lps-solver-batch".into())
            .spawn(move || {
                for flop in flops {
                    if me.cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    me.progress.lock().unwrap().current = Some(flop.clone());
                    let s = SpotConfig {
                        board: flop.clone(),
                        ..spot.clone()
                    };
                    let res = solve_now(&s, &params, &[]).and_then(|(mut game, e)| {
                        let entry = cache::save(
                            &dir,
                            &format!("{name} · {flop}"),
                            &s,
                            &[],
                            &game,
                            e,
                            params.max_iterations,
                            now,
                        )
                        .map_err(crate::spot::SpotError::Engine)?;
                        let v = view(&mut game, &[]).map_err(crate::spot::SpotError::Engine)?;
                        Ok(BatchRow {
                            flop: flop.clone(),
                            hash: entry.hash,
                            exploitability_pct: e,
                            actions: v.actions.clone(),
                            totals: v.totals.clone(),
                            ev_oop: v.players.first().map(|p| p.ev).unwrap_or(0.0),
                            equity_oop: v.players.first().map(|p| p.equity).unwrap_or(0.0),
                        })
                    });
                    let mut p = me.progress.lock().unwrap();
                    match res {
                        Ok(row) => p.rows.push(row),
                        Err(e) => p.errors.push(format!("{flop}: {e}")),
                    }
                    p.done += 1;
                }
                let mut p = me.progress.lock().unwrap();
                p.running = false;
                p.current = None;
            })
            .expect("batch thread");
        b
    }

    pub fn progress(&self) -> BatchProgress {
        self.progress.lock().unwrap().clone()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flop_lists() {
        assert_eq!(flops(&FlopFilter::default()).len(), 1755);
        let mono = flops(&FlopFilter {
            suits: Some("monotone".into()),
            ..Default::default()
        });
        assert_eq!(mono.len(), 286);
    }
}
