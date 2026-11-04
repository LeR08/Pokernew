//! Drills dealt from a solved postflop game.
//!
//! A question is a decision node and a hero combo:
//! * **node**: the first decision of a player (OOP at the root, or IP after a check);
//! * **street**: a decision anywhere on one street, reached by playing both
//!   players' range-wide frequencies from the root;
//! * **hand**: the whole hand, hero plays one side, the other side and the
//!   cards are dealt from the solution, question after question.
//!
//! The hero combo is drawn from hero's range *at that node* (combos weighted
//! by how often they get there), so questions follow the solution.

use std::collections::HashMap;

use lps_evaluator::card_str;
use lps_solver::spot::SCALE;
use lps_solver::view::ActionInfo;
use postflop_solver::{Action, PostFlopGame};
use serde::{Deserialize, Serialize};

use crate::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Node,
    Street,
    Hand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    /// "OOP", "IP" or "card".
    pub who: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub path: Vec<usize>,
    pub history: Vec<Step>,
    pub board: Vec<String>,
    /// 0 = OOP, 1 = IP.
    pub player: usize,
    pub pot: f64,
    /// Hero's hole cards, e.g. "AhKd", and their index in the engine's hand list.
    pub combo: String,
    pub hand: usize,
    pub actions: Vec<ActionInfo>,
    /// Kind of spot, for "my weak spots": "flop · OOP · first" …
    pub node_key: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub chosen: usize,
    /// Solution frequency of each action for this combo.
    pub freqs: Vec<f32>,
    /// EV of each action for this combo, bb.
    pub evs: Vec<f32>,
    pub best: usize,
    pub loss_bb: f64,
    /// Loss in % of the pot at the decision.
    pub loss_pct: f64,
    pub equity: f32,
}

fn info(a: &Action) -> ActionInfo {
    let bb = |x: i32| x as f64 / SCALE;
    match *a {
        Action::Fold => ActionInfo {
            kind: "fold".into(),
            amount: 0.0,
        },
        Action::Check => ActionInfo {
            kind: "check".into(),
            amount: 0.0,
        },
        Action::Call => ActionInfo {
            kind: "call".into(),
            amount: 0.0,
        },
        Action::Bet(x) => ActionInfo {
            kind: "bet".into(),
            amount: bb(x),
        },
        Action::Raise(x) => ActionInfo {
            kind: "raise".into(),
            amount: bb(x),
        },
        Action::AllIn(x) => ActionInfo {
            kind: "allin".into(),
            amount: bb(x),
        },
        _ => ActionInfo {
            kind: "chance".into(),
            amount: 0.0,
        },
    }
}

pub fn label(a: &ActionInfo) -> String {
    if a.amount > 0.0 {
        format!("{} {}", a.kind, (a.amount * 100.0).round() / 100.0)
    } else {
        a.kind.to_string()
    }
}

fn street_name(board_len: usize) -> &'static str {
    match board_len {
        3 => "flop",
        4 => "turn",
        _ => "river",
    }
}

/// Goes to `path` and caches the normalised weights.
fn goto(game: &mut PostFlopGame, path: &[usize]) {
    game.back_to_root();
    game.apply_history(path);
    game.cache_normalized_weights();
}

/// Range-wide frequency of each action of the current player.
fn totals(game: &PostFlopGame) -> Vec<f64> {
    let p = game.current_player();
    let w = game.normalized_weights(p);
    let s = game.strategy();
    let n = w.len();
    let na = s.len().checked_div(n).unwrap_or(0);
    (0..na)
        .map(|a| (0..n).map(|h| (w[h] * s[a * n + h]) as f64).sum())
        .collect()
}

/// Plays one random step from the current node (action by range frequency, or a card).
fn random_step(
    game: &mut PostFlopGame,
    path: &mut Vec<usize>,
    history: &mut Vec<Step>,
    avoid: u64,
    rng: &mut Rng,
) -> bool {
    if game.is_terminal_node() {
        return false;
    }
    if game.is_chance_node() {
        let m = game.possible_cards() & !avoid;
        let cards: Vec<usize> = (0..52).filter(|c| m & (1u64 << c) != 0).collect();
        if cards.is_empty() {
            return false;
        }
        let c = cards[(rng.unit() * cards.len() as f64) as usize % cards.len()];
        history.push(Step {
            who: "card".into(),
            label: card_str(c as u8),
        });
        path.push(c);
        goto(game, path);
        return true;
    }
    let t = totals(game);
    let Some(a) = rng.pick(&t) else { return false };
    let who = if game.current_player() == 0 {
        "OOP"
    } else {
        "IP"
    };
    history.push(Step {
        who: who.into(),
        label: label(&info(&game.available_actions()[a])),
    });
    path.push(a);
    goto(game, path);
    true
}

fn facing(game: &PostFlopGame) -> bool {
    game.available_actions()
        .iter()
        .any(|a| matches!(a, Action::Fold))
}

