//! Per-player, per-hand counters computed at import time.
//!
//! Every stat is `SUM(counter_a) / SUM(counter_b)` over `hand_player_stats`,
//! so reports never re-scan actions. The list is declared (with French and
//! English descriptions) in `config/defaults/counters.toml`; [`COUNTERS`]
//! must match it exactly — a test enforces this. Definitions are in
//! `docs/counters.md`.

use lps_core_model::{ActionKind, Hand, Position, Street};

/// Counter ids, in column order.
pub const COUNTERS: &[&str] = &[
    "cnt_hands",
    "cnt_vpip",
    "cnt_pfr",
    "cnt_limp",
    "opp_3bet",
    "cnt_3bet",
    "opp_fold_to_3bet",
    "cnt_fold_to_3bet",
    "opp_steal",
    "cnt_steal",
    "opp_fold_to_steal",
    "cnt_fold_to_steal",
    "cnt_saw_flop",
    "opp_cbet_flop",
    "cnt_cbet_flop",
    "opp_fold_to_cbet_flop",
    "cnt_fold_to_cbet_flop",
    "cnt_wtsd",
    "cnt_wsd",
    "cnt_wwsf",
    "cnt_postflop_bets_raises",
    "cnt_postflop_calls",
    "cnt_postflop_folds",
    "cnt_allin_preflop",
    "cnt_won_hand",
];

/// Counters of one player in one hand, keyed by [`COUNTERS`] order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCounters {
    pub hands: i64,
    pub vpip: i64,
    pub pfr: i64,
    pub limp: i64,
    pub opp_3bet: i64,
    pub three_bet: i64,
    pub opp_fold_to_3bet: i64,
    pub fold_to_3bet: i64,
    pub opp_steal: i64,
    pub steal: i64,
    pub opp_fold_to_steal: i64,
    pub fold_to_steal: i64,
    pub saw_flop: i64,
    pub opp_cbet_flop: i64,
    pub cbet_flop: i64,
    pub opp_fold_to_cbet_flop: i64,
    pub fold_to_cbet_flop: i64,
    pub wtsd: i64,
    pub wsd: i64,
    pub wwsf: i64,
    pub postflop_bets_raises: i64,
    pub postflop_calls: i64,
    pub postflop_folds: i64,
    pub allin_preflop: i64,
    pub won_hand: i64,
}

impl PlayerCounters {
    /// Values in [`COUNTERS`] order.
    pub fn values(&self) -> [i64; 25] {
        [
            self.hands,
            self.vpip,
            self.pfr,
            self.limp,
            self.opp_3bet,
            self.three_bet,
            self.opp_fold_to_3bet,
            self.fold_to_3bet,
            self.opp_steal,
            self.steal,
            self.opp_fold_to_steal,
            self.fold_to_steal,
            self.saw_flop,
            self.opp_cbet_flop,
            self.cbet_flop,
            self.opp_fold_to_cbet_flop,
            self.fold_to_cbet_flop,
            self.wtsd,
            self.wsd,
            self.wwsf,
            self.postflop_bets_raises,
            self.postflop_calls,
            self.postflop_folds,
            self.allin_preflop,
            self.won_hand,
        ]
    }
}

