//! A played hand against the solver.
//!
//! Heads-up pots that see a flop only. The spot is rebuilt from the hand:
//! flop, pot and effective stack at the flop, who is in position. The ranges
//! come from the user's chart library (by default the charts of a chosen
//! format): the preflop raiser's open (or 3-bet) range and the caller's call
//! range. The spot is solved (or found in the cache), then hero's real line is
//! followed in the tree: real bet sizes are mapped to the nearest size of the
//! tree, the real turn and river are dealt. At each of hero's decisions the EV
//! of every action for hero's real cards is read; the loss is the best EV minus
//! the EV of the action played.

use lps_core_model::{ActionKind, Hand, Position, Street};
use lps_solver::spot::SCALE;
use lps_solver::{SpotConfig, Tree};
use postflop_solver::{Action, PostFlopGame};
use serde::{Deserialize, Serialize};

use crate::drill::label;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Decision {
    pub street: String,
    /// e.g. "turn · PFR · OOP · first"
    pub node_key: String,
    pub played: String,
    pub best: String,
    pub loss_bb: f64,
    pub loss_pct: f64,
    /// Real action far from every tree size (> 30 % of the pot away).
    pub approximate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandAnalysis {
    pub spot: SpotConfig,
    pub hero_cards: String,
    /// Ranges used (library names).
    pub hero_range: String,
    pub villain_range: String,
    pub decisions: Vec<Decision>,
    pub total_loss_bb: f64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AnalysisError {
    #[error("not a heads-up pot seeing the flop with hero")]
    NotHeadsUp,
    #[error("hero's cards are unknown")]
    NoCards,
    #[error("preflop line not covered (limped or 4-bet pot)")]
    Line,
    #[error("no chart '{0}' in the range library")]
    Chart(String),
    #[error("hero's hand is not in the range '{0}'")]
    NotInRange(String),
    #[error("{0}")]
    Solve(String),
}

/// Chart position name (6-max charts) of a table position.
pub fn chart_position(p: Position) -> &'static str {
    match p {
        Position::Utg | Position::Utg1 | Position::Utg2 | Position::Mp => "UTG",
        Position::Lj | Position::Hj => "HJ",
        Position::Co => "CO",
        Position::Btn => "BTN",
        Position::Sb => "SB",
        Position::Bb => "BB",
    }
}

/// What the analysis needs from the hand, before solving.
pub struct Prepared {
    pub spot: SpotConfig,
    pub hero: usize,
    /// Engine player of hero: 0 = OOP, 1 = IP.
    pub hero_side: usize,
    pub hero_cards: (u8, u8),
    pub hero_range: String,
    pub villain_range: String,
    pub hero_is_pfr: bool,
}

/// Rebuilds the flop spot. `charts(name)` returns a library range's text.
pub fn prepare(
    hand: &Hand,
    format: &str,
    tree: &Tree,
    charts: &dyn Fn(&str) -> Option<String>,
) -> Result<Prepared, AnalysisError> {
    let hero = hand
        .players
        .iter()
        .position(|p| p.is_hero)
        .ok_or(AnalysisError::NoCards)?;
    if hand.players[hero].hole_cards.len() != 2 || hand.board.len() < 3 {
        return Err(if hand.board.len() < 3 {
            AnalysisError::NotHeadsUp
        } else {
            AnalysisError::NoCards
        });
    }
    let pre: Vec<_> = hand
        .actions
        .iter()
        .filter(|a| a.street == Street::Preflop && !a.kind.is_post())
        .collect();
    let folded: std::collections::HashSet<usize> = pre
        .iter()
        .filter(|a| a.kind == ActionKind::Fold)
        .map(|a| a.player)
        .collect();
    let flop: Vec<usize> = (0..hand.players.len())
        .filter(|i| !folded.contains(i))
        .collect();
    if flop.len() != 2 || !flop.contains(&hero) {
        return Err(AnalysisError::NotHeadsUp);
    }
    let villain = if flop[0] == hero { flop[1] } else { flop[0] };
    let pos = |i: usize| {
        hand.players[i]
            .position
            .map(chart_position)
            .ok_or(AnalysisError::Line)
    };
    let (hp, vp) = (pos(hero)?, pos(villain)?);
    // preflop raises: who opened, who 3-bet
    let raises: Vec<usize> = pre
        .iter()
        .filter(|a| a.kind == ActionKind::Raise || (a.kind == ActionKind::Bet))
        .map(|a| a.player)
        .collect();
    let (hero_range, villain_range, hero_is_pfr) = match raises.as_slice() {
        [opener] => {
            let caller = if *opener == hero { villain } else { hero };
            let (o, c) = (pos(*opener)?, pos(caller)?);
            let open = format!("{format}/{o}/open");
            let call = format!("{format}/{c} vs {o}/call");
            if *opener == hero {
                (open, call, true)
            } else {
                (call, open, false)
            }
        }
        [opener, three] if *three != *opener => {
            let (o, t) = (pos(*opener)?, pos(*three)?);
            let three_r = format!("{format}/{t} vs {o}/3bet");
            let call = format!("{format}/{o} vs 3bet/call");
            if *three == hero {
                (three_r, call, true)
            } else {
                (call, three_r, false)
            }
        }
        _ => return Err(AnalysisError::Line),
    };
    let load = |n: &str| charts(n).ok_or_else(|| AnalysisError::Chart(n.to_string()));
    let (hr, vr) = (load(&hero_range)?, load(&villain_range)?);
    // postflop order: SB, BB, then the others in preflop order (heads-up the SB / button acts last)
    let order_rank = |p: &str| match p {
        "SB" if hand.players.len() == 2 => 9,
        "SB" => 0,
        "BB" => 1,
        "UTG" => 2,
        "HJ" => 3,
        "CO" => 4,
        _ => 5,
    };
    let hero_side = usize::from(order_rank(hp) > order_rank(vp));
    let bb = hand.big_blind as f64;
    let invested = |i: usize| -> i64 {
        hand.actions
            .iter()
            .filter(|a| a.player == i && a.street == Street::Preflop)
            .map(|a| a.amount)
            .sum()
    };
    let pot = hand
        .actions
        .iter()
        .filter(|a| a.street == Street::Preflop)
        .map(|a| a.amount)
        .sum::<i64>() as f64
        / bb;
    let stack = ((hand.players[hero].stack - invested(hero))
        .min(hand.players[villain].stack - invested(villain))) as f64
        / bb;
    let board: String = hand.board[..3].iter().map(|c| format!("{c}")).collect();
    let (oop, ip) = if hero_side == 0 {
        (hr.clone(), vr.clone())
    } else {
        (vr.clone(), hr.clone())
    };
    let spot = SpotConfig {
        oop_range: oop.trim().to_string(),
        ip_range: ip.trim().to_string(),
        board,
        pot: (pot * 100.0).round() / 100.0,
        stack: (stack * 100.0).round() / 100.0,
        tree: tree.clone(),
        ..SpotConfig::default()
    };
    if spot.stack.is_nan() || spot.stack <= 0.0 {
        return Err(AnalysisError::NotHeadsUp);
    }
    let hc = &hand.players[hero].hole_cards;
    Ok(Prepared {
        spot,
        hero,
        hero_side,
        hero_cards: (hc[0].index(), hc[1].index()),
        hero_range,
        villain_range,
        hero_is_pfr,
    })
}

/// The tree action closest to a real one (kind first, then size in chips).
fn map_action(
    actions: &[Action],
    kind: ActionKind,
    to: f64,
    all_in: bool,
    pot: f64,
) -> Option<(usize, bool)> {
    let pick = |want: &dyn Fn(&Action) -> bool| actions.iter().position(want);
    match kind {
        ActionKind::Fold => pick(&|a| matches!(a, Action::Fold)).map(|i| (i, false)),
        ActionKind::Check => pick(&|a| matches!(a, Action::Check)).map(|i| (i, false)),
        ActionKind::Call => pick(&|a| matches!(a, Action::Call)).map(|i| (i, false)),
        ActionKind::Bet | ActionKind::Raise => {
            let cands: Vec<(usize, f64)> = actions
                .iter()
                .enumerate()
                .filter_map(|(i, a)| match *a {
                    Action::Bet(x) | Action::Raise(x) | Action::AllIn(x) => Some((i, x as f64)),
                    _ => None,
                })
                .collect();
            if all_in {
                if let Some(i) = actions.iter().position(|a| matches!(a, Action::AllIn(_))) {
                    return Some((i, false));
                }
            }
            let (i, x) = cands
                .into_iter()
                .min_by(|a, b| (a.1 - to).abs().total_cmp(&(b.1 - to).abs()))?;
            Some((i, (x - to).abs() > 0.3 * pot))
        }
        _ => None,
    }
}

/// Follows the real line in a solved game and grades hero's decisions.
pub fn grade_line(
    game: &mut PostFlopGame,
    hand: &Hand,
    prep: &Prepared,
) -> Result<HandAnalysis, AnalysisError> {
    let side = prep.hero_side;
    let cards = game.private_cards(side).to_vec();
    let (a, b) = prep.hero_cards;
    let hero_hand = cards
        .iter()
        .position(|&(x, y)| (x == a && y == b) || (x == b && y == a))
        .ok_or_else(|| AnalysisError::NotInRange(prep.hero_range.clone()))?;
    let bb = hand.big_blind as f64;
    let mut path: Vec<usize> = vec![];
    let mut decisions = vec![];
    let mut street = Street::Flop;
    let mut street_bets = [0.0f64; 2]; // chips put in on this street by OOP / IP (engine scale)
    game.back_to_root();
    for act in hand.actions.iter().filter(|x| x.street != Street::Preflop) {
        // deal the next street's card in the tree when the street changes
        while act.street != street {
            street = match street {
                Street::Flop => Street::Turn,
                _ => Street::River,
            };
            if game.is_chance_node() {
                let card = hand
                    .board
                    .get(if street == Street::Turn { 3 } else { 4 })
                    .ok_or(AnalysisError::NotHeadsUp)?
                    .index() as usize;
                path.push(card);
                game.play(card);
            }
            street_bets = [0.0; 2];
        }
        if game.is_terminal_node() || game.is_chance_node() {
            break;
        }
        let actor_side = if act.player == prep.hero {
            side
        } else {
            1 - side
        };
        if game.current_player() != actor_side {
            break; // the tree and the hand disagree (should not happen)
        }
        let actions = game.available_actions();
        let pot_now = (game.tree_config().starting_pot
            + game.total_bet_amount()[0]
            + game.total_bet_amount()[1]) as f64;
        let to = match act.kind {
            ActionKind::Raise => act.raise_to.unwrap_or(act.amount) as f64 / bb * SCALE,
            _ => street_bets[actor_side] + act.amount as f64 / bb * SCALE,
        };
        let Some((idx, approx)) = map_action(&actions, act.kind, to, act.is_all_in, pot_now) else {
            break;
        };
        if act.player == prep.hero {
            game.cache_normalized_weights();
            let n = cards.len();
            let ev = game.expected_values_detail(side);
            let na = actions.len();
            let evs: Vec<f64> = (0..na)
                .map(|k| ev[k * n + hero_hand] as f64 / SCALE)
                .collect();
            let best = (0..na)
                .max_by(|&x, &y| evs[x].total_cmp(&evs[y]))
                .unwrap_or(0);
            let loss = (evs[best] - evs[idx]).max(0.0);
            let pot_bb = pot_now / SCALE;
            let infos: Vec<_> = actions
                .iter()
                .map(|x| lps_solver::view::ActionInfo {
                    kind: kind_name(x).into(),
                    amount: amount(x),
                })
                .collect();
            let facing = actions.iter().any(|x| matches!(x, Action::Fold));
            decisions.push(Decision {
                street: street_name(street).into(),
                node_key: format!(
                    "{} · {} · {} · {}",
                    street_name(street),
                    if prep.hero_is_pfr { "PFR" } else { "caller" },
                    if side == 0 { "OOP" } else { "IP" },
                    if facing { "facing" } else { "first" }
                ),
                played: label(&infos[idx]),
                best: label(&infos[best]),
                loss_bb: loss,
                loss_pct: if pot_bb > 0.0 {
                    loss / pot_bb * 100.0
                } else {
                    0.0
                },
                approximate: approx,
            });
        }
        street_bets[actor_side] = match actions[idx] {
            Action::Bet(x) | Action::Raise(x) | Action::AllIn(x) => x as f64,
            Action::Call => street_bets[1 - actor_side],
            _ => street_bets[actor_side],
        };
        path.push(idx);
        game.play(idx);
    }
    let total = decisions.iter().map(|d| d.loss_bb).sum();
    let hc = &hand.players[prep.hero].hole_cards;
    Ok(HandAnalysis {
        spot: prep.spot.clone(),
        hero_cards: hc.iter().map(|c| format!("{c}")).collect(),
        hero_range: prep.hero_range.clone(),
        villain_range: prep.villain_range.clone(),
        decisions,
        total_loss_bb: total,
    })
}

fn kind_name(a: &Action) -> &'static str {
    match a {
        Action::Fold => "fold",
        Action::Check => "check",
        Action::Call => "call",
        Action::Bet(_) => "bet",
        Action::Raise(_) => "raise",
        Action::AllIn(_) => "allin",
        _ => "chance",
    }
}

fn amount(a: &Action) -> f64 {
    match *a {
        Action::Bet(x) | Action::Raise(x) | Action::AllIn(x) => x as f64 / SCALE,
        _ => 0.0,
    }
}

fn street_name(s: Street) -> &'static str {
    match s {
        Street::Preflop => "preflop",
        Street::Flop => "flop",
        Street::Turn => "turn",
        Street::River => "river",
    }
}
