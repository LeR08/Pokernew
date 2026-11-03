//! Preflop all-in equity between the 169 starting-hand classes.
//!
//! `equity(a, b)` is class `a`'s exact equity against class `b` (grid cells,
//! see [`crate::combo`]), averaged over every pair of combos that share no
//! card; `pairs(a, b)` is how many such pairs exist (card removal). The table
//! is computed once by `examples/gen_preflop_matrix.rs` and stored in
//! `data/preflop_matrix.txt`.

use std::sync::OnceLock;

use crate::combo::{all_combos, NUM_CLASSES};

pub struct PreflopMatrix {
    eq: Vec<f64>,
    pairs: Vec<f64>,
}

impl PreflopMatrix {
    #[inline]
    pub fn equity(&self, a: usize, b: usize) -> f64 {
        self.eq[a * NUM_CLASSES + b]
    }

    /// Number of combo pairs (a, b) with no shared card.
    #[inline]
    pub fn pairs(&self, a: usize, b: usize) -> f64 {
        self.pairs[a * NUM_CLASSES + b]
    }
}

pub fn matrix() -> &'static PreflopMatrix {
    static M: OnceLock<PreflopMatrix> = OnceLock::new();
    M.get_or_init(|| {
        let text = include_str!("../data/preflop_matrix.txt");
        let eq: Vec<f64> = text
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .flat_map(|l| {
                l.split_whitespace()
                    .map(|x| x.parse::<f64>().expect("preflop_matrix.txt"))
            })
            .collect();
        assert_eq!(eq.len(), NUM_CLASSES * NUM_CLASSES, "preflop_matrix.txt");
        let mut pairs = vec![0.0; NUM_CLASSES * NUM_CLASSES];
        let all = all_combos();
        for x in all.iter() {
            for y in all.iter() {
                if x.mask().0 & y.mask().0 == 0 {
                    pairs[x.class() * NUM_CLASSES + y.class()] += 1.0;
                }
            }
        }
        PreflopMatrix { eq, pairs }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combo::class_name;

    fn cell(name: &str) -> usize {
        (0..NUM_CLASSES).find(|&c| class_name(c) == name).unwrap()
    }

    #[test]
    fn matrix_is_consistent() {
        let m = matrix();
        for a in 0..NUM_CLASSES {
            for b in 0..NUM_CLASSES {
                assert!((m.equity(a, b) + m.equity(b, a) - 1.0).abs() < 2e-6);
                assert_eq!(m.pairs(a, b), m.pairs(b, a));
            }
        }
        assert_eq!(m.pairs(cell("AA"), cell("AA")), 6.0); // AA vs AA: only 6 disjoint pairs... (each combo has 1 disjoint partner)
        assert_eq!(m.pairs(cell("AKs"), cell("AKo")), 4.0 * 6.0); // each AKs combo leaves 2 aces × 3 kings, of which 6 offsuit
                                                                  // AA vs KK ≈ 82 %
        assert!((m.equity(cell("AA"), cell("KK")) - 0.8195).abs() < 0.002);
    }
}
