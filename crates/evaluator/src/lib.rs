//! Hand evaluator for 5, 6 and 7 cards.
//!
//! # Representation
//! A card is a number `0..52`: `rank * 4 + suit`, rank `0` = deuce … `12` = ace,
//! suit `0..4` = clubs, diamonds, hearts, spades (`"cdhs"`).
//!
//! A *hand* (any set of cards) is a [`Hand`]: a 64-bit mask with one 16-bit lane
//! per suit; bit `rank` of lane `suit` is set when the card is present. Adding a
//! card is an OR, so the board can be combined with each hole-card pair for free.
//!
//! # Algorithm (see also `docs/evaluator.md`)
//! With four 13-bit suit masks `a, b, c, d`:
//! * a flush exists when one suit has ≥ 5 bits. With at most 7 cards, a flush
//!   excludes quads and full houses (they would need ≥ 8 cards), so the flush
//!   (or straight flush) is the answer;
//! * otherwise the rank masks are combined with bit operations:
//!   `any = a|b|c|d` (ranks present), `two = ranks present in ≥ 2 suits`,
//!   `three = ranks in ≥ 3 suits`, `four = a&b&c&d`;
//! * straights come from a precomputed 8 192-entry table indexed by a rank mask
//!   ([`STRAIGHT`]); the "top N ranks of a mask" is done by clearing low bits.
//!
//! # Value
//! [`Value`] is a `u32`: `category << 26 | major << 13 | minor`, where `major`
//! and `minor` are 13-bit rank masks. Masks with the same number of bits compare
//! like the sorted rank lists they stand for, so **a bigger value is a better
//! hand** and equal values are a split pot.

use std::fmt;

/// A card `0..52` (`rank * 4 + suit`).
pub type Card = u8;

/// Hand strength; bigger is better.
pub type Value = u32;

pub const RANKS: &[u8; 13] = b"23456789TJQKA";
pub const SUITS: &[u8; 4] = b"cdhs";

/// Builds a card from rank `0..13` and suit `0..4`.
pub const fn card(rank: u8, suit: u8) -> Card {
    rank * 4 + suit
}

pub const fn rank_of(c: Card) -> u8 {
    c >> 2
}

pub const fn suit_of(c: Card) -> u8 {
    c & 3
}

/// Parses `"Ah"`, `"td"`… (rank case-insensitive, `10` accepted for ten).
pub fn parse_card(s: &str) -> Option<Card> {
    let s = s.trim();
    let (r, su) = if let Some(rest) = s.strip_prefix("10") {
        ('T', rest)
    } else {
        let mut it = s.chars();
        (it.next()?, it.as_str())
    };
    let mut sc = su.chars();
    let suit_ch = sc.next()?.to_ascii_lowercase();
    if sc.next().is_some() {
        return None;
    }
    let rank = RANKS
        .iter()
        .position(|&x| x as char == r.to_ascii_uppercase())? as u8;
    let suit = SUITS.iter().position(|&x| x as char == suit_ch)? as u8;
    Some(card(rank, suit))
}

/// Parses a run of cards like `"AhKd7c"` or `"Ah Kd 7c"`.
pub fn parse_cards(s: &str) -> Option<Vec<Card>> {
    let compact: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .collect();
    let compact = compact.replace("10", "T");
    let b = compact.as_bytes();
    if !b.len().is_multiple_of(2) {
        return None;
    }
    b.chunks(2)
        .map(|p| parse_card(std::str::from_utf8(p).ok()?))
        .collect()
}

pub fn card_str(c: Card) -> String {
    format!(
        "{}{}",
        RANKS[rank_of(c) as usize] as char,
        SUITS[suit_of(c) as usize] as char
    )
}

/// A set of cards as a 4×16-bit mask.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Hand(pub u64);

impl Hand {
    pub const EMPTY: Hand = Hand(0);

    #[inline]
    pub const fn bit(c: Card) -> u64 {
        1u64 << ((suit_of(c) as u64) * 16 + rank_of(c) as u64)
    }

    pub fn from_cards(cards: &[Card]) -> Hand {
        Hand(cards.iter().fold(0, |m, &c| m | Self::bit(c)))
    }

    #[inline]
    pub const fn with(self, c: Card) -> Hand {
        Hand(self.0 | Self::bit(c))
    }

