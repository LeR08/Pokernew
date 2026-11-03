//! Review of a tournament hand: was hero's push, call or fold the best push/fold play?
//!
//! The spot is rebuilt from the hand history: the stacks at the start of the
//! hand in preflop order, the blinds and antes, and hero's decision:
//! * **push**: everybody before hero folded and hero moved all-in (or folded);
//! * **call**: exactly one player moved all-in first, the others folded, and
//!   hero called or folded.
//!
//! Other lines (limps, raises that are not all-in, several all-ins) are not
//! push/fold decisions and are skipped, as are decisions above `max_bb`.
//! The equilibrium is solved for everybody; hero's value of each option is
//! read at hero's node, for hero's hand class.

use lps_core_model::{ActionKind, Hand, Street};
use lps_equity::combo::{class_name, Combo};
use serde::Serialize;

use crate::pushfold::{solve, Decision, Options, Spot};

#[derive(Debug, Clone)]
pub struct ReviewModel {
    /// Prizes (empty: chip EV only).
    pub payouts: Vec<f64>,
    /// Stacks of the players at other tables.
    pub others: Vec<f64>,
    /// Share of a knocked-out player's bounty paid in cash (0.5 for progressive KO).
    pub bounty_share: f64,
    /// Count bounties (PKO).
    pub use_bounties: bool,
    pub max_bb: f64,
    pub fgs_depth: u8,
}

