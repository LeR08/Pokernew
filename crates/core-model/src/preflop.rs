//! Starting-hand helpers: the 169 hand classes ("AKs", "AKo", "TT") and the
//! Chen formula, a classic, simple preflop strength score (Bill Chen):
//!
//! 1. highest card: A = 10, K = 8, Q = 7, J = 6, then half the face value
//!    (T = 5 … 2 = 1);
//! 2. pairs: double it, minimum 5 (so 22 = 5, AA = 20);
//! 3. suited: +2;
//! 4. gap between the ranks: 0 → 0, 1 → −1, 2 → −2, 3 → −4, 4+ → −5;
//! 5. +1 for connectors or one-gappers with both cards below a queen;
//! 6. round half up.
//!
//! It is used by the session review for plain-language rules
//! ("you shoved a weak hand"), not as a solver.

use crate::card::{Card, Rank};

/// "AKs", "AKo" or "TT"; higher rank first. `None` unless exactly 2 cards.
pub fn hand_class(cards: &[Card]) -> Option<String> {
    let [a, b] = cards else { return None };
    let (hi, lo) = if a.rank() >= b.rank() { (a, b) } else { (b, a) };
    let mut s = String::with_capacity(3);
    s.push(hi.rank().to_char());
    s.push(lo.rank().to_char());
    if hi.rank() != lo.rank() {
        s.push(if hi.suit() == lo.suit() { 's' } else { 'o' });
    }
    Some(s)
}

fn high_card_points(r: Rank) -> f64 {
    match r {
        Rank::Ace => 10.0,
        Rank::King => 8.0,
        Rank::Queen => 7.0,
        Rank::Jack => 6.0,
        r => (r as u8 + 2) as f64 / 2.0,
    }
}

/// Chen score, from −1 (72o) to 20 (AA).
pub fn chen(cards: &[Card]) -> Option<i32> {
    let [a, b] = cards else { return None };
    let (hi, lo) = if a.rank() >= b.rank() {
        (*a, *b)
    } else {
        (*b, *a)
    };
    let mut score = high_card_points(hi.rank());
    if hi.rank() == lo.rank() {
        score = (score * 2.0).max(5.0);
        return Some(score.ceil() as i32);
    }
    if hi.suit() == lo.suit() {
        score += 2.0;
    }
    let gap = hi.rank() as i32 - lo.rank() as i32 - 1;
    score -= match gap {
        0 => 0.0,
        1 => 1.0,
        2 => 2.0,
        3 => 4.0,
        _ => 5.0,
    };
    if gap <= 1 && hi.rank() < Rank::Queen {
        score += 1.0;
    }
    Some(score.ceil() as i32)
}

/// AA, KK, QQ, AKs, AKo.
pub fn is_premium(cards: &[Card]) -> bool {
    matches!(
        hand_class(cards).as_deref(),
        Some("AA" | "KK" | "QQ" | "AKs" | "AKo")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::parse_cards;

    fn c(s: &str) -> Vec<Card> {
        parse_cards(s).unwrap()
    }

    #[test]
    fn classes() {
        assert_eq!(hand_class(&c("Kd As")).as_deref(), Some("AKo"));
        assert_eq!(hand_class(&c("Ah Kh")).as_deref(), Some("AKs"));
        assert_eq!(hand_class(&c("Td Tc")).as_deref(), Some("TT"));
        assert_eq!(hand_class(&c("Ah")), None);
    }

    #[test]
    fn chen_reference_values() {
        // Values from Bill Chen's published examples.
        assert_eq!(chen(&c("Ah Ad")), Some(20));
        assert_eq!(chen(&c("Ah Kh")), Some(12));
        assert_eq!(chen(&c("2h 2d")), Some(5));
        assert_eq!(chen(&c("Th 9h")), Some(8)); // 5 + 2 (suited) + 1 (connector below Q)
        assert_eq!(chen(&c("7s 2d")), Some(-1));
        assert_eq!(chen(&c("5h 5d")), Some(5));
        assert_eq!(chen(&c("Jh Td")), Some(7)); // 6 + 1 (connector below Q) = 7
    }

    #[test]
    fn premium() {
        assert!(is_premium(&c("Ks Kd")));
        assert!(is_premium(&c("Ad Kc")));
        assert!(!is_premium(&c("Js Jd")));
    }
}
