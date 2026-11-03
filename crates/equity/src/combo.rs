//! The 1 326 two-card combos and the 169 starting-hand classes.
//!
//! Combo index (shared with the front end, `src/lib/range.ts`): for cards
//! `hi > lo`, `index = hi * (hi - 1) / 2 + lo`.
//!
//! Class grid (13×13, as in every range editor): row/column 0 = ace … 12 = deuce;
//! the diagonal holds pairs, above it (row < column) suited hands, below it
//! offsuit hands.

use lps_evaluator::{rank_of, suit_of, Card, Hand, RANKS};

pub const NUM_COMBOS: usize = 1326;
pub const NUM_CLASSES: usize = 169;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Combo {
    /// Higher card first.
    pub hi: Card,
    pub lo: Card,
}

impl Combo {
    pub fn new(a: Card, b: Card) -> Combo {
        debug_assert_ne!(a, b);
        if a > b {
            Combo { hi: a, lo: b }
        } else {
            Combo { hi: b, lo: a }
        }
    }

    pub const fn index(self) -> usize {
        let h = self.hi as usize;
        h * (h - 1) / 2 + self.lo as usize
    }

    pub fn mask(self) -> Hand {
        Hand::from_cards(&[self.hi, self.lo])
    }

    /// Grid cell of this combo's class: `row * 13 + col`.
    pub fn class(self) -> usize {
        let (r1, r2) = (
            12 - rank_of(self.hi) as usize,
            12 - rank_of(self.lo) as usize,
        );
        if r1 == r2 {
            r1 * 13 + r1
        } else if suit_of(self.hi) == suit_of(self.lo) {
            r1.min(r2) * 13 + r1.max(r2)
        } else {
            r1.max(r2) * 13 + r1.min(r2)
        }
    }
}

/// All combos, by index.
pub fn all_combos() -> &'static [Combo; NUM_COMBOS] {
    static ALL: std::sync::OnceLock<[Combo; NUM_COMBOS]> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut out = [Combo { hi: 1, lo: 0 }; NUM_COMBOS];
        for hi in 1..52u8 {
            for lo in 0..hi {
                let c = Combo { hi, lo };
                out[c.index()] = c;
            }
        }
        out
    })
}

/// Name of a grid cell (`"AKs"`, `"TT"`, `"72o"`).
pub fn class_name(cell: usize) -> String {
    let (row, col) = (cell / 13, cell % 13);
    let r = |i: usize| RANKS[12 - i] as char;
    match row.cmp(&col) {
        std::cmp::Ordering::Equal => format!("{}{}", r(row), r(row)),
        std::cmp::Ordering::Less => format!("{}{}s", r(row), r(col)),
        std::cmp::Ordering::Greater => format!("{}{}o", r(col), r(row)),
    }
}

/// Number of combos in a class: 6 (pair), 4 (suited) or 12 (offsuit).
pub const fn class_size(cell: usize) -> usize {
    let (row, col) = (cell / 13, cell % 13);
    if row == col {
        6
    } else if row < col {
        4
    } else {
        12
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lps_evaluator::parse_card;

    #[test]
    fn indices_are_a_bijection() {
        let all = all_combos();
        for (i, c) in all.iter().enumerate() {
            assert_eq!(c.index(), i);
            assert!(c.hi > c.lo);
        }
    }

    #[test]
    fn classes() {
        let c = |a: &str, b: &str| Combo::new(parse_card(a).unwrap(), parse_card(b).unwrap());
        assert_eq!(class_name(c("Ah", "Kh").class()), "AKs");
        assert_eq!(class_name(c("Kd", "Ah").class()), "AKo");
        assert_eq!(class_name(c("2d", "2h").class()), "22");
        assert_eq!(class_name(c("7d", "2h").class()), "72o");
        let mut sizes = [0usize; NUM_CLASSES];
        for x in all_combos() {
            sizes[x.class()] += 1;
        }
        for (cell, n) in sizes.iter().enumerate() {
            assert_eq!(*n, class_size(cell), "{}", class_name(cell));
        }
    }
}
