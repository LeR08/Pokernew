//! Weighted ranges and the standard text notation.
//!
//! A range gives every one of the 1 326 combos a weight in `0..=1`.
//!
//! Notation (tokens separated by commas or spaces; a later token overrides an
//! earlier one for the same combos):
//! * `AA`, `AKs`, `AKo`, `AK` (= suited + offsuit);
//! * `TT+` (TT up to AA), `ATs+` / `ATo+` / `AT+` (kicker up to one below the top card);
//! * `99-66`, `A5s-A2s`, `KQo-K9o` (same first card);
//! * `AhKh` (one combo);
//! * `random` or `any` (all 1 326 combos);
//! * `15%` (the top 15 % of hands by preflop equity, see [`crate::ranking`]);
//! * a weight after a colon: `KQo:50%`, `KQo:50` (percent) or `KQo:0.5` (fraction:
//!   values ≤ 1 are fractions).
//!
//! The same grammar is implemented by the front end (`src/lib/range.ts`); both
//! are tested against `data/range_cases.json`.

use lps_evaluator::{card, parse_card, Card, Hand, RANKS};

use crate::combo::{all_combos, Combo, NUM_COMBOS};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RangeError {
    #[error("invalid range token '{0}'")]
    BadToken(String),
    #[error("invalid weight in '{0}'")]
    BadWeight(String),
}

#[derive(Clone, PartialEq)]
pub struct Range {
    pub weights: Vec<f32>,
}

impl std::fmt::Debug for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Range({} combos)", self.combo_count())
    }
}

impl Default for Range {
    fn default() -> Self {
        Range {
            weights: vec![0.0; NUM_COMBOS],
        }
    }
}

impl Range {
    pub fn parse(text: &str) -> Result<Range, RangeError> {
        let mut r = Range::default();
        for tok in text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|t| !t.is_empty())
        {
            let (body, w) = match tok.split_once(':') {
                Some((b, w)) => (
                    b,
                    parse_weight(w).ok_or_else(|| RangeError::BadWeight(tok.to_string()))?,
                ),
                None => (tok, 1.0),
            };
            let combos = expand(body).ok_or_else(|| RangeError::BadToken(tok.to_string()))?;
            for c in combos {
                r.weights[c.index()] = w;
            }
        }
        Ok(r)
    }

    pub fn from_weights(weights: Vec<f32>) -> Range {
        assert_eq!(weights.len(), NUM_COMBOS);
        Range { weights }
    }

    pub fn single(c: Combo) -> Range {
        let mut r = Range::default();
        r.weights[c.index()] = 1.0;
        r
    }

    /// Sum of weights.
    pub fn combo_count(&self) -> f64 {
        self.weights.iter().map(|&w| w as f64).sum()
    }

    /// Sum of weights of the combos that do not touch `dead`.
    pub fn live_count(&self, dead: Hand) -> f64 {
        self.live(dead).map(|(_, w)| w).sum()
    }

    /// Combos with a weight > 0 that do not touch `dead`.
    pub fn live(&self, dead: Hand) -> impl Iterator<Item = (Combo, f64)> + '_ {
        all_combos()
            .iter()
            .zip(&self.weights)
            .filter(move |(c, &w)| w > 0.0 && c.mask().0 & dead.0 == 0)
            .map(|(c, &w)| (*c, w as f64))
    }
}

fn parse_weight(s: &str) -> Option<f32> {
    let s = s.trim();
    let (num, pct) = match s.strip_suffix('%') {
        Some(n) => (n, true),
        None => (s, false),
    };
    let v: f32 = num.trim().parse().ok()?;
    let w = if pct || v > 1.0 { v / 100.0 } else { v };
    (0.0..=1.0).contains(&w).then_some(w)
}

fn rank(c: char) -> Option<u8> {
    RANKS
        .iter()
        .position(|&r| r as char == c.to_ascii_uppercase())
        .map(|i| i as u8)
}

/// Combos of two ranks; `suited`: Some(true) suited only, Some(false) offsuit only, None both.
fn combos_of(r1: u8, r2: u8, suited: Option<bool>) -> Vec<Combo> {
    let mut out = Vec::new();
    for s1 in 0..4 {
        for s2 in 0..4 {
            let a = card(r1, s1);
            let b = card(r2, s2);
            if a == b {
                continue;
            }
            if r1 == r2 && s1 >= s2 {
                continue; // each pair combo once
            }
            if let Some(want) = suited {
                if (s1 == s2) != want {
                    continue;
                }
            }
            out.push(Combo::new(a, b));
        }
    }
    out
}

