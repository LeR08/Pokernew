//! Stats engine: user-editable stat definitions, an expression language
//! compiled to SQL, and the report queries of the tracker.

pub mod bankroll;
pub mod compile;
pub mod defs;
pub mod expr;
pub mod grid;
pub mod leaks;
pub mod query;
pub mod review;

pub use defs::{compile as compile_stat, CompiledStat, StatDef, StatError, StatsFile};

/// Compiles every stat of the given files; returns the stats and the errors (by id).
pub fn compile_all(files: &[StatsFile]) -> (Vec<CompiledStat>, Vec<(String, StatError)>) {
    let mut ok = Vec::new();
    let mut errs = Vec::new();
    for f in files {
        for (id, def) in &f.stat {
            match compile_stat(id, def) {
                Ok(c) => ok.push(c),
                Err(e) => errs.push((id.clone(), e)),
            }
        }
    }
    (ok, errs)
}

#[cfg(test)]
mod tests;
