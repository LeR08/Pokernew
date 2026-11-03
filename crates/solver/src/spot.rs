//! A postflop spot: ranges, board, pot, stacks, rake and bet tree, turned into
//! a `postflop_solver::PostFlopGame`.
//!
//! Amounts are in big blinds; the engine works in integer chips, so every
//! amount is multiplied by [`SCALE`] (0.01 bb precision).

use lps_equity::combo::all_combos;
use lps_equity::Range as OurRange;
use postflop_solver::{
    card_from_str, flop_from_str, ActionTree, BetSizeOptions, BoardState, CardConfig,
    DonkSizeOptions, PostFlopGame, Range, TreeConfig, NOT_DEALT,
};
use serde::{Deserialize, Serialize};

/// Chips per big blind inside the engine.
pub const SCALE: f64 = 100.0;

/// Bet and raise sizes of one player on one street, in the engine's syntax:
/// `"33%, 75%"` (pot fractions), `"2.5x"` (times the last bet), `"e"`
/// (geometric), `"a"` (all-in), several separated by commas.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Sizes {
    pub bet: String,
    pub raise: String,
}

/// One street: OOP and IP sizes, plus OOP's donk sizes (turn and river).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct StreetSizes {
    pub oop: Sizes,
    pub ip: Sizes,
    /// Empty: OOP leads with its normal bet sizes.
    pub donk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Tree {
    pub flop: StreetSizes,
    pub turn: StreetSizes,
    pub river: StreetSizes,
    /// Add an all-in when the biggest bet is at most this many pots.
    pub add_allin_threshold: f64,
    /// Force all-in when the stack-to-pot ratio after a call would be at most this.
    pub force_allin_threshold: f64,
    /// Merge bet sizes closer than this fraction of the pot.
    pub merging_threshold: f64,
    /// Raises allowed after the first bet of a street (2 = bet, raise, re-raise); 0 = no limit.
    pub max_raises: u32,
}