impl Default for ReviewModel {
    fn default() -> Self {
        ReviewModel {
            payouts: vec![],
            others: vec![],
            bounty_share: 0.5,
            use_bounties: false,
            max_bb: 20.0,
            fgs_depth: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Push,
    Call,
    Fold,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub spot: Spot,
    /// Seat names in spot order.
    pub names: Vec<String>,
    pub positions: Vec<String>,
    pub hero: usize,
    /// The all-in player hero faced (call decisions).
    pub pusher: Option<usize>,
    pub hero_cards: String,
    pub hero_class: String,
    pub played: Choice,
    pub best: Choice,
    /// Hero's value of each option, chips.
    pub chips_act: f64,
    pub chips_fold: f64,
    /// Same in money (ICM), when payouts are known.
    pub money_act: Option<f64>,
    pub money_fold: Option<f64>,
    /// Value lost by the play made (≥ 0), chips and money.
    pub loss_chips: f64,
    pub loss_money: Option<f64>,
    pub effective_bb: f64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ReviewError {
    #[error("not a tournament hand")]
    NotTournament,
    #[error("hero's cards are unknown")]
    NoHeroCards,
    #[error("hero's decision is not a push/fold decision")]
    NotPushFold,
    #[error("hero's effective stack is above {0} bb")]
    TooDeep(f64),
    #[error("{0}")]
    Solve(String),
}

/// Players dealt in, in preflop order (after the big blind round to the big blind).
fn preflop_order(hand: &Hand) -> Option<Vec<usize>> {
    let posts = |k: ActionKind| {
        hand.actions
            .iter()
            .find(|a| a.street == Street::Preflop && a.kind == k)
            .map(|a| a.player)
    };
    let bb = posts(ActionKind::PostBigBlind)?;
    let sb = posts(ActionKind::PostSmallBlind)?;
    let mut seats: Vec<usize> = (0..hand.players.len()).collect();
    seats.sort_by_key(|&i| hand.players[i].seat);
    let start = seats.iter().position(|&i| i == bb)?;
    let order: Vec<usize> = (1..=seats.len())
        .map(|k| seats[(start + k) % seats.len()])
        .collect();
    // order ends with BB; SB must be just before it
    (order.len() >= 2 && order[order.len() - 2] == sb).then_some(order)
}

pub fn review(hand: &Hand, model: &ReviewModel) -> Result<Review, ReviewError> {
    if !hand.game_type.is_tournament() {
        return Err(ReviewError::NotTournament);
    }
    let order = preflop_order(hand).ok_or(ReviewError::NotPushFold)?;
    let hero_i = hand
        .players
        .iter()
        .position(|p| p.is_hero)
        .ok_or(ReviewError::NoHeroCards)?;
    let hero_p = &hand.players[hero_i];
    if hero_p.hole_cards.len() != 2 {
        return Err(ReviewError::NoHeroCards);
    }
    let hero = order
        .iter()
        .position(|&i| i == hero_i)
        .ok_or(ReviewError::NotPushFold)?;

    // Walk the preflop actions up to hero's first decision.
    let mut shover: Option<usize> = None;
    let mut played = None;
    for a in hand
        .actions
        .iter()
        .filter(|a| a.street == Street::Preflop && !a.kind.is_post())
    {
        if a.player == hero_i {
            played = Some(match (a.kind, shover) {
                (ActionKind::Fold, _) => Choice::Fold,
                (ActionKind::Raise | ActionKind::Bet, None) if a.is_all_in => Choice::Push,
                (ActionKind::Call, Some(_)) => Choice::Call,
                // hero's raise all-in over a shove is a call for the push/fold model
                (ActionKind::Raise, Some(_)) if a.is_all_in => Choice::Call,
                _ => return Err(ReviewError::NotPushFold),
            });
            break;
        }
        match a.kind {
            ActionKind::Fold => {}
            ActionKind::Raise | ActionKind::Bet if a.is_all_in && shover.is_none() => {
                shover = Some(a.player)
            }
            _ => return Err(ReviewError::NotPushFold),
        }
    }
    let played = played.ok_or(ReviewError::NotPushFold)?;
    let pusher = shover.map(|s| order.iter().position(|&i| i == s).unwrap());
    if played == Choice::Fold && pusher.is_none() && hero == order.len() - 1 {
        return Err(ReviewError::NotPushFold); // BB folding to nothing does not happen
    }

    let bb = hand.big_blind as f64;
    let stacks: Vec<f64> = order
        .iter()
        .map(|&i| hand.players[i].stack as f64)
        .collect();
    let others_max = stacks
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != hero)
        .map(|(_, s)| *s)
        .fold(0.0, f64::max);
    let effective_bb = stacks[hero].min(match pusher {
        Some(p) => stacks[p],
        None => others_max,
    }) / bb;
    if effective_bb > model.max_bb {
        return Err(ReviewError::TooDeep(model.max_bb));
    }

    let bounties: Vec<f64> = if model.use_bounties {
        order
            .iter()
            .map(|&i| hand.players[i].bounty_cents.unwrap_or(0) as f64 / 100.0 * model.bounty_share)
            .collect()
    } else {
        vec![]
    };
    let base = Spot {
        stacks: stacks.clone(),
        sb: hand.small_blind as f64,
        bb,
        ante: hand.ante as f64,
        bb_ante: false,
        others: vec![],
        payouts: vec![],
        bounties: vec![],
        fgs_depth: 0,
    };
    let decision = match pusher {
        None => Decision::Push { p: hero },
        Some(p) => Decision::Call { p, q: hero },
    };
    let c = Combo::new(hero_p.hole_cards[0].index(), hero_p.hole_cards[1].index());
    let class = c.class();
    let opts = Options {
        max_iterations: 600,
        target: 0.002,
        locks: vec![],
    };
    let read = |spot: &Spot| -> Result<(f64, f64), ReviewError> {
        let sol = solve(spot, &opts).map_err(|e| ReviewError::Solve(e.to_string()))?;
        let node = sol.node(decision).ok_or(ReviewError::NotPushFold)?;
        Ok((node.ev_act[class], node.ev_fold[class]))
    };
    let (chips_act, chips_fold) = read(&base)?;
    let money = if model.payouts.is_empty() {
        None
    } else {
        let spot = Spot {
            payouts: model.payouts.clone(),
            others: model.others.clone(),
            bounties: bounties.clone(),
            fgs_depth: model.fgs_depth,
            ..base.clone()
        };
        Some(read(&spot)?)
    };
    let act = if pusher.is_some() {
        Choice::Call
    } else {
        Choice::Push
    };
    // The best play follows money when known, chips otherwise.
    let (a, f) = money.unwrap_or((chips_act, chips_fold));
    let best = if a > f { act } else { Choice::Fold };
    let loss = |a: f64, f: f64| {
        if played == Choice::Fold {
            (a - f).max(0.0)
        } else {
            (f - a).max(0.0)
        }
    };
    let spot = Spot {
        payouts: model.payouts.clone(),
        others: model.others.clone(),
        bounties,
        fgs_depth: model.fgs_depth,
        ..base
    };
    Ok(Review {
        names: order
            .iter()
            .map(|&i| hand.players[i].name.clone())
            .collect(),
        positions: order
            .iter()
            .map(|&i| {
                hand.players[i]
                    .position
                    .map(|p| p.as_str().to_string())
                    .unwrap_or_default()
            })
            .collect(),
        spot,
        hero,
        pusher,
        hero_cards: hero_p.hole_cards.iter().map(|c| format!("{c}")).collect(),
        hero_class: class_name(class),
        played,
        best,
        chips_act,
        chips_fold,
        money_act: money.map(|m| m.0),
        money_fold: money.map(|m| m.1),
        loss_chips: loss(chips_act, chips_fold),
        loss_money: money.map(|(a, f)| loss(a, f)),
        effective_bb,
    })
}

/// Reviews many hands in parallel; returns (index, review) for the push/fold decisions.
pub fn review_many(hands: &[Hand], model: &ReviewModel) -> Vec<(usize, Review)> {
    use rayon::prelude::*;
    hands
        .par_iter()
        .enumerate()
        .filter_map(|(i, h)| review(h, model).ok().map(|r| (i, r)))
        .collect()
}
