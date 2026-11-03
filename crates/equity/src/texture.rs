//! Flop textures: a range's breakdown over all flops, or a subset.
//!
//! The 22 100 flops fall into 1 755 classes that only differ by a renaming of
//! the suits ("strategically different" flops). A suit renaming does not
//! change a suit-symmetric range's breakdown, but a range may be suit-specific
//! (`AhKh`), so the breakdown is computed on **every** one of the 22 100 flops
//! and then summed per class: the result is exact for any range.
//! Every flop counts the same (shares are averaged over flops).

use std::collections::HashMap;
use std::sync::OnceLock;

use lps_evaluator::{card, card_str, rank_of, suit_of, Card, Hand};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::breakdown::{classify, Draw, Made, Row};
use crate::range::Range;

/// One class of flops.
#[derive(Debug, Clone)]
pub struct FlopClass {
    /// Representative flop, high card first.
    pub cards: [Card; 3],
    /// Every flop of the class.
    pub members: Vec<[Card; 3]>,
}

/// The 1 755 flop classes.
pub fn flop_classes() -> &'static [FlopClass] {
    static F: OnceLock<Vec<FlopClass>> = OnceLock::new();
    F.get_or_init(|| {
        let perms = suit_permutations();
        let mut map: HashMap<[Card; 3], Vec<[Card; 3]>> = HashMap::new();
        for a in 0..52u8 {
            for b in a + 1..52 {
                for c in b + 1..52 {
                    let key = perms
                        .iter()
                        .map(|p| {
                            let mut x = [a, b, c].map(|k| card(rank_of(k), p[suit_of(k) as usize]));
                            x.sort_unstable_by(|u, v| v.cmp(u));
                            x
                        })
                        .min()
                        .unwrap();
                    map.entry(key).or_default().push([a, b, c]);
                }
            }
        }
        let mut out: Vec<FlopClass> = map
            .into_iter()
            .map(|(cards, members)| FlopClass { cards, members })
            .collect();
        out.sort_by(|x, y| {
            y.cards
                .map(rank_of)
                .cmp(&x.cards.map(rank_of))
                .then(x.cards.cmp(&y.cards))
        });
        out
    })
}