    #[inline]
    pub const fn union(self, o: Hand) -> Hand {
        Hand(self.0 | o.0)
    }

    #[inline]
    pub const fn contains(self, c: Card) -> bool {
        self.0 & Self::bit(c) != 0
    }

    #[inline]
    pub const fn len(self) -> u32 {
        self.0.count_ones()
    }

    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// 13-bit rank mask of one suit.
    #[inline]
    pub const fn suit_mask(self, suit: u8) -> u16 {
        ((self.0 >> (suit as u64 * 16)) & 0x1fff) as u16
    }

    /// Ranks present in any suit.
    #[inline]
    pub const fn ranks(self) -> u16 {
        self.suit_mask(0) | self.suit_mask(1) | self.suit_mask(2) | self.suit_mask(3)
    }

    pub fn cards(self) -> impl Iterator<Item = Card> {
        (0u8..52).filter(move |&c| self.contains(c))
    }
}

impl fmt::Debug for Hand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: Vec<String> = self.cards().map(card_str).collect();
        write!(f, "Hand({})", s.join(" "))
    }
}

/// Hand categories, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Category {
    HighCard = 0,
    Pair = 1,
    TwoPair = 2,
    Trips = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    Quads = 7,
    StraightFlush = 8,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::HighCard,
        Category::Pair,
        Category::TwoPair,
        Category::Trips,
        Category::Straight,
        Category::Flush,
        Category::FullHouse,
        Category::Quads,
        Category::StraightFlush,
    ];

    pub const fn of(v: Value) -> Category {
        Self::ALL[(v >> 26) as usize]
    }
}

/// `STRAIGHT[mask]` = 1 + rank of the top card of the best straight in the rank
/// mask, or 0. The wheel (A-2-3-4-5) has top card 5 (rank 3).
pub static STRAIGHT: [u8; 8192] = build_straight();

const fn build_straight() -> [u8; 8192] {
    let mut t = [0u8; 8192];
    let mut m = 0usize;
    while m < 8192 {
        let mut top = 12i32;
        while top >= 4 {
            let run = 0x1fusize << (top - 4);
            if m & run == run {
                t[m] = top as u8 + 1;
                break;
            }
            top -= 1;
        }
        if t[m] == 0 {
            let wheel = 0x100f; // A,5,4,3,2
            if m & wheel == wheel {
                t[m] = 3 + 1;
            }
        }
        m += 1;
    }
    t
}

/// Keeps the `n` highest bits of `m`.
#[inline]
fn top_n(mut m: u16, n: u32) -> u16 {
    while m.count_ones() > n {
        m &= m - 1; // clears the lowest set bit
    }
    m
}

#[inline]
fn top1(m: u16) -> u16 {
    if m == 0 {
        0
    } else {
        1 << (15 - m.leading_zeros())
    }
}

#[inline]
const fn value(cat: Category, major: u16, minor: u16) -> Value {
    ((cat as u32) << 26) | ((major as u32) << 13) | minor as u32
}

/// Evaluates a hand of 5 to 7 cards (fewer cards also work: missing cards
/// simply never make straights or flushes).
#[inline]
pub fn evaluate(h: Hand) -> Value {
    let a = h.suit_mask(0);
    let b = h.suit_mask(1);
    let c = h.suit_mask(2);
    let d = h.suit_mask(3);
    for s in [a, b, c, d] {
        if s.count_ones() >= 5 {
            let st = STRAIGHT[s as usize];
            return if st != 0 {
                value(Category::StraightFlush, 1 << (st - 1), 0)
            } else {
                value(Category::Flush, top_n(s, 5), 0)
            };
        }
    }
    let any = a | b | c | d;
    let two = (a & b) | (c & d) | ((a | b) & (c | d));
    let three = (a & b & (c | d)) | (c & d & (a | b));
    let four = a & b & c & d;
    if four != 0 {
        return value(Category::Quads, four, top1(any & !four));
    }
    if three != 0 {
        let t = top1(three);
        let pair = top1(two & !t);
        if pair != 0 {
            return value(Category::FullHouse, t, pair);
        }
    }
    let st = STRAIGHT[any as usize];
    if st != 0 {
        return value(Category::Straight, 1 << (st - 1), 0);
    }
    if three != 0 {
        return value(Category::Trips, three, top_n(any & !three, 2));
    }
    match two.count_ones() {
        0 => value(Category::HighCard, top_n(any, 5), 0),
        1 => value(Category::Pair, two, top_n(any & !two, 3)),
        _ => {
            let pairs = top_n(two, 2);
            value(Category::TwoPair, pairs, top1(any & !pairs))
        }
    }
}

