//! Sessions and the post-session review.
//!
//! **Sessions** are built on the fly from the hero's hands: a gap of more than
//! [`SESSION_GAP`] between two hands starts a new session.
//!
//! The **review** applies plain poker rules to each finished hand and flags
//! spots *to look at again* (`review`) and *good plays* (`good`). These are
//! rules of thumb, not solver output: each finding names its rule and why it
//! matters, and the user decides. Only completed hands are ever reviewed
//! (study tool, see the Fair-play section of the spec).
//!
//! | rule | kind | when |
//! |---|---|---|
//! | `open_limp` | review | first to enter the pot, by calling (not from the blinds) |
//! | `weak_shove` | review | first raise preflop is all-in with > 15 bb and a Chen score < 6 |
//! | `raise_fold_short` | review | ≤ 12 bb: raised preflop without going all-in, then folded |
//! | `premium_fold` | review | folded AA/KK/QQ/AK preflop facing at most one raise |
//! | `sb_flat` | review | small blind just calls an open raise (4+ players) |
//! | `river_call_lost` | review | called a river bet of at least half the pot and lost at showdown |
//! | `big_fold` | review | folded after putting in ≥ 15 bb and ≥ 30 % of the stack |
//! | `steal_won` | good | steal raise from CO/BTN/SB took the blinds |
//! | `three_bet_won` | good | 3-bet preflop and won the hand |
//! | `cbet_won` | good | flop c-bet won the pot without showdown |
//! | `value_bet` | good | bet or raised the river and won at showdown |
//! | `short_shove` | good | ≤ 10 bb, first in, all-in with a Chen score ≥ 7 |
//! | `big_pot` | good | won at least 20 bb |

use lps_core_model::preflop::{chen, hand_class, is_premium};
use lps_core_model::{ActionKind, Hand, Position, Street};
use rusqlite::{params_from_iter, Connection};
use serde::Serialize;

use crate::query::Filter;

/// Gap between two hands that starts a new session.
pub const SESSION_GAP: i64 = 30 * 60;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub start: i64,
    pub end: i64,
    pub hands: i64,
    pub net_bb: f64,
    pub net_cash_cents: i64,
    pub tournaments: i64,
    pub vpip_hands: i64,
    pub pfr_hands: i64,
}

/// Hero sessions matching `filter`, newest first.
pub fn sessions(conn: &Connection, filter: &Filter) -> rusqlite::Result<Vec<Session>> {
    let w = filter.to_sql();
    let sql = format!(
        "SELECT h.played_at, hp.net_won_bb, CASE WHEN h.unit = 'cents' THEN hp.net_won ELSE 0 END, h.tournament_id,
                s.cnt_vpip, s.cnt_pfr
         FROM hand_player_stats s
         JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
         JOIN hands h ON h.id = s.hand_id JOIN players p ON p.id = hp.player_id
         WHERE {} ORDER BY h.played_at, h.id",
        w.text
    );
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(params_from_iter(w.params), |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, i64>(5)?,
        ))
    })?;
    let mut out: Vec<Session> = Vec::new();
    let mut tourneys: Vec<i64> = Vec::new();
    for row in rows {
        let (t, bb, cents, tid, vpip, pfr) = row?;
        let new = out.last().is_none_or(|s| t - s.end > SESSION_GAP);
        if new {
            tourneys.clear();
            out.push(Session {
                start: t,
                end: t,
                hands: 0,
                net_bb: 0.0,
                net_cash_cents: 0,
                tournaments: 0,
                vpip_hands: 0,
                pfr_hands: 0,
            });
        }
        let s = out.last_mut().expect("just pushed");
        s.end = t;
        s.hands += 1;
        s.net_bb += bb;
        s.net_cash_cents += cents;
        s.vpip_hands += vpip;
        s.pfr_hands += pfr;
        if let Some(id) = tid {
            if !tourneys.contains(&id) {
                tourneys.push(id);
                s.tournaments += 1;
            }
        }
    }
    out.reverse();
    Ok(out)
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Review,
    Good,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub rule: &'static str,
    pub kind: Kind,
    pub hand_id: i64,
    pub played_at: i64,
    pub hole_cards: String,
    pub position: Option<String>,
    pub stack_bb: f64,
    pub net_bb: f64,
    /// Values for the explanation text (bb amounts, hand class…).
    pub detail: serde_json::Map<String, serde_json::Value>,
}

