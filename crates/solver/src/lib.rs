//! Postflop solver: our layer around the vendored `postflop-solver` engine
//! (`crates/solver/engine`, AGPL-3.0, Discounted CFR).
//!
//! * [`spot`]: ranges, board, pot, stacks, rake and bet tree → engine game;
//! * [`job`]: background solve with progress, pause/resume, cancel, node locks;
//! * [`view`]: strategy, EV, equity and 13×13 grids at any node;
//! * [`cache`]: compressed solutions on disk, reloaded without solving.

pub mod batch;
pub mod cache;
pub mod job;
pub mod spot;
pub mod view;

pub use job::{Job, Lock, Progress, SolveParams, State};
pub use spot::{estimate, Estimate, Sizes, SpotConfig, SpotError, StreetSizes, Tree};
pub use view::{view, NodeView};