/// Parses a hand class like `AKs`, `AK`, `TT` into (high rank, low rank, suitedness).
fn class(s: &str) -> Option<(u8, u8, Option<bool>)> {
    let ch: Vec<char> = s.chars().collect();
    if ch.len() < 2 || ch.len() > 3 {
        return None;
    }
    let (a, b) = (rank(ch[0])?, rank(ch[1])?);
    let suited = match ch.get(2) {
        None => None,
        Some('s') | Some('S') if a != b => Some(true),
        Some('o') | Some('O') if a != b => Some(false),
        _ => return None,
    };
    Some((a.max(b), a.min(b), suited))
}

fn expand(tok: &str) -> Option<Vec<Combo>> {
    let lower = tok.to_ascii_lowercase();
    if lower == "random" || lower == "any" {
        return Some(all_combos().to_vec());
    }
    if let Some(p) = tok.strip_suffix('%') {
        let pct: f64 = p.parse().ok()?;
        if !(0.0..=100.0).contains(&pct) {
            return None;
        }
        return Some(crate::ranking::top_percent(pct));
    }
    // One combo: "AhKh".
    if tok.len() == 4 {
        if let (Some(a), Some(b)) = (parse_card(&tok[..2]), parse_card(&tok[2..])) {
            if a != b && tok.chars().nth(1).is_some_and(|c| c.is_ascii_lowercase()) {
                return Some(vec![Combo::new(a, b)]);
            }
        }
    }
    if let Some(base) = tok.strip_suffix('+') {
        let (hi, lo, suited) = class(base)?;
        let mut out = Vec::new();
        if hi == lo {
            for r in hi..13 {
                out.extend(combos_of(r, r, None));
            }
        } else {
            for k in lo..hi {
                out.extend(combos_of(hi, k, suited));
            }
        }
        return Some(out);
    }
    if let Some((x, y)) = tok.split_once('-') {
        let (h1, l1, s1) = class(x)?;
        let (h2, l2, s2) = class(y)?;
        if s1 != s2 {
            return None;
        }
        let mut out = Vec::new();
        if h1 == l1 && h2 == l2 {
            for r in h1.min(h2)..=h1.max(h2) {
                out.extend(combos_of(r, r, None));
            }
        } else if h1 == h2 && h1 != l1 && h2 != l2 {
            for k in l1.min(l2)..=l1.max(l2) {
                out.extend(combos_of(h1, k, s1));
            }
        } else {
            return None;
        }
        return Some(out);
    }
    let (hi, lo, suited) = class(tok)?;
    Some(combos_of(hi, lo, suited))
}

/// Cards from a board/dead string, as a mask (`None` on a bad card).
pub fn cards_mask(cards: &[Card]) -> Hand {
    Hand::from_cards(cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> f64 {
        Range::parse(s).unwrap().combo_count()
    }

    #[test]
    fn notation() {
        assert_eq!(n("AA"), 6.0);
        assert_eq!(n("AKs"), 4.0);
        assert_eq!(n("AKo"), 12.0);
        assert_eq!(n("AK"), 16.0);
        assert_eq!(n("TT+"), 30.0);
        assert_eq!(n("ATs+"), 16.0);
        assert_eq!(n("A5s-A2s"), 16.0);
        assert_eq!(n("99-66"), 24.0);
        assert_eq!(n("66-99"), 24.0);
        assert_eq!(n("AhKh"), 1.0);
        assert_eq!(n("random"), 1326.0);
        assert_eq!(n("KQo:50%"), 6.0);
        assert_eq!(n("KQo:0.5"), 6.0);
        assert_eq!(n("KQo:50"), 6.0);
        assert_eq!(n("AA, KK QQ"), 18.0);
        assert_eq!(n("AK, AKo:0"), 4.0);
        assert_eq!(n(""), 0.0);
        assert!(Range::parse("AKx").is_err());
        assert!(Range::parse("AA:150").is_err());
        assert!(Range::parse("AKs-KQs").is_err());
        assert!(Range::parse("AA+-KK").is_err());
    }

    #[test]
    fn dead_cards() {
        let r = Range::parse("AK").unwrap();
        let dead = Hand::from_cards(&[parse_card("Ah").unwrap()]);
        assert_eq!(r.live_count(dead), 12.0);
    }

    #[test]
    fn shared_cases() {
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../data/range_cases.json")).unwrap();
        for c in cases {
            let text = c["text"].as_str().unwrap();
            match c.get("combos").and_then(|v| v.as_f64()) {
                Some(want) => assert!(
                    (n(text) - want).abs() < 1e-4,
                    "{text}: {} != {want}",
                    n(text)
                ),
                None => assert!(Range::parse(text).is_err(), "{text} should fail"),
            }
        }
    }
}
