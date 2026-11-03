//! Preflop ranking of the 169 starting hands, used by "top X %".
//!
//! The order is each hand's all-in equity against one random hand, computed
//! by this crate (`cargo run --release -p lps-equity --example gen_ranking`)
//! and stored in `data/preflop_ranking.json`. "Top X %" takes whole classes in
//! that order until X % of the 1 326 combos are covered (rounded to the nearest
//! class boundary, like the usual calculators).

use std::sync::OnceLock;

use serde::Deserialize;

use crate::combo::{all_combos, class_name, class_size, Combo, NUM_CLASSES, NUM_COMBOS};

#[derive(Debug, Clone, Deserialize)]
pub struct RankedHand {
    pub hand: String,
    pub equity: f64,
}

/// The 169 classes, best first.
pub fn ranking() -> &'static [RankedHand] {
    static R: OnceLock<Vec<RankedHand>> = OnceLock::new();
    R.get_or_init(|| {
        serde_json::from_str(include_str!("../data/preflop_ranking.json"))
            .expect("preflop_ranking.json")
    })
}

/// Grid cells (row * 13 + col), best first.
pub fn ranked_cells() -> &'static [usize] {
    static C: OnceLock<Vec<usize>> = OnceLock::new();
    C.get_or_init(|| {
        let by_name: std::collections::HashMap<String, usize> =
            (0..NUM_CLASSES).map(|c| (class_name(c), c)).collect();
        ranking().iter().map(|h| by_name[&h.hand]).collect()
    })
}

/// Number of classes making up the top `pct` %.
pub fn top_classes(pct: f64) -> usize {
    let target = pct / 100.0 * NUM_COMBOS as f64;
    let mut total = 0.0;
    for (i, &cell) in ranked_cells().iter().enumerate() {
        let next = total + class_size(cell) as f64;
        // stop at the boundary closest to the target
        if next > target && (next - target) > (target - total) {
            return i;
        }
        total = next;
    }
    ranked_cells().len()
}

pub fn top_percent(pct: f64) -> Vec<Combo> {
    let cells: std::collections::HashSet<usize> =
        ranked_cells()[..top_classes(pct)].iter().copied().collect();
    all_combos()
        .iter()
        .filter(|c| cells.contains(&c.class()))
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_and_ordered() {
        let r = ranking();
        assert_eq!(r.len(), NUM_CLASSES);
        assert_eq!(r[0].hand, "AA");
        assert!(r.windows(2).all(|w| w[0].equity >= w[1].equity));
        let cells: std::collections::HashSet<_> = ranked_cells().iter().collect();
        assert_eq!(cells.len(), NUM_CLASSES);
    }

    #[test]
    fn top_percent_sizes() {
        assert_eq!(top_percent(0.0).len(), 0);
        assert_eq!(top_percent(100.0).len(), NUM_COMBOS);
        assert_eq!(top_percent(0.5).len(), 6); // AA = 0.45 %
        let n = top_percent(10.0).len() as f64 / NUM_COMBOS as f64;
        assert!((n - 0.10).abs() < 0.01, "{n}");
    }
}
