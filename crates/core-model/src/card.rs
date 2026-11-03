//! Cards. A card is one byte: `rank * 4 + suit`, ranks 0 (deuce) to 12 (ace).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    pub fn to_char(self) -> char {
        ['c', 'd', 'h', 's'][self as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub const ALL: [Rank; 13] = [
        Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight,
        Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace,
    ];
    pub fn to_char(self) -> char {
        b"23456789TJQKA"[self as usize] as char
    }
    pub fn from_char(c: char) -> Option<Rank> {
        let i = "23456789TJQKA".find(c.to_ascii_uppercase())?;
        Some(Rank::ALL[i])
    }
}

/// A playing card.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Card(u8);

impl Card {
    pub fn new(rank: Rank, suit: Suit) -> Self {
        Card(rank as u8 * 4 + suit as u8)
    }
    pub fn rank(self) -> Rank {
        Rank::ALL[(self.0 / 4) as usize]
    }
    pub fn suit(self) -> Suit {
        Suit::ALL[(self.0 % 4) as usize]
    }
    /// 0..52, stable across versions (used as a database/encoding index).
    pub fn index(self) -> u8 {
        self.0
    }
    pub fn from_index(i: u8) -> Option<Self> {
        (i < 52).then_some(Card(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid card '{0}'")]
pub struct CardParseError(pub String);

impl FromStr for Card {
    type Err = CardParseError;
    /// Parses "Ah", "td", "Ts" (rank then suit).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut it = s.chars();
        let (Some(r), Some(su), None) = (it.next(), it.next(), it.next()) else {
            return Err(CardParseError(s.into()));
        };
        let rank = Rank::from_char(r).ok_or_else(|| CardParseError(s.into()))?;
        let suit = match su.to_ascii_lowercase() {
            'c' => Suit::Clubs,
            'd' => Suit::Diamonds,
            'h' => Suit::Hearts,
            's' => Suit::Spades,
            _ => return Err(CardParseError(s.into())),
        };
        Ok(Card::new(rank, suit))
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.rank().to_char(), self.suit().to_char())
    }
}

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Serialize for Card {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Parses space-separated cards: "Ah Kd 2c".
pub fn parse_cards(s: &str) -> Result<Vec<Card>, CardParseError> {
    s.split_whitespace().map(str::parse).collect()
}

/// Formats cards as "AhKd2c" (compact, used in the database).
pub fn cards_to_string(cards: &[Card]) -> String {
    cards.iter().map(ToString::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_all_52() {
        for i in 0..52 {
            let c = Card::from_index(i).unwrap();
            assert_eq!(c.to_string().parse::<Card>().unwrap(), c);
        }
        assert!(Card::from_index(52).is_none());
    }

    #[test]
    fn parses_and_rejects() {
        let c: Card = "Th".parse().unwrap();
        assert_eq!((c.rank(), c.suit()), (Rank::Ten, Suit::Hearts));
        assert!("1h".parse::<Card>().is_err());
        assert!("Ahh".parse::<Card>().is_err());
        assert_eq!(cards_to_string(&parse_cards("Ac 4c 8d").unwrap()), "Ac4c8d");
    }
}
