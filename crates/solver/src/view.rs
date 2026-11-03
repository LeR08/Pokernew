//! Reading a solved game at any node: actions, strategy, EV, equity, grid.

use lps_equity::combo::{Combo, NUM_CLASSES};
use lps_evaluator::card_str;
use postflop_solver::{Action, PostFlopGame};
use serde::Serialize;

use crate::spot::SCALE;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionInfo {
    /// "fold", "check", "call", "bet", "raise", "allin"
    pub kind: &'static str,
    /// Amount in bb (bets and raises: the total put in on this street).
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandInfo {
    /// Combo index (Equity Lab numbering).
    pub combo: u16,
    pub cards: String,
    /// Share of this combo still in the range at this node (0–1, relative to the start).
    pub weight: f32,
    /// Frequency of each action (actor only).
    pub strategy: Vec<f32>,
    /// Expected value in bb and equity (0–1).
    pub ev: f32,
    pub equity: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassCell {
    /// Grid cell (Equity Lab numbering).
    pub cell: usize,
    /// Sum of combo weights.
    pub weight: f32,
    /// Weighted frequency of each action.
    pub freq: Vec<f32>,
    pub ev: f32,
    pub equity: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub hands: Vec<HandInfo>,
    pub grid: Vec<ClassCell>,
    /// Range-weighted averages.
    pub ev: f32,
    pub equity: f32,
    /// EV ÷ (equity × pot): how much of its equity the range realises.
    pub eqr: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeView {
    pub path: Vec<usize>,
    pub board: Vec<String>,
    /// "action", "chance" or "terminal".
    pub kind: &'static str,
    /// 0 = OOP, 1 = IP (action nodes).
    pub player: Option<usize>,
    pub actions: Vec<ActionInfo>,
    /// Overall frequency of each action (range-weighted).
    pub totals: Vec<f32>,
    /// Cards that can come (chance nodes).
    pub cards: Vec<String>,
    pub pot: f64,
    /// Chips already put in this hand by OOP and IP since the start of the spot, bb.
    pub bets: [f64; 2],
    pub locked: bool,
    /// [OOP, IP] (only at action nodes of a solved game).
    pub players: Vec<PlayerView>,
}

fn action_info(a: &Action) -> ActionInfo {
    let bb = |x: i32| x as f64 / SCALE;
    match *a {
        Action::Fold => ActionInfo {
            kind: "fold",
            amount: 0.0,
        },
        Action::Check => ActionInfo {
            kind: "check",
            amount: 0.0,
        },
        Action::Call => ActionInfo {
            kind: "call",
            amount: 0.0,
        },
        Action::Bet(x) => ActionInfo {
            kind: "bet",
            amount: bb(x),
        },
        Action::Raise(x) => ActionInfo {
            kind: "raise",
            amount: bb(x),
        },
        Action::AllIn(x) => ActionInfo {
            kind: "allin",
            amount: bb(x),
        },
        _ => ActionInfo {
            kind: "chance",
            amount: 0.0,
        },
    }
}

fn player_view(game: &PostFlopGame, player: usize, actor: Option<usize>, pot: f64) -> PlayerView {
    let cards = game.private_cards(player);
    let weights = game.normalized_weights(player);
    let raw = game.weights(player);
    let ev = game.expected_values(player);
    let eq = game.equity(player);
    let strat = if actor == Some(player) {
        game.strategy()
    } else {
        vec![]
    };
    let n = cards.len();
    let na = strat.len().checked_div(n).unwrap_or(0);
    let mut grid: Vec<ClassCell> = (0..NUM_CLASSES)
        .map(|cell| ClassCell {
            cell,
            weight: 0.0,
            freq: vec![0.0; na],
            ev: 0.0,
            equity: 0.0,
        })
        .collect();
    let mut hands = Vec::with_capacity(n);
    let (mut wsum, mut evs, mut eqs) = (0.0f64, 0.0f64, 0.0f64);
    for (i, &(c1, c2)) in cards.iter().enumerate() {
        let combo = Combo::new(c1, c2);
        let w = weights[i];
        let s: Vec<f32> = (0..na).map(|a| strat[a * n + i]).collect();
        let hev = ev[i] / SCALE as f32;
        let g = &mut grid[combo.class()];
        g.weight += w;
        for (a, f) in s.iter().enumerate() {
            g.freq[a] += w * f;
        }
        g.ev += w * hev;
        g.equity += w * eq[i];
        wsum += w as f64;
        evs += (w * hev) as f64;
        eqs += (w * eq[i]) as f64;
        hands.push(HandInfo {
            combo: combo.index() as u16,
            cards: format!("{}{}", card_str(combo.hi), card_str(combo.lo)),
            weight: raw[i],
            strategy: s,
            ev: hev,
            equity: eq[i],
        });
    }
    for g in grid.iter_mut().filter(|g| g.weight > 0.0) {
        let w = g.weight;
        g.freq.iter_mut().for_each(|f| *f /= w);
        g.ev /= w;
        g.equity /= w;
    }
    let (ev_avg, eq_avg) = if wsum > 0.0 {
        (evs / wsum, eqs / wsum)
    } else {
        (0.0, 0.0)
    };
    PlayerView {
        hands,
        grid,
        ev: ev_avg as f32,
        equity: eq_avg as f32,
        eqr: if eq_avg > 0.0 && pot > 0.0 {
            (ev_avg / (eq_avg * pot)) as f32
        } else {
            0.0
        },
    }
}

/// Moves the game to `path` (from the root) and describes the node.
pub fn view(game: &mut PostFlopGame, path: &[usize]) -> Result<NodeView, String> {
    game.back_to_root();
    for &a in path {
        if game.is_terminal_node() {
            return Err("path goes past a terminal node".into());
        }
        if game.is_chance_node() {
            if game.possible_cards() & (1u64 << a) == 0 {
                return Err(format!("card {a} cannot come here"));
            }
        } else if a >= game.available_actions().len() {
            return Err(format!("no action {a} here"));
        }
        game.play(a);
    }
    let board: Vec<String> = game.current_board().iter().map(|&c| card_str(c)).collect();
    let bets = game.total_bet_amount();
    let start = game.tree_config().starting_pot as f64;
    let pot = (start + bets[0] as f64 + bets[1] as f64) / SCALE;
    let bets_bb = [bets[0] as f64 / SCALE, bets[1] as f64 / SCALE];
    let mut v = NodeView {
        path: path.to_vec(),
        board,
        kind: "action",
        player: None,
        actions: vec![],
        totals: vec![],
        cards: vec![],
        pot,
        bets: bets_bb,
        locked: false,
        players: vec![],
    };
    if game.is_terminal_node() {
        v.kind = "terminal";
        return Ok(v);
    }
    if game.is_chance_node() {
        v.kind = "chance";
        let m = game.possible_cards();
        v.cards = (0..52u8)
            .filter(|c| m & (1u64 << c) != 0)
            .map(card_str)
            .collect();
        return Ok(v);
    }
    let actor = game.current_player();
    v.player = Some(actor);
    v.actions = game.available_actions().iter().map(action_info).collect();
    v.locked = game.current_locking_strategy().is_some();
    game.cache_normalized_weights();
    v.players = (0..2)
        .map(|p| player_view(game, p, Some(actor), pot))
        .collect();
    let me = &v.players[actor];
    let wsum: f32 = me.grid.iter().map(|g| g.weight).sum();
    v.totals = (0..v.actions.len())
        .map(|a| {
            if wsum > 0.0 {
                me.grid.iter().map(|g| g.weight * g.freq[a]).sum::<f32>() / wsum
            } else {
                0.0
            }
        })
        .collect();
    Ok(v)
}

/// The current node's strategy as CSV (hand; weight; one column per action; EV; equity).
pub fn to_csv(v: &NodeView) -> String {
    let Some(p) = v.player else {
        return String::new();
    };
    let mut out = String::from("hand;weight");
    for a in &v.actions {
        out.push_str(&format!(
            ";{}{}",
            a.kind,
            if a.amount > 0.0 {
                format!(" {}", a.amount)
            } else {
                String::new()
            }
        ));
    }
    out.push_str(";ev_bb;equity\n");
    for h in &v.players[p].hands {
        out.push_str(&format!("{};{:.4}", h.cards, h.weight));
        for f in &h.strategy {
            out.push_str(&format!(";{f:.4}"));
        }
        out.push_str(&format!(";{:.4};{:.4}\n", h.ev, h.equity));
    }
    out
}
