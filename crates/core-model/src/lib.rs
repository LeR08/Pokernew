//! Normalized poker model shared by every crate.
//!
//! Nothing here knows which room a hand came from: parsers translate their
//! text format into [`Hand`], and the database, stats, replayer and HUD only
//! ever see this structure.
//!
//! Amounts are integers in the hand's [`Unit`]: cents for cash games, chips
//! for tournaments. Big-blind units are derived (`amount / big_blind`).

pub mod card;
pub mod hand;
pub mod position;
pub mod preflop;

pub use card::{cards_to_string, parse_cards, Card, CardParseError, Rank, Suit};
/// Alias kept for test readability.
pub use card::cards_to_string as cards_to_string_compat;
pub use hand::*;
pub use position::{assign_positions, Position};
