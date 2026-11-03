//! What a range has on a board (Flopzilla-style breakdown).
//!
//! Every live combo gets exactly one **made-hand class** and any number of
//! **draws**. Definitions (also in `docs/equity.md`):
//!
//! Made hands, strongest first (the first that applies):
//! * straight flush, quads, full house, flush, straight — from the evaluator
//!   (also when the board alone makes the hand);
//! * set — pocket pair matching a board card; trips — any other three of a kind;
//! * pairs only count when a **hole card** makes them (a pair on the board
//!   alone is ignored):
//!   - two pair: both hole cards pair two different board ranks;
//!   - overpair: pocket pair above the highest board card;
//!   - top pair: a hole card pairs the highest board rank;
//!   - pocket pair below top pair: pocket pair between the two highest board ranks;
//!   - middle pair: a hole card pairs the second-highest board rank;
//!   - weak pair: a hole card pairs a lower board rank, or a pocket pair below
//!     the second-highest board rank;
//! * ace high: no pair from the hole cards, but an ace in hand;
//! * no made hand.
//!
//! Draws (flop and turn only), each needing at least one hole card:
//! * flush draw: four cards of a suit (not already a flush);
//! * OESD: two or more card ranks complete a straight (includes double gutshots);
//!   gutshot: exactly one rank (not already a straight); a rank only counts when
//!   it gives a better straight than the board plus that card alone;
//! * backdoor flush draw (flop): three cards of a suit;
//! * backdoor straight draw (flop): no straight draw, but two more ranks would
//!   make a straight;
//! * overcards: no pair from the hole cards and both hole cards above the board.

use lps_evaluator::{evaluate, rank_of, suit_of, Card, Category, Hand, STRAIGHT};
use serde::Serialize;

use crate::combo::Combo;
use crate::range::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Made {
    StraightFlush,
    Quads,
    FullHouse,
    Flush,
    Straight,
    Set,
    Trips,
    TwoPair,
    Overpair,
    TopPair,
    PpBelowTop,
    MiddlePair,
    WeakPair,
    AceHigh,
    NoMadeHand,
}

