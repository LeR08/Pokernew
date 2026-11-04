//! Grades. The EV lost by a choice is measured against the best action for the
//! hand, in % of the pot at the decision. Thresholds are the user's (settings).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Thresholds {
    /// Losses up to these % of the pot are: correct, inaccuracy, mistake; above: blunder.
    pub correct: f64,
    pub inaccuracy: f64,
    pub mistake: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            correct: 1.0,
            inaccuracy: 3.0,
            mistake: 8.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Grade {
    Best,
    Correct,
    Inaccuracy,
    Mistake,
    Blunder,
}

/// `loss_pct`: EV lost in % of the pot; `best`: the choice was the highest-EV action.
pub fn grade(loss_pct: f64, best: bool, t: &Thresholds) -> Grade {
    if best || loss_pct <= 0.05 {
        Grade::Best
    } else if loss_pct <= t.correct {
        Grade::Correct
    } else if loss_pct <= t.inaccuracy {
        Grade::Inaccuracy
    } else if loss_pct <= t.mistake {
        Grade::Mistake
    } else {
        Grade::Blunder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grades() {
        let t = Thresholds::default();
        assert_eq!(grade(0.0, true, &t), Grade::Best);
        assert_eq!(grade(0.5, false, &t), Grade::Correct);
        assert_eq!(grade(2.0, false, &t), Grade::Inaccuracy);
        assert_eq!(grade(5.0, false, &t), Grade::Mistake);
        assert_eq!(grade(20.0, false, &t), Grade::Blunder);
    }
}
