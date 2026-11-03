//! Equity Lab maths: ranges, equity (exhaustive and Monte Carlo), range
//! breakdown on a board, flop textures and the preflop ranking.
//!
//! See `docs/equity.md` for the definitions and the maths.

pub mod breakdown;
pub mod calc;
pub mod combo;
pub mod preflop;
pub mod range;
pub mod ranking;
pub mod texture;

pub use calc::{equity, CalcError, EquityResult, Mode, Options};
pub use range::{Range, RangeError};