fn make_question(
    game: &mut PostFlopGame,
    path: Vec<usize>,
    history: Vec<Step>,
    hand: Option<usize>,
    rng: &mut Rng,
) -> Option<Question> {
    goto(game, &path);
    if game.is_terminal_node() || game.is_chance_node() {
        return None;
    }
    let player = game.current_player();
    let w: Vec<f64> = game
        .normalized_weights(player)
        .iter()
        .map(|&x| x as f64)
        .collect();
    let hand = match hand {
        Some(h) => h,
        None => rng.pick(&w)?,
    };
    let (c1, c2) = game.private_cards(player)[hand];
    let board: Vec<String> = game.current_board().iter().map(|&c| card_str(c)).collect();
    let bets = game.total_bet_amount();
    let pot = (game.tree_config().starting_pot + bets[0] + bets[1]) as f64 / SCALE;
    let who = if player == 0 { "OOP" } else { "IP" };
    let node_key = format!(
        "{} · {} · {}",
        street_name(board.len()),
        who,
        if facing(game) { "facing" } else { "first" }
    );
    Some(Question {
        path,
        history,
        board,
        player,
        pot,
        combo: format!("{}{}", card_str(c1.max(c2)), card_str(c1.min(c2))),
        hand,
        actions: game.available_actions().iter().map(info).collect(),
        node_key,
    })
}

/// Deals a question. With `weak` (average loss per node key), several
/// candidates are drawn and the one in the weakest kind of spot is kept.
pub fn deal(
    game: &mut PostFlopGame,
    mode: Mode,
    hero: Option<usize>,
    weak: Option<&HashMap<String, f64>>,
    rng: &mut Rng,
) -> Option<Question> {
    let tries = if weak.is_some() { 6 } else { 1 };
    let mut best: Option<(f64, Question)> = None;
    for _ in 0..tries * 4 {
        let q = deal_one(game, mode, hero, rng);
        if let Some(q) = q {
            let score = weak
                .and_then(|w| w.get(&q.node_key).copied())
                .unwrap_or(0.0)
                + rng.unit() * 0.1;
            if best.as_ref().is_none_or(|(s, _)| score > *s) {
                best = Some((score, q));
            }
            if best.is_some() && weak.is_none() {
                break;
            }
        }
    }
    best.map(|(_, q)| q)
}

fn deal_one(
    game: &mut PostFlopGame,
    mode: Mode,
    hero: Option<usize>,
    rng: &mut Rng,
) -> Option<Question> {
    let mut path = vec![];
    let mut history = vec![];
    goto(game, &path);
    match mode {
        Mode::Node | Mode::Hand => {
            // OOP's first decision, or IP's after a check (hero side if chosen)
            let want = hero.unwrap_or(if rng.unit() < 0.5 { 0 } else { 1 });
            if want == 1 {
                let check = game
                    .available_actions()
                    .iter()
                    .position(|a| matches!(a, Action::Check))?;
                history.push(Step {
                    who: "OOP".into(),
                    label: "check".into(),
                });
                path.push(check);
            }
            make_question(game, path, history, None, rng)
        }
        Mode::Street => {
            // walk to a random decision; stop at each decision with probability 1/3
            for _ in 0..40 {
                goto(game, &path);
                if game.is_terminal_node() {
                    return None;
                }
                if !game.is_chance_node()
                    && hero.is_none_or(|h| h == game.current_player())
                    && rng.unit() < 0.34
                {
                    return make_question(game, path, history, None, rng);
                }
                if !random_step(game, &mut path, &mut history, 0, rng) {
                    return None;
                }
            }
            None
        }
    }
}

/// Grades hero's choice at the question's node.
pub fn answer(game: &mut PostFlopGame, q: &Question, chosen: usize) -> Option<Answer> {
    goto(game, &q.path);
    if game.is_terminal_node() || game.is_chance_node() || game.current_player() != q.player {
        return None;
    }
    let n = game.private_cards(q.player).len();
    let na = game.available_actions().len();
    if chosen >= na || q.hand >= n {
        return None;
    }
    let s = game.strategy();
    let ev = game.expected_values_detail(q.player);
    let eq = game.equity(q.player);
    let freqs: Vec<f32> = (0..na).map(|a| s[a * n + q.hand]).collect();
    let evs: Vec<f32> = (0..na).map(|a| ev[a * n + q.hand] / SCALE as f32).collect();
    let best = (0..na)
        .max_by(|&a, &b| evs[a].total_cmp(&evs[b]))
        .unwrap_or(0);
    let loss_bb = (evs[best] - evs[chosen]).max(0.0) as f64;
    Some(Answer {
        chosen,
        freqs,
        evs,
        best,
        loss_bb,
        loss_pct: if q.pot > 0.0 {
            loss_bb / q.pot * 100.0
        } else {
            0.0
        },
        equity: eq[q.hand],
    })
}

/// Full-hand mode: plays hero's choice, then the other side and the cards
/// from the solution, up to hero's next decision (None: the hand is over).
pub fn next(
    game: &mut PostFlopGame,
    q: &Question,
    chosen: usize,
    rng: &mut Rng,
) -> Option<Question> {
    goto(game, &q.path);
    let mut path = q.path.clone();
    let mut history = q.history.clone();
    let who = if q.player == 0 { "OOP" } else { "IP" };
    history.push(Step {
        who: who.into(),
        label: label(&q.actions[chosen]),
    });
    path.push(chosen);
    goto(game, &path);
    let (c1, c2) = game.private_cards(q.player)[q.hand];
    let avoid = (1u64 << c1) | (1u64 << c2);
    for _ in 0..40 {
        if game.is_terminal_node() {
            return None;
        }
        if !game.is_chance_node() && game.current_player() == q.player {
            return make_question(game, path, history, Some(q.hand), rng);
        }
        if !random_step(game, &mut path, &mut history, avoid, rng) {
            return None;
        }
    }
    None
}