/// Convenience: evaluates a slice of cards.
pub fn evaluate_cards(cards: &[Card]) -> Value {
    evaluate(Hand::from_cards(cards))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Value {
        evaluate_cards(&parse_cards(s).unwrap())
    }

    #[test]
    fn parse_and_print() {
        assert_eq!(parse_card("Ah"), Some(card(12, 2)));
        assert_eq!(parse_card("2c"), Some(0));
        assert_eq!(parse_card("10s"), Some(card(8, 3)));
        assert_eq!(parse_card("Ax"), None);
        assert_eq!(parse_cards("AhKd 7c").unwrap().len(), 3);
        assert_eq!(card_str(51), "As");
    }

    #[test]
    fn categories() {
        assert_eq!(Category::of(v("AsKsQsJsTs2d3c")), Category::StraightFlush);
        assert_eq!(Category::of(v("As2s3s4s5s9d9c")), Category::StraightFlush);
        assert_eq!(Category::of(v("AsAdAhAc2d")), Category::Quads);
        assert_eq!(Category::of(v("AsAdAhKcKd2c")), Category::FullHouse);
        assert_eq!(Category::of(v("AsAdAhKcKdKh")), Category::FullHouse);
        assert_eq!(Category::of(v("2s7s9sJsKs")), Category::Flush);
        assert_eq!(Category::of(v("Ad2s3c4h5h")), Category::Straight);
        assert_eq!(Category::of(v("7d7s7c2h5h")), Category::Trips);
        assert_eq!(Category::of(v("7d7s2c2h5h")), Category::TwoPair);
        assert_eq!(Category::of(v("7d7s2c3h5h")), Category::Pair);
        assert_eq!(Category::of(v("7d8s2c3h5h")), Category::HighCard);
    }

    #[test]
    fn ordering() {
        // wheel < six-high straight
        assert!(v("Ad2s3c4h5h") < v("2d3s4c5h6h"));
        // three pairs: best two pairs + best kicker among the rest
        assert_eq!(v("AsAdKsKd2c2dQh"), v("AsAdKsKdQh3c4d"));
        assert!(v("AsAdKsKd2c2dQh") > v("AsAdKsKdJh2c2d"));
        // kickers
        assert!(v("AsAd9c8d7h") > v("AhAc9s8d6h"));
        // split pot (board plays)
        assert_eq!(v("2c3dAsKsQsJsTs"), v("4c5dAsKsQsJsTs"));
        // flush beats straight even when both are present
        assert_eq!(Category::of(v("5h6h7h8h2h9c")), Category::Flush);
    }

    /// Known combinatorics: number of 5-card hands of each category.
    #[test]
    fn all_five_card_hands() {
        let mut counts = [0u32; 9];
        for a in 0..52u8 {
            for b in a + 1..52 {
                for c in b + 1..52 {
                    for d in c + 1..52 {
                        let h4 = Hand::from_cards(&[a, b, c, d]);
                        for e in d + 1..52 {
                            counts[Category::of(evaluate(h4.with(e))) as usize] += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(
            counts,
            [1_302_540, 1_098_240, 123_552, 54_912, 10_200, 5_108, 3_744, 624, 40]
        );
    }

    /// Known combinatorics: number of 7-card hands of each category (133 784 560 hands).
    #[test]
    fn all_seven_card_hands() {
        let mut counts = [0u64; 9];
        let cards: Vec<Card> = (0..52).collect();
        rec7(&cards, 0, Hand::EMPTY, 0, &mut counts);
        assert_eq!(
            counts,
            [
                23_294_460, 58_627_800, 31_433_400, 6_461_620, 6_180_020, 4_047_644, 3_473_184,
                224_848, 41_584
            ]
        );
    }

    fn rec7(cards: &[Card], start: usize, h: Hand, n: u32, counts: &mut [u64; 9]) {
        if n == 7 {
            counts[Category::of(evaluate(h)) as usize] += 1;
            return;
        }
        for i in start..=(52 - (7 - n) as usize) {
            rec7(cards, i + 1, h.with(cards[i]), n + 1, counts);
        }
    }
}