/// Preflop facts about the hero, computed once per hand.
struct Ctx {
    bb: f64,
    stack_bb: f64,
    class: Option<String>,
    chen: Option<i32>,
}

impl Ctx {
    fn bb(&self, chips: i64) -> f64 {
        (chips as f64 / self.bb * 10.0).round() / 10.0
    }
}

/// Applies every rule to one hand.
pub fn review_hand(hand: &Hand, hand_id: i64) -> Vec<Finding> {
    let Some(hero) = hand.players.iter().position(|p| p.is_hero) else {
        return vec![];
    };
    let p = &hand.players[hero];
    let bb = hand.big_blind.max(1) as f64;
    let cx = Ctx {
        bb,
        stack_bb: p.stack as f64 / bb,
        class: hand_class(&p.hole_cards),
        chen: chen(&p.hole_cards),
    };
    let net_bb = p.net_won() as f64 / bb;
    let mut found: Vec<(
        &'static str,
        Kind,
        serde_json::Map<String, serde_json::Value>,
    )> = Vec::new();
    macro_rules! add {
        ($rule:expr, $kind:expr, $detail:expr) => {
            found.push((
                $rule,
                $kind,
                $detail.as_object().cloned().unwrap_or_default(),
            ))
        };
    }
    let class = cx.class.clone().unwrap_or_default();

    // ---- Preflop walk ----
    let pre: Vec<_> = hand
        .actions
        .iter()
        .filter(|a| a.street == Street::Preflop && !a.kind.is_post())
        .collect();
    let mut raises = 0;
    let mut entered = false; // someone called or raised before
    let mut hero_first = true;
    let mut hero_raised_not_allin = false;
    let mut hero_folded_pre = false;
    let mut hero_3bet = false;
    let mut hero_steal = false;
    let mut hero_open_shove = false;
    let mut flat_call_sb = false;
    for a in &pre {
        if a.player == hero {
            if hero_first {
                let pos = p.position;
                if !entered
                    && a.kind == ActionKind::Call
                    && !matches!(pos, Some(Position::Sb | Position::Bb))
                {
                    add!("open_limp", Kind::Review, serde_json::json!({}));
                }
                if !entered
                    && a.kind == ActionKind::Raise
                    && pos.is_some_and(Position::is_steal_position)
                {
                    hero_steal = true;
                }
                if raises == 0 && a.kind == ActionKind::Raise && a.is_all_in {
                    hero_open_shove = true;
                }
                if pos == Some(Position::Sb)
                    && raises == 1
                    && a.kind == ActionKind::Call
                    && hand.players.len() >= 4
                {
                    flat_call_sb = true;
                }
            }
            if raises == 1 && a.kind == ActionKind::Raise {
                hero_3bet = true;
            }
            if a.kind == ActionKind::Raise && !a.is_all_in {
                hero_raised_not_allin = true;
            }
            if a.kind == ActionKind::Fold {
                hero_folded_pre = true;
                if is_premium(&p.hole_cards)
                    && (raises <= 1 || matches!(class.as_str(), "AA" | "KK"))
                {
                    add!(
                        "premium_fold",
                        Kind::Review,
                        serde_json::json!({ "hand": class, "raises": raises })
                    );
                }
            }
            hero_first = false;
        }
        if matches!(
            a.kind,
            ActionKind::Call | ActionKind::Raise | ActionKind::Bet
        ) {
            entered = true;
        }
        if a.kind == ActionKind::Raise {
            raises += 1;
        }
    }
    if flat_call_sb {
        add!(
            "sb_flat",
            Kind::Review,
            serde_json::json!({ "hand": class })
        );
    }
    if hero_open_shove && cx.stack_bb > 15.0 && cx.chen.is_some_and(|c| c < 6) {
        add!(
            "weak_shove",
            Kind::Review,
            serde_json::json!({ "hand": class, "stack": cx.bb(p.stack) })
        );
    }
    if hero_open_shove && cx.stack_bb <= 10.0 && cx.chen.is_some_and(|c| c >= 7) {
        add!(
            "short_shove",
            Kind::Good,
            serde_json::json!({ "hand": class, "stack": cx.bb(p.stack) })
        );
    }
    if cx.stack_bb <= 12.0 && hero_raised_not_allin && hero_folded_pre {
        add!(
            "raise_fold_short",
            Kind::Review,
            serde_json::json!({ "stack": cx.bb(p.stack), "lost": cx.bb(p.invested) })
        );
    }

    // ---- Later streets ----
    let won_without_showdown = p.net_won() > 0 && !p.went_to_showdown;
    if hero_steal && won_without_showdown && hand.board.is_empty() {
        add!(
            "steal_won",
            Kind::Good,
            serde_json::json!({ "won": cx.bb(p.net_won()) })
        );
    }
    if hero_3bet && p.net_won() > 0 {
        add!(
            "three_bet_won",
            Kind::Good,
            serde_json::json!({ "hand": class, "won": cx.bb(p.net_won()) })
        );
    }
    // C-bet: hero was the last preflop raiser and bet first on the flop.
    let last_raiser = pre
        .iter()
        .rev()
        .find(|a| a.kind == ActionKind::Raise)
        .map(|a| a.player);
    let first_flop_bet = hand
        .actions
        .iter()
        .find(|a| a.street == Street::Flop && a.kind.is_aggressive());
    if last_raiser == Some(hero)
        && first_flop_bet.is_some_and(|a| a.player == hero)
        && won_without_showdown
    {
        add!(
            "cbet_won",
            Kind::Good,
            serde_json::json!({ "won": cx.bb(p.net_won()) })
        );
    }
    let river: Vec<_> = hand
        .actions
        .iter()
        .filter(|a| a.street == Street::River)
        .collect();
    if p.went_to_showdown
        && p.net_won() > 0
        && river
            .iter()
            .any(|a| a.player == hero && a.kind.is_aggressive())
    {
        add!(
            "value_bet",
            Kind::Good,
            serde_json::json!({ "won": cx.bb(p.net_won()) })
        );
    }
    if p.went_to_showdown && p.net_won() < 0 {
        if let Some(call) = river
            .iter()
            .find(|a| a.player == hero && a.kind == ActionKind::Call)
        {
            // The bet being called is at least half of the pot before it.
            let bet = river.iter().rev().find(|b| {
                b.kind.is_aggressive() && b.player != hero && b.pot_before <= call.pot_before
            });
            if let Some(bet) = bet {
                if bet.amount * 2 >= bet.pot_before {
                    add!(
                        "river_call_lost",
                        Kind::Review,
                        serde_json::json!({ "call": cx.bb(call.amount), "lost": cx.bb(-p.net_won()) })
                    );
                }
            }
        }
    }
    let folded = hand
        .actions
        .iter()
        .any(|a| a.player == hero && a.kind == ActionKind::Fold);
    if folded
        && !hero_folded_pre_premium_only(&found)
        && p.invested as f64 >= 15.0 * bb
        && p.invested * 10 >= p.stack * 3
    {
        add!(
            "big_fold",
            Kind::Review,
            serde_json::json!({ "lost": cx.bb(p.invested) })
        );
    }
    if net_bb >= 20.0 {
        add!(
            "big_pot",
            Kind::Good,
            serde_json::json!({ "won": cx.bb(p.net_won()) })
        );
    }

    found
        .into_iter()
        .map(|(rule, kind, detail)| Finding {
            rule,
            kind,
            hand_id,
            played_at: hand.played_at,
            hole_cards: p.hole_cards.iter().map(ToString::to_string).collect(),
            position: p.position.map(|x| x.as_str().to_string()),
            stack_bb: (cx.stack_bb * 10.0).round() / 10.0,
            net_bb: (net_bb * 10.0).round() / 10.0,
            detail,
        })
        .collect()
}

