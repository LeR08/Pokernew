//! Stat definitions and the expression compiler (expression -> SQL).
//!
//! Placeholder created in phase 0 to fix the workspace layout.
//! Implementation lands in phase 2 (see `docs/phase-2.md`).

/// Phase in which this crate gets its implementation.
pub const IMPLEMENTED_IN_PHASE: u8 = 2;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_phase_is_set() {
        assert_eq!(super::IMPLEMENTED_IN_PHASE, 2);
    }
}