impl Made {
    pub const ALL: [Made; 15] = [
        Made::StraightFlush,
        Made::Quads,
        Made::FullHouse,
        Made::Flush,
        Made::Straight,
        Made::Set,
        Made::Trips,
        Made::TwoPair,
        Made::Overpair,
        Made::TopPair,
        Made::PpBelowTop,
        Made::MiddlePair,
        Made::WeakPair,
        Made::AceHigh,
        Made::NoMadeHand,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Draw {
    FlushDraw,
    Oesd,
    Gutshot,
    BackdoorFlush,
    BackdoorStraight,
    Overcards,
}

impl Draw {
    pub const ALL: [Draw; 6] = [
        Draw::FlushDraw,
        Draw::Oesd,
        Draw::Gutshot,
        Draw::BackdoorFlush,
        Draw::BackdoorStraight,
        Draw::Overcards,
    ];

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Class {
    pub made: Made,
    /// Bit set of [`Draw`]s.
    pub draws: u8,
}

impl Class {
    pub fn has(self, d: Draw) -> bool {
        self.draws & d.bit() != 0
    }
}

/// Straight top (1 + rank) for a rank mask, 0 when none.
fn straight(mask: u16) -> u8 {
    STRAIGHT[(mask & 0x1fff) as usize]
}

/// Classifies one combo on a board of 3 to 5 cards.
pub fn classify(combo: Combo, board: &[Card]) -> Class {
    let hole = [combo.hi, combo.lo];
    let bmask = Hand::from_cards(board);
    let all = bmask.union(combo.mask());
    let cat = Category::of(evaluate(all));
    let (h1, h2) = (rank_of(combo.hi), rank_of(combo.lo));
    let board_ranks = bmask.ranks();
    // distinct board ranks, high to low
    let mut br: Vec<u8> = (0..13u8)
        .rev()
        .filter(|r| board_ranks & (1 << r) != 0)
        .collect();
    br.dedup();
    let top = br[0];
    let second = br.get(1).copied();
    let pocket = h1 == h2;
    let in_board = |r: u8| board_ranks & (1 << r) != 0;

    let made = match cat {
        Category::StraightFlush => Made::StraightFlush,
        Category::Quads => Made::Quads,
        Category::FullHouse => Made::FullHouse,
        Category::Flush => Made::Flush,
        Category::Straight => Made::Straight,
        Category::Trips if pocket && in_board(h1) => Made::Set,
        Category::Trips => Made::Trips,
        _ => {
            if pocket {
                if h1 > top {
                    Made::Overpair
                } else if second.is_some_and(|s| h1 > s) {
                    Made::PpBelowTop
                } else {
                    Made::WeakPair
                }
            } else {
                match (in_board(h1), in_board(h2)) {
                    (true, true) => Made::TwoPair,
                    (true, false) | (false, true) => {
                        let m = if in_board(h1) { h1 } else { h2 };
                        if m == top {
                            Made::TopPair
                        } else if Some(m) == second {
                            Made::MiddlePair
                        } else {
                            Made::WeakPair
                        }
                    }
                    (false, false) if h1 == 12 || h2 == 12 => Made::AceHigh,
                    (false, false) => Made::NoMadeHand,
                }
            }
        }
    };

    let mut draws = 0u8;
    if board.len() < 5 {
        // flush draws
        for s in 0..4u8 {
            let n = all.suit_mask(s).count_ones();
            let mine = hole.iter().any(|&c| suit_of(c) == s);
            if !mine {
                continue;
            }
            if n == 4 && cat < Category::Flush {
                draws |= Draw::FlushDraw.bit();
            }
            if n == 3 && board.len() == 3 {
                draws |= Draw::BackdoorFlush.bit();
            }
        }
        // straight draws
        let ranks = all.ranks();
        if cat < Category::Straight {
            let outs = (0..13u8)
                .filter(|r| ranks & (1 << r) == 0)
                .filter(|&r| straight(ranks | 1 << r) > straight(board_ranks | 1 << r))
                .count();
            if outs >= 2 {
                draws |= Draw::Oesd.bit();
            } else if outs == 1 {
                draws |= Draw::Gutshot.bit();
            } else if board.len() == 3 {
                let bd = (0..13u8).any(|r1| {
                    (r1 + 1..13).any(|r2| {
                        let add = (1u16 << r1) | (1 << r2);
                        ranks & add == 0 && straight(ranks | add) > straight(board_ranks | add)
                    })
                });
                if bd {
                    draws |= Draw::BackdoorStraight.bit();
                }
            }
        }
    }
    if matches!(made, Made::AceHigh | Made::NoMadeHand) && h1.min(h2) > top {
        draws |= Draw::Overcards.bit();
    }
    Class { made, draws }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    /// Weighted combos.
    pub combos: f64,
    /// Share of the live range (0–1).
    pub share: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakdown {
    pub total: f64,
    pub made: Vec<Row>,
    pub draws: Vec<Row>,
}

fn id<T: Serialize>(x: T) -> String {
    serde_json::to_value(x)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Breakdown of `range` on `board` (3 to 5 cards); `dead` cards are removed too.
pub fn breakdown(range: &Range, board: &[Card], dead: &[Card]) -> Breakdown {
    let blocked = Hand::from_cards(board).union(Hand::from_cards(dead));
    let mut made = [0.0f64; 15];
    let mut draws = [0.0f64; 6];
    let mut total = 0.0;
    for (c, w) in range.live(blocked) {
        let cl = classify(c, board);
        total += w;
        made[cl.made as usize] += w;
        for d in Draw::ALL {
            if cl.has(d) {
                draws[d as usize] += w;
            }
        }
    }
    let share = |x: f64| if total > 0.0 { x / total } else { 0.0 };
    Breakdown {
        total,
        made: Made::ALL
            .iter()
            .map(|&m| Row {
                id: id(m),
                combos: made[m as usize],
                share: share(made[m as usize]),
            })
            .collect(),
        draws: Draw::ALL
            .iter()
            .map(|&d| Row {
                id: id(d),
                combos: draws[d as usize],
                share: share(draws[d as usize]),
            })
            .collect(),
    }
}

/// Keeps only the combos having one of the selected classes or draws (the range
/// for the next street). Weights are kept; other combos get 0.
pub fn filter(
    range: &Range,
    board: &[Card],
    dead: &[Card],
    made: &[Made],
    draws: &[Draw],
) -> Range {
    let blocked = Hand::from_cards(board).union(Hand::from_cards(dead));
    let mut out = Range::default();
    for (c, w) in range.live(blocked) {
        let cl = classify(c, board);
        if made.contains(&cl.made) || draws.iter().any(|&d| cl.has(d)) {
            out.weights[c.index()] = w as f32;
        }
    }
    out
}

pub fn parse_made(s: &str) -> Option<Made> {
    Made::ALL.into_iter().find(|&m| id(m) == s)
}

pub fn parse_draw(s: &str) -> Option<Draw> {
    Draw::ALL.into_iter().find(|&d| id(d) == s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lps_evaluator::parse_cards;

    fn bd(range: &str, board: &str) -> Breakdown {
        breakdown(
            &Range::parse(range).unwrap(),
            &parse_cards(board).unwrap(),
            &[],
        )
    }

    fn made(b: &Breakdown, id: &str) -> f64 {
        b.made.iter().find(|r| r.id == id).unwrap().combos
    }

    fn draw(b: &Breakdown, id: &str) -> f64 {
        b.draws.iter().find(|r| r.id == id).unwrap().combos
    }

    /// Counted by hand.
    #[test]
    fn hand_counted_cases() {
        // AA: 3 combos without Ah → set; KK: 6 → below top pair; AK: 3 aces × 4 kings → top pair.
        let b = bd("AA,KK,AK", "Ah7c2d");
        assert_eq!(b.total, 21.0);
        assert_eq!(made(&b, "set"), 3.0);
        assert_eq!(made(&b, "pp_below_top"), 6.0);
        assert_eq!(made(&b, "top_pair"), 12.0);

        // AKs on Qh7h2c: AhKh flush draw; AcKc backdoor flush; all four: ace high + overcards.
        let b = bd("AKs", "Qh7h2c");
        assert_eq!(made(&b, "ace_high"), 4.0);
        assert_eq!(draw(&b, "overcards"), 4.0);
        assert_eq!(draw(&b, "flush_draw"), 1.0);
        assert_eq!(draw(&b, "backdoor_flush"), 1.0);

        // JT on 9c8d2h: Q or 7 → OESD (16 combos). 65 on 9c8d2h: only 7 → gutshot.
        let b = bd("JT,65", "9c8d2h");
        assert_eq!(draw(&b, "oesd"), 16.0);
        assert_eq!(draw(&b, "gutshot"), 16.0);
        assert_eq!(made(&b, "no_made_hand"), 32.0);

        // Pairs only when a hole card makes them: 77 on KK2 sits between K and 2 → pocket pair below top pair.
        let b = bd("77,AQo", "KsKd2c");
        assert_eq!(made(&b, "pp_below_top"), 6.0);
        assert_eq!(made(&b, "ace_high"), 12.0);

        // Weights and dead cards.
        let b = breakdown(
            &Range::parse("KQo:50%").unwrap(),
            &parse_cards("Kh7c2d").unwrap(),
            &parse_cards("Qs").unwrap(),
        );
        // KQo without Kh and Qs: K∈{c,d,s} × Q∈{c,d,h}, offsuit → 9 − 2 (KcQc, KdQd) = 7, × 0.5
        assert_eq!(b.total, 3.5);
        assert_eq!(made(&b, "top_pair"), 3.5);

        // River: no draws.
        let b = bd("AKs", "Qh7h2c3h9d");
        assert_eq!(made(&b, "flush"), 1.0);
        assert!(b
            .draws
            .iter()
            .all(|r| r.combos == 0.0 || r.id == "overcards"));
    }

    /// The made classes are a partition of the range, on many boards.
    #[test]
    fn made_classes_partition_the_range() {
        let r = Range::parse("random").unwrap();
        for board in ["AsKsQs", "2c2d2h", "7h8h9hTh", "AcAdKcKd5s", "5c4d3h"] {
            let b = bd("random", board);
            let sum: f64 = b.made.iter().map(|x| x.combos).sum();
            assert_eq!(sum, b.total);
            let n = parse_cards(board).unwrap().len();
            let live = (52 - n) * (51 - n) / 2;
            assert_eq!(b.total, live as f64);
            assert_eq!(
                r.live_count(Hand::from_cards(&parse_cards(board).unwrap())),
                live as f64
            );
        }
    }

    #[test]
    fn filter_next_street() {
        let r = Range::parse("AA,KK,AK,JT").unwrap();
        let board = parse_cards("Ah7c2d").unwrap();
        let f = filter(&r, &board, &[], &[Made::Set, Made::TopPair], &[]);
        assert_eq!(f.combo_count(), 15.0);
        assert_eq!(parse_made("top_pair"), Some(Made::TopPair));
        assert_eq!(parse_draw("oesd"), Some(Draw::Oesd));
    }
}
