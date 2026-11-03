//! Table positions, computed from the button and the blinds actually posted.
//!
//! Rooms such as Winamax use a *dead small blind*: when the player who should
//! post the small blind has just busted, nobody posts it. Positions are
//! therefore derived from who posted which blind, not from seat arithmetic:
//!
//! * the big-blind poster is `BB`, the small-blind poster `SB`;
//! * heads-up, the button posts the small blind and is labelled `BTN`;
//! * the other players, in clockwise order from the seat after the big blind
//!   to the button, get positions from the table below, counted from the
//!   button backwards (CO is always the seat right before the button).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Position {
    #[serde(rename = "UTG")]
    Utg,
    #[serde(rename = "UTG1")]
    Utg1,
    #[serde(rename = "UTG2")]
    Utg2,
    #[serde(rename = "MP")]
    Mp,
    #[serde(rename = "LJ")]
    Lj,
    #[serde(rename = "HJ")]
    Hj,
    #[serde(rename = "CO")]
    Co,
    #[serde(rename = "BTN")]
    Btn,
    #[serde(rename = "SB")]
    Sb,
    #[serde(rename = "BB")]
    Bb,
}

impl Position {
    pub fn as_str(self) -> &'static str {
        match self {
            Position::Utg => "UTG",
            Position::Utg1 => "UTG1",
            Position::Utg2 => "UTG2",
            Position::Mp => "MP",
            Position::Lj => "LJ",
            Position::Hj => "HJ",
            Position::Co => "CO",
            Position::Btn => "BTN",
            Position::Sb => "SB",
            Position::Bb => "BB",
        }
    }

    /// Late position (CO, BTN) or SB: the seats a steal attempt comes from.
    pub fn is_steal_position(self) -> bool {
        matches!(self, Position::Co | Position::Btn | Position::Sb)
    }
}

/// Positions of the `k` non-blind, non-button seats, from first to act to CO.
fn middle_positions(k: usize) -> &'static [Position] {
    use Position::*;
    match k {
        0 => &[],
        1 => &[Co],
        2 => &[Hj, Co],
        3 => &[Utg, Hj, Co],
        4 => &[Utg, Lj, Hj, Co],
        5 => &[Utg, Utg1, Lj, Hj, Co],
        6 => &[Utg, Utg1, Utg2, Lj, Hj, Co],
        _ => &[Utg, Utg1, Utg2, Mp, Lj, Hj, Co],
    }
}

/// Assigns positions.
///
/// * `seats`: seat numbers of the players dealt in, any order;
/// * `button`: button seat number (may be empty: dead button);
/// * `sb`, `bb`: seat numbers of the blind posters, if any.
///
/// Returns one position per entry of `seats`, in the same order.
pub fn assign_positions(seats: &[u8], button: u8, sb: Option<u8>, bb: Option<u8>) -> Vec<Option<Position>> {
    let mut clockwise: Vec<u8> = seats.to_vec();
    clockwise.sort_unstable();
    // Rotate so the first seat is the first one after the button.
    let start = clockwise.iter().position(|&s| s > button).unwrap_or(0);
    clockwise.rotate_left(start);
    let n = clockwise.len();

    let mut out: Vec<(u8, Position)> = Vec::with_capacity(n);
    if n == 2 {
        // Heads-up: the button posts the small blind; the other player is BB.
        let bb_seat = bb.or_else(|| clockwise.iter().copied().find(|&s| s != button)).unwrap_or(clockwise[0]);
        for &s in &clockwise {
            out.push((s, if s == bb_seat { Position::Bb } else { Position::Btn }));
        }
    } else {
        let btn_seat = seats.contains(&button).then_some(button);
        // Seats after the big blind (or after the button when nobody posted it).
        let anchor = bb.or(sb).and_then(|b| clockwise.iter().position(|&s| s == b));
        let after_blinds: Vec<u8> = match anchor {
            Some(i) => clockwise[i + 1..].to_vec(),
            None => clockwise.clone(),
        };
        let middle: Vec<u8> = after_blinds
            .iter()
            .copied()
            .filter(|&s| Some(s) != btn_seat && Some(s) != sb && Some(s) != bb)
            .collect();
        let labels = middle_positions(middle.len());
        // With more than 7 middle seats (never in NLHE ≤ 10-max) the extra ones get UTG.
        let offset = middle.len().saturating_sub(labels.len());
        for (i, &s) in middle.iter().enumerate() {
            let p = if i < offset { Position::Utg } else { labels[i - offset] };
            out.push((s, p));
        }
        for &s in &clockwise {
            if Some(s) == bb {
                out.push((s, Position::Bb));
            } else if Some(s) == sb {
                out.push((s, Position::Sb));
            } else if Some(s) == btn_seat {
                out.push((s, Position::Btn));
            }
        }
    }
    seats
        .iter()
        .map(|s| out.iter().find(|(x, _)| x == s).map(|(_, p)| *p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Position::*;
    use super::*;

    fn labels(seats: &[u8], button: u8, sb: Option<u8>, bb: Option<u8>) -> Vec<&'static str> {
        assign_positions(seats, button, sb, bb)
            .into_iter()
            .map(|p| p.map(Position::as_str).unwrap_or("?"))
            .collect()
    }

    #[test]
    fn six_max_full() {
        // Button on 3: SB 4, BB 5, then 6 (UTG), 1 (HJ), 2 (CO).
        assert_eq!(labels(&[1, 2, 3, 4, 5, 6], 3, Some(4), Some(5)), ["HJ", "CO", "BTN", "SB", "BB", "UTG"]);
    }

    #[test]
    fn three_handed_and_heads_up() {
        assert_eq!(labels(&[1, 2, 3], 3, Some(1), Some(2)), ["SB", "BB", "BTN"]);
        assert_eq!(labels(&[2, 5], 5, Some(5), Some(2)), ["BB", "BTN"]);
    }

    #[test]
    fn dead_small_blind() {
        // Real Winamax hand: button seat 5, nobody posts SB, seat 1 posts BB.
        assert_eq!(labels(&[1, 2, 3, 4, 5], 5, None, Some(1)), ["BB", "UTG", "HJ", "CO", "BTN"]);
    }

    #[test]
    fn dead_button() {
        // Button seat 6 is empty; SB 1, BB 2.
        assert_eq!(labels(&[1, 2, 3, 4, 5], 6, Some(1), Some(2)), ["SB", "BB", "UTG", "HJ", "CO"]);
    }

    #[test]
    fn nine_and_ten_max() {
        let nine: Vec<u8> = (1..=9).collect();
        assert_eq!(
            labels(&nine, 9, Some(1), Some(2)),
            ["SB", "BB", "UTG", "UTG1", "UTG2", "LJ", "HJ", "CO", "BTN"]
        );
        let ten: Vec<u8> = (1..=10).collect();
        let p = assign_positions(&ten, 10, Some(1), Some(2));
        assert_eq!(p[2..9].iter().map(|x| x.unwrap()).collect::<Vec<_>>(), [Utg, Utg1, Utg2, Mp, Lj, Hj, Co]);
    }

    #[test]
    fn every_size_from_2_to_10_has_unique_positions() {
        for n in 2..=10u8 {
            let seats: Vec<u8> = (1..=n).collect();
            let p = assign_positions(&seats, n, Some(1), Some(2));
            let mut uniq: Vec<_> = p.iter().map(|x| x.unwrap()).collect();
            uniq.sort();
            uniq.dedup();
            assert_eq!(uniq.len(), n as usize, "n = {n}");
        }
    }
}
