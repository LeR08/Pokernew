//! Hand-history parsers, one module per room, plus format autodetection.
//!
//! Placeholder created in phase 0 to fix the workspace layout.
//! Implementation lands in phase 1 (see `docs/phase-1.md`).

/// Phase in which this crate gets its implementation.
pub const IMPLEMENTED_IN_PHASE: u8 = 1;

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_phase_is_set() {
        assert_eq!(super::IMPLEMENTED_IN_PHASE, 1);
    }
}