impl Default for Tree {
    fn default() -> Self {
        let s = StreetSizes {
            oop: Sizes {
                bet: "33%, 75%".into(),
                raise: "2.5x".into(),
            },
            ip: Sizes {
                bet: "33%, 75%".into(),
                raise: "2.5x".into(),
            },
            donk: String::new(),
        };
        Tree {
            flop: s.clone(),
            turn: s.clone(),
            river: s,
            add_allin_threshold: 1.5,
            force_allin_threshold: 0.15,
            merging_threshold: 0.1,
            max_raises: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SpotConfig {
    /// Ranges in Equity Lab notation (`TT+, AQs+, KQo:50%`).
    pub oop_range: String,
    pub ip_range: String,
    /// 3 to 5 cards, e.g. `"Td9d6h"`.
    pub board: String,
    /// Pot and effective stack at the start of the street, in bb.
    pub pot: f64,
    pub stack: f64,
    /// Rake: fraction of the pot (0.05 = 5 %) and cap in bb (0 = no rake).
    pub rake: f64,
    pub rake_cap: f64,
    pub tree: Tree,
}

impl Default for SpotConfig {
    fn default() -> Self {
        SpotConfig {
            oop_range: String::new(),
            ip_range: String::new(),
            board: String::new(),
            pot: 5.5,
            stack: 97.5,
            rake: 0.0,
            rake_cap: 0.0,
            tree: Tree::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SpotError {
    #[error("{0}: {1}")]
    Range(&'static str, String),
    #[error("the board must have 3 to 5 different cards")]
    Board,
    #[error("pot and stack must be positive")]
    Amounts,
    #[error("rake must be between 0 and 100 %")]
    Rake,
    #[error("bet sizes ({0}): {1}")]
    Sizes(String, String),
    #[error("{0}")]
    Engine(String),
}

fn engine_range(text: &str, who: &'static str) -> Result<Range, SpotError> {
    let ours = OurRange::parse(text).map_err(|e| SpotError::Range(who, e.to_string()))?;
    if ours.combo_count() == 0.0 {
        return Err(SpotError::Range(who, "empty range".into()));
    }
    let mut r = Range::new();
    for (c, &w) in all_combos().iter().zip(&ours.weights) {
        if w > 0.0 {
            r.set_weight_by_cards(c.hi, c.lo, w);
        }
    }
    Ok(r)
}

fn sizes(s: &Sizes, what: &str) -> Result<BetSizeOptions, SpotError> {
    BetSizeOptions::try_from((s.bet.as_str(), s.raise.as_str()))
        .map_err(|e| SpotError::Sizes(what.into(), e))
}

fn donk(s: &str, what: &str) -> Result<Option<DonkSizeOptions>, SpotError> {
    if s.trim().is_empty() {
        return Ok(None);
    }
    DonkSizeOptions::try_from(s)
        .map(Some)
        .map_err(|e| SpotError::Sizes(what.into(), e))
}

/// Board cards, checked: 3 to 5 distinct cards.
pub fn board_cards(board: &str) -> Result<Vec<u8>, SpotError> {
    let cards = lps_evaluator::parse_cards(board).ok_or(SpotError::Board)?;
    let set: std::collections::HashSet<_> = cards.iter().collect();
    if !(3..=5).contains(&cards.len()) || set.len() != cards.len() {
        return Err(SpotError::Board);
    }
    Ok(cards)
}

impl SpotConfig {
    pub fn tree_config(&self) -> Result<(TreeConfig, BoardState), SpotError> {
        if !(self.pot > 0.0 && self.stack > 0.0 && self.pot.is_finite() && self.stack.is_finite()) {
            return Err(SpotError::Amounts);
        }
        if !(0.0..1.0).contains(&self.rake) || self.rake_cap < 0.0 {
            return Err(SpotError::Rake);
        }
        let n = board_cards(&self.board)?.len();
        let state = match n {
            3 => BoardState::Flop,
            4 => BoardState::Turn,
            _ => BoardState::River,
        };
        let t = &self.tree;
        let cfg = TreeConfig {
            initial_state: state,
            starting_pot: (self.pot * SCALE).round() as i32,
            effective_stack: (self.stack * SCALE).round() as i32,
            rake_rate: self.rake,
            rake_cap: self.rake_cap * SCALE,
            flop_bet_sizes: [
                sizes(&t.flop.oop, "flop OOP")?,
                sizes(&t.flop.ip, "flop IP")?,
            ],
            turn_bet_sizes: [
                sizes(&t.turn.oop, "turn OOP")?,
                sizes(&t.turn.ip, "turn IP")?,
            ],
            river_bet_sizes: [
                sizes(&t.river.oop, "river OOP")?,
                sizes(&t.river.ip, "river IP")?,
            ],
            turn_donk_sizes: donk(&t.turn.donk, "turn donk")?,
            river_donk_sizes: donk(&t.river.donk, "river donk")?,
            add_allin_threshold: t.add_allin_threshold,
            force_allin_threshold: t.force_allin_threshold,
            merging_threshold: t.merging_threshold,
        };
        Ok((cfg, state))
    }

    pub fn card_config(&self) -> Result<CardConfig, SpotError> {
        let cards = board_cards(&self.board)?;
        let s: Vec<String> = cards.iter().map(|&c| lps_evaluator::card_str(c)).collect();
        let flop = flop_from_str(&s[..3].concat()).map_err(SpotError::Engine)?;
        let turn = match s.get(3) {
            Some(c) => card_from_str(c).map_err(SpotError::Engine)?,
            None => NOT_DEALT,
        };
        let river = match s.get(4) {
            Some(c) => card_from_str(c).map_err(SpotError::Engine)?,
            None => NOT_DEALT,
        };
        Ok(CardConfig {
            range: [
                engine_range(&self.oop_range, "OOP range")?,
                engine_range(&self.ip_range, "IP range")?,
            ],
            flop,
            turn,
            river,
        })
    }

    /// The betting tree, with the raise cap applied.
    pub fn action_tree(&self) -> Result<ActionTree, SpotError> {
        let (tree, _) = self.tree_config()?;
        let mut t = ActionTree::new(tree).map_err(SpotError::Engine)?;
        if self.tree.max_raises > 0 {
            cap_raises(&mut t, self.tree.max_raises);
        }
        Ok(t)
    }

    /// Builds the game (memory not allocated yet).
    pub fn build(&self) -> Result<PostFlopGame, SpotError> {
        PostFlopGame::with_config(self.card_config()?, self.action_tree()?)
            .map_err(SpotError::Engine)
    }

    /// Stable fingerprint of the spot (FNV-1a over its canonical JSON).
    pub fn hash(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in json.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{h:016x}")
    }
}

/// Size of a game before solving.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Estimate {
    /// Bytes needed, 32-bit floats / 16-bit compression.
    pub memory: u64,
    pub memory_compressed: u64,
    /// Action nodes of the betting tree (one street path; chance nodes not expanded).
    pub tree_nodes: usize,
    pub oop_hands: usize,
    pub ip_hands: usize,
}

pub fn estimate(spot: &SpotConfig) -> Result<Estimate, SpotError> {
    let game = spot.build()?;
    let (memory, memory_compressed) = game.memory_usage();
    let mut action_tree = spot.action_tree()?;
    Ok(Estimate {
        memory,
        memory_compressed,
        tree_nodes: count_nodes(&mut action_tree),
        oop_hands: game.private_cards(0).len(),
        ip_hands: game.private_cards(1).len(),
    })
}

/// Number of decision nodes of the betting tree (chance nodes count once, not per card).
fn count_nodes(t: &mut ActionTree) -> usize {
    fn rec(t: &mut ActionTree) -> usize {
        if t.is_terminal_node() {
            return 0;
        }
        let mut n = usize::from(!t.is_chance_node());
        for a in t.available_actions().to_vec() {
            if t.play(a).is_ok() {
                n += rec(t);
                let _ = t.undo();
            }
        }
        n
    }
    t.back_to_root();
    rec(t)
}

/// Removes raises once `max` raises have been made after the first bet of a street.
fn cap_raises(t: &mut ActionTree, max: u32) {
    use postflop_solver::Action;
    fn rec(t: &mut ActionTree, max: u32, raises: u32, facing: bool) {
        if t.is_terminal_node() {
            return;
        }
        if t.is_chance_node() {
            for a in t.available_actions().to_vec() {
                if t.play(a).is_ok() {
                    rec(t, max, 0, false);
                    let _ = t.undo();
                }
            }
            return;
        }
        if facing && raises >= max {
            for a in t.available_actions().to_vec() {
                if matches!(a, Action::Raise(_) | Action::AllIn(_)) {
                    let _ = t.remove_action(a);
                }
            }
        }
        for a in t.available_actions().to_vec() {
            let (r, f) = match a {
                Action::Bet(_) => (raises, true),
                Action::Raise(_) => (raises + 1, true),
                Action::AllIn(_) if facing => (raises + 1, true),
                Action::AllIn(_) => (raises, true),
                _ => (raises, false),
            };
            if t.play(a).is_ok() {
                rec(t, max, r, f);
                let _ = t.undo();
            }
        }
    }
    t.back_to_root();
    rec(t, max, 0, false);
    t.back_to_root();
}