fn suit_permutations() -> Vec<[u8; 4]> {
    let mut out = Vec::new();
    for a in 0..4u8 {
        for b in 0..4u8 {
            for c in 0..4u8 {
                for d in 0..4u8 {
                    let p = [a, b, c, d];
                    if (0..4).all(|i| (0..4).all(|j| i == j || p[i] != p[j])) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FlopFilter {
    /// "monotone", "two_tone" or "rainbow".
    pub suits: Option<String>,
    /// "unpaired", "paired" or "trips".
    pub pairing: Option<String>,
    /// Highest card's rank between these (0 = deuce … 12 = ace).
    pub high_min: Option<u8>,
    pub high_max: Option<u8>,
    /// Three different ranks that fit in a straight.
    pub connected: Option<bool>,
}

impl FlopFilter {
    pub fn matches(&self, f: &[Card; 3]) -> bool {
        let suits = Hand::from_cards(f);
        let n_suits = (0..4).filter(|&s| suits.suit_mask(s) != 0).count();
        let ranks = suits.ranks();
        let n_ranks = ranks.count_ones();
        let high = f.iter().map(|&c| rank_of(c)).max().unwrap();
        let ok_suits = match self.suits.as_deref() {
            Some("monotone") => n_suits == 1,
            Some("two_tone") => n_suits == 2,
            Some("rainbow") => n_suits == 3,
            _ => true,
        };
        let ok_pair = match self.pairing.as_deref() {
            Some("unpaired") => n_ranks == 3,
            Some("paired") => n_ranks == 2,
            Some("trips") => n_ranks == 1,
            _ => true,
        };
        // a window of five ranks (the wheel A-5 included) holds all three
        let connected = n_ranks == 3
            && std::iter::once(0x100fu16)
                .chain((0..=8).map(|lo| 0x1fu16 << lo))
                .any(|w| ranks & w == ranks);
        ok_suits
            && ok_pair
            && self.high_min.is_none_or(|m| high >= m)
            && self.high_max.is_none_or(|m| high <= m)
            && self.connected.is_none_or(|c| c == connected)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlopRow {
    pub cards: String,
    /// Number of actual flops in the class.
    pub weight: u32,
    /// Average shares, in the order of [`Made::ALL`] and [`Draw::ALL`].
    pub made: Vec<f64>,
    pub draws: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureReport {
    /// Flop classes kept by the filter.
    pub classes: u32,
    /// Actual flops kept.
    pub flops: u32,
    pub made: Vec<Row>,
    pub draws: Vec<Row>,
    pub rows: Vec<FlopRow>,
}

fn shares(range: &Range, flop: &[Card; 3]) -> ([f64; 15], [f64; 6]) {
    let mut made = [0.0; 15];
    let mut draws = [0.0; 6];
    let mut total = 0.0;
    for (c, w) in range.live(Hand::from_cards(flop)) {
        let cl = classify(c, flop);
        total += w;
        made[cl.made as usize] += w;
        for d in Draw::ALL {
            if cl.has(d) {
                draws[d as usize] += w;
            }
        }
    }
    if total > 0.0 {
        made.iter_mut().for_each(|x| *x /= total);
        draws.iter_mut().for_each(|x| *x /= total);
    }
    (made, draws)
}

pub fn texture(range: &Range, filter: &FlopFilter) -> TextureReport {
    let classes: Vec<&FlopClass> = flop_classes()
        .iter()
        .filter(|f| filter.matches(&f.cards))
        .collect();
    let rows: Vec<FlopRow> = classes
        .par_iter()
        .map(|f| {
            let mut made = vec![0.0; 15];
            let mut draws = vec![0.0; 6];
            for m in &f.members {
                let (a, b) = shares(range, m);
                made.iter_mut().zip(a).for_each(|(x, y)| *x += y);
                draws.iter_mut().zip(b).for_each(|(x, y)| *x += y);
            }
            let n = f.members.len() as f64;
            made.iter_mut().for_each(|x| *x /= n);
            draws.iter_mut().for_each(|x| *x /= n);
            FlopRow {
                cards: f.cards.iter().map(|&c| card_str(c)).collect(),
                weight: f.members.len() as u32,
                made,
                draws,
            }
        })
        .collect();
    let flops: u32 = rows.iter().map(|r| r.weight).sum();
    let avg = |get: &dyn Fn(&FlopRow) -> f64| {
        if flops == 0 {
            0.0
        } else {
            rows.iter().map(|r| get(r) * r.weight as f64).sum::<f64>() / flops as f64
        }
    };
    let id = |v: serde_json::Value| v.as_str().unwrap_or_default().to_string();
    TextureReport {
        classes: rows.len() as u32,
        flops,
        made: Made::ALL
            .iter()
            .map(|&m| {
                let s = avg(&|r| r.made[m as usize]);
                Row {
                    id: id(serde_json::to_value(m).unwrap()),
                    combos: s * range.combo_count(),
                    share: s,
                }
            })
            .collect(),
        draws: Draw::ALL
            .iter()
            .map(|&d| {
                let s = avg(&|r| r.draws[d as usize]);
                Row {
                    id: id(serde_json::to_value(d).unwrap()),
                    combos: s * range.combo_count(),
                    share: s,
                }
            })
            .collect(),
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_1755_flop_classes() {
        let f = flop_classes();
        assert_eq!(f.len(), 1755);
        assert_eq!(f.iter().map(|c| c.members.len()).sum::<usize>(), 22_100);
        // 13 trips boards ×4, monotone and rainbow counts
        let count = |flt: FlopFilter| {
            f.iter()
                .filter(|c| flt.matches(&c.cards))
                .map(|c| c.members.len())
                .sum::<usize>()
        };
        assert_eq!(
            count(FlopFilter {
                pairing: Some("trips".into()),
                ..Default::default()
            }),
            13 * 4
        );
        assert_eq!(
            count(FlopFilter {
                suits: Some("monotone".into()),
                ..Default::default()
            }),
            4 * 286
        );
        assert_eq!(
            count(FlopFilter {
                pairing: Some("paired".into()),
                ..Default::default()
            }),
            13 * 6 * 12 * 4
        );
        // rainbow, unpaired: 286 rank sets × 4·3·2 suit choices
        assert_eq!(
            count(FlopFilter {
                suits: Some("rainbow".into()),
                pairing: Some("unpaired".into()),
                ..Default::default()
            }),
            286 * 24
        );
    }

    #[test]
    fn texture_of_pairs() {
        let r = Range::parse("AA").unwrap();
        let rep = texture(
            &r,
            &FlopFilter {
                suits: Some("monotone".into()),
                ..Default::default()
            },
        );
        assert_eq!(rep.flops, 4 * 286);
        let total: f64 = rep.made.iter().map(|m| m.share).sum();
        assert!((total - 1.0).abs() < 1e-9);
    }
}