/// `big_fold` would repeat `premium_fold`; keep the more specific one.
fn hero_folded_pre_premium_only(
    found: &[(
        &'static str,
        Kind,
        serde_json::Map<String, serde_json::Value>,
    )],
) -> bool {
    found.iter().any(|(r, _, _)| *r == "premium_fold")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReview {
    pub session: Option<Session>,
    pub findings: Vec<Finding>,
    /// VPIP / PFR of the session and over all the hero's hands (same filter, no dates).
    pub vpip: Option<f64>,
    pub pfr: Option<f64>,
    pub vpip_overall: Option<f64>,
    pub pfr_overall: Option<f64>,
    /// Session VPIP ≥ 10 points above usual over ≥ 50 hands: possible tilt.
    pub tilt_warning: bool,
    /// Biggest single-hand loss and gain (hand ids).
    pub worst_hand: Option<(i64, f64)>,
    pub best_hand: Option<(i64, f64)>,
}

/// Reviews the hero's hands between `start` and `end` (inclusive, Unix seconds).
pub fn review_session(
    conn: &Connection,
    filter: &Filter,
    start: i64,
    end: i64,
) -> Result<SessionReview, lps_db::DbError> {
    let mut f = filter.clone();
    f.from = Some(start);
    f.to = Some(end + 1);
    let session = sessions(conn, &f)?.into_iter().next();
    let w = f.to_sql();
    let sql = format!(
        "SELECT h.id, hp.net_won_bb FROM hand_player_stats s
         JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
         JOIN hands h ON h.id = hp.hand_id JOIN players p ON p.id = hp.player_id
         WHERE {} ORDER BY h.played_at, h.id",
        w.text
    );
    let ids: Vec<(i64, f64)> = {
        let mut st = conn.prepare(&sql)?;
        let rows = st.query_map(params_from_iter(w.params), |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut findings = Vec::new();
    for (id, _) in &ids {
        let (hand, _) = lps_db::load::load_hand(conn, *id)?;
        findings.extend(review_hand(&hand, *id));
    }
    // Biggest losses first among reviews, biggest wins first among good plays.
    findings.sort_by(|a, b| match (a.kind, b.kind) {
        (Kind::Review, Kind::Good) => std::cmp::Ordering::Less,
        (Kind::Good, Kind::Review) => std::cmp::Ordering::Greater,
        (Kind::Review, Kind::Review) => a.net_bb.total_cmp(&b.net_bb),
        (Kind::Good, Kind::Good) => b.net_bb.total_cmp(&a.net_bb),
    });

    let ratio = |a: i64, b: i64| (b > 0).then(|| a as f64 / b as f64);
    let (vpip, pfr, hands) = session
        .as_ref()
        .map(|s| {
            (
                ratio(s.vpip_hands, s.hands),
                ratio(s.pfr_hands, s.hands),
                s.hands,
            )
        })
        .unwrap_or((None, None, 0));
    let mut all = filter.clone();
    all.from = None;
    all.to = None;
    let wa = all.to_sql();
    let (v_all, p_all, n_all): (i64, i64, i64) = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(s.cnt_vpip), 0), COALESCE(SUM(s.cnt_pfr), 0), COUNT(*)
             FROM hand_player_stats s JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
             JOIN hands h ON h.id = s.hand_id JOIN players p ON p.id = hp.player_id WHERE {}",
            wa.text
        ),
        params_from_iter(wa.params.to_vec()),
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let vpip_overall = ratio(v_all, n_all);
    let tilt_warning =
        hands >= 50 && matches!((vpip, vpip_overall), (Some(s), Some(o)) if s >= o + 0.10);
    let worst_hand = ids
        .iter()
        .copied()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|x| x.1 < 0.0);
    let best_hand = ids
        .iter()
        .copied()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .filter(|x| x.1 > 0.0);
    Ok(SessionReview {
        session,
        findings,
        vpip,
        pfr,
        vpip_overall,
        pfr_overall: ratio(p_all, n_all),
        tilt_warning,
        worst_hand,
        best_hand,
    })
}
