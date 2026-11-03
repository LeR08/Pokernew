//! Per-hand facts derived once at import, so filters are plain SQL.

use crate::card::{Card, Rank, Suit};
use crate::hand::{ActionKind, Hand, Street};

/// Flop texture flags (bitmask stored in `hands.flop_texture`).
pub mod texture {
    pub const PAIRED: i64 = 1;
    pub const MONOTONE: i64 = 2;
    pub const TWO_TONE: i64 = 4;
    pub const RAINBOW: i64 = 8;
    /// Three distinct ranks within a span of 4 (straight possible), ace counts low too.
    pub const CONNECTED: i64 = 16;
    /// Highest card is an ace or a king.
    pub const HIGH: i64 = 32;
    /// Every card is 9 or lower.
    pub const LOW: i64 = 64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Derived {
    /// Raises before the flop (0 = limped or walk, 1 = single-raised, 2 = 3-bet, 3+ = 4-bet+).
    pub pf_raises: i64,
    /// Players still in when the flop is dealt (0 when there is no flop).
    pub players_flop: i64,
    /// [`texture`] flags of the flop, 0 without flop.
    pub flop_texture: i64,
}

pub fn flop_texture(flop: &[Card]) -> i64 {
    let [a, b, c] = flop else { return 0 };
    let cards = [*a, *b, *c];
    let mut t = 0;
    let ranks: Vec<Rank> = cards.iter().map(|x| x.rank()).collect();
    let mut distinct = ranks.clone();
    distinct.sort();
    distinct.dedup();
    if distinct.len() < 3 {
        t |= texture::PAIRED;
    }
    let mut suits: Vec<Suit> = cards.iter().map(|x| x.suit()).collect();
    suits.sort();
    suits.dedup();
    t |= match suits.len() {
        1 => texture::MONOTONE,
        2 => texture::TWO_TONE,
        _ => texture::RAINBOW,
    };
    if distinct.len() == 3 {
        let r: Vec<i64> = distinct.iter().map(|x| *x as i64).collect();
        let span = r[2] - r[0];
        // Ace low: A23, A24, A35, A45…
        let low_span = if distinct[2] == Rank::Ace {
            r[1] + 1
        } else {
            i64::MAX
        };
        if span <= 4 || low_span <= 4 {
            t |= texture::CONNECTED;
        }
    }
    let top = *ranks.iter().max().expect("3 cards");
    if top >= Rank::King {
        t |= texture::HIGH;
    }
    if top <= Rank::Nine {
        t |= texture::LOW;
    }
    t
}

pub fn derive(hand: &Hand) -> Derived {
    let pf_raises = hand
        .actions
        .iter()
        .filter(|a| a.street == Street::Preflop && a.kind == ActionKind::Raise)
        .count() as i64;
    let players_flop = if hand.board.len() >= 3 {
        let folded: std::collections::HashSet<usize> = hand
            .actions
            .iter()
            .filter(|a| a.street == Street::Preflop && a.kind == ActionKind::Fold)
            .map(|a| a.player)
            .collect();
        (hand.players.len() - folded.len()) as i64
    } else {
        0
    };
    Derived {
        pf_raises,
        players_flop,
        flop_texture: flop_texture(hand.board.get(..3).unwrap_or(&[])),
    }
}

#[cfg(test)]
mod tests {
    use super::texture::*;
    use super::*;
    use crate::card::parse_cards;

    fn tex(s: &str) -> i64 {
        flop_texture(&parse_cards(s).unwrap())
    }

    #[test]
    fn textures() {
        assert_eq!(tex("Ac 4c 8d"), TWO_TONE | HIGH);
        assert_eq!(tex("8s 3s 8c"), PAIRED | TWO_TONE | LOW);
        assert_eq!(tex("Tc Qh Kc"), TWO_TONE | CONNECTED | HIGH);
        assert_eq!(tex("6c 7c 9c"), MONOTONE | CONNECTED | LOW);
        assert_eq!(tex("Ah 2d 4s"), RAINBOW | CONNECTED | HIGH);
        assert_eq!(tex("2c 7h Kd"), RAINBOW | HIGH);
        assert_eq!(flop_texture(&[]), 0);
    }
}