/// Computes the counters of every player of `hand` (same order as `hand.players`).
pub fn compute(hand: &Hand) -> Vec<PlayerCounters> {
    let n = hand.players.len();
    let mut c = vec![PlayerCounters { hands: 1, ..Default::default() }; n];
    let pos = |i: usize| hand.players[i].position;

    // ---- Preflop -------------------------------------------------------
    let mut raises = 0u32;
    let mut first_raiser: Option<usize> = None;
    let mut last_aggressor: Option<usize> = None;
    let mut anyone_entered = false; // a call or raise happened (not posts)
    let mut steal_raiser: Option<usize> = None;
    let mut entered_after_steal = false;
    let mut acted = vec![false; n];
    let mut folded_pre = vec![false; n];
    let mut seen_3bet_opp = vec![false; n];
    let mut seen_f3b_opp = vec![false; n];
    let mut seen_fts_opp = vec![false; n];

    for a in hand.actions.iter().filter(|a| a.street == Street::Preflop && !a.kind.is_post()) {
        let p = a.player;
        let pc = &mut c[p];
        let first_decision = !acted[p];

        // Steal opportunity: first decision, in CO/BTN/SB, everyone folded before.
        if first_decision && !anyone_entered && pos(p).is_some_and(Position::is_steal_position) {
            pc.opp_steal = 1;
            if a.kind == ActionKind::Raise {
                pc.steal = 1;
                steal_raiser = Some(p);
            }
        }
        // Facing a steal in the blinds.
        if let Some(s) = steal_raiser {
            if s != p && raises == 1 && !entered_after_steal && !seen_fts_opp[p] && matches!(pos(p), Some(Position::Sb | Position::Bb)) {
                seen_fts_opp[p] = true;
                pc.opp_fold_to_steal = 1;
                pc.fold_to_steal = i64::from(a.kind == ActionKind::Fold);
            }
        }
        // Limp: first voluntary action is a call with no raise before.
        if first_decision && raises == 0 && a.kind == ActionKind::Call {
            pc.limp = 1;
        }
        // 3-bet opportunity: facing exactly one raise made by someone else.
        if raises == 1 && first_raiser != Some(p) && !seen_3bet_opp[p] {
            seen_3bet_opp[p] = true;
            pc.opp_3bet = 1;
            pc.three_bet = i64::from(a.kind == ActionKind::Raise);
        }
        // Fold to 3-bet: the opener facing a 3-bet.
        if raises == 2 && first_raiser == Some(p) && !seen_f3b_opp[p] {
            seen_f3b_opp[p] = true;
            pc.opp_fold_to_3bet = 1;
            pc.fold_to_3bet = i64::from(a.kind == ActionKind::Fold);
        }

        match a.kind {
            ActionKind::Call => {
                pc.vpip = 1;
                anyone_entered = true;
                if steal_raiser.is_some() {
                    entered_after_steal = true;
                }
            }
            ActionKind::Bet | ActionKind::Raise => {
                pc.vpip = 1;
                pc.pfr = 1;
                anyone_entered = true;
                raises += 1;
                first_raiser.get_or_insert(p);
                last_aggressor = Some(p);
            }
            ActionKind::Fold => folded_pre[p] = true,
            _ => {}
        }
        if a.is_all_in {
            pc.allin_preflop = 1;
        }
        acted[p] = true;
    }

    // ---- Flop and later ------------------------------------------------
    let flop_dealt = hand.board.len() >= 3;
    for (i, pc) in c.iter_mut().enumerate() {
        if flop_dealt && !folded_pre[i] {
            pc.saw_flop = 1;
        }
    }

    if flop_dealt {
        let mut bet_made = false;
        let mut cbet_done = false;
        let mut raised_after_cbet = false;
        let mut faced = vec![false; n];
        let mut acted_flop = vec![false; n];
        for a in hand.actions.iter().filter(|a| a.street == Street::Flop) {
            let p = a.player;
            if Some(p) == last_aggressor && !acted_flop[p] && !bet_made {
                c[p].opp_cbet_flop = 1;
                if a.kind == ActionKind::Bet {
                    c[p].cbet_flop = 1;
                    cbet_done = true;
                }
            } else if cbet_done && !raised_after_cbet && !faced[p] && Some(p) != last_aggressor {
                faced[p] = true;
                c[p].opp_fold_to_cbet_flop = 1;
                c[p].fold_to_cbet_flop = i64::from(a.kind == ActionKind::Fold);
            }
            if a.kind.is_aggressive() {
                if cbet_done && bet_made {
                    raised_after_cbet = true;
                }
                bet_made = true;
            }
            acted_flop[p] = true;
        }
    }

    for a in hand.actions.iter().filter(|a| a.street != Street::Preflop) {
        let pc = &mut c[a.player];
        match a.kind {
            ActionKind::Bet | ActionKind::Raise => pc.postflop_bets_raises += 1,
            ActionKind::Call => pc.postflop_calls += 1,
            ActionKind::Fold => pc.postflop_folds += 1,
            _ => {}
        }
    }

    for (i, p) in hand.players.iter().enumerate() {
        let won = p.net_won() > 0;
        let pc = &mut c[i];
        if pc.saw_flop == 1 && p.went_to_showdown {
            pc.wtsd = 1;
        }
        if p.went_to_showdown && won {
            pc.wsd = 1;
        }
        if pc.saw_flop == 1 && won {
            pc.wwsf = 1;
        }
        pc.won_hand = i64::from(won);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_matches_counters_toml() {
        #[derive(serde::Deserialize)]
        struct File {
            counter: Vec<Entry>,
        }
        #[derive(serde::Deserialize)]
        struct Entry {
            id: String,
        }
        let text = include_str!("../../../config/defaults/counters.toml");
        let f: File = toml::from_str(text).unwrap();
        let ids: Vec<&str> = f.counter.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, COUNTERS);
        assert_eq!(PlayerCounters::default().values().len(), COUNTERS.len());
    }
}
