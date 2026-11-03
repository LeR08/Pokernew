//! Leak finder: the hero's stats compared with target ranges
//! (`leaks/*.toml`, user-editable), per game type.
//!
//! For a percentage stat the 95 % confidence interval is the Wilson score
//! interval of `value` over `sample` opportunities. A stat is a **leak** when
//! the whole interval is outside the target (we are confident), a
//! **possible leak** when only the value is outside.
//!
//! "Impact": for a *decision* stat `a / b` made of two counters, the hero's
//! bb/100 in the hands where he did it (`a = 1`) versus the opportunities
//! where he did not (`b = 1, a = 0`) — what this spot actually earned him.
//! Outcome stats (WTSD, W$SD, WWSF…) are excluded: winning at showdown is
//! profitable by definition, so the comparison would say nothing. Each side
//! needs [`IMPACT_MIN_HANDS`] hands, as bb/100 is very noisy on small samples.

use indexmap::IndexMap;
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};

use crate::defs::{CompiledStat, Text};
use crate::expr::{self, BinOp, Expr};
use crate::query::{self, Filter, Group};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    /// Range for every game type without its own.
    #[serde(default)]
    pub default: Option<Range>,
    #[serde(default)]
    pub cash: Option<Range>,
    #[serde(default)]
    pub mtt: Option<Range>,
    #[serde(default)]
    pub spin: Option<Range>,
    #[serde(default)]
    pub too_low: Option<Text>,
    #[serde(default)]
    pub too_high: Option<Text>,
}

impl Target {
    pub fn range_for(&self, game_type: &str) -> Option<Range> {
        match game_type {
            "cash" => self.cash,
            "mtt" | "sng" => self.mtt,
            "spin" => self.spin,
            _ => None,
        }
        .or(self.default)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TargetsFile {
    #[serde(default)]
    pub target: IndexMap<String, Target>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    /// Value outside the target, but the interval overlaps it.
    Possible,
    /// The whole confidence interval is outside the target.
    Leak,
    /// Not enough hands to say.
    TooFew,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeakRow {
    pub stat: String,
    pub game_type: String,
    pub value: Option<f64>,
    pub sample: i64,
    pub ci_low: Option<f64>,
    pub ci_high: Option<f64>,
    pub target: Range,
    pub status: Status,
    /// "low" or "high" when outside the target.
    pub direction: Option<&'static str>,
    pub too_low: Option<Text>,
    pub too_high: Option<Text>,
    /// bb/100 when doing it / when not doing it (simple ratio stats only).
    pub bb100_when_done: Option<f64>,
    pub bb100_when_not: Option<f64>,
}

/// Wilson score interval, 95 %.
pub fn wilson(p: f64, n: i64) -> (f64, f64) {
    if n <= 0 {
        return (0.0, 1.0);
    }
    let n = n as f64;
    let z = 1.96_f64;
    let denom = 1.0 + z * z / n;
    let centre = (p + z * z / (2.0 * n)) / denom;
    let half = z * ((p * (1.0 - p) / n) + z * z / (4.0 * n * n)).sqrt() / denom;
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

pub const MIN_SAMPLE: i64 = 20;
pub const IMPACT_MIN_HANDS: i64 = 50;
/// Counters that describe an outcome, not a decision.
const OUTCOME_COUNTERS: &[&str] = &["cnt_wtsd", "cnt_wsd", "cnt_wwsf", "cnt_won_hand", "cnt_saw_flop"];

fn ratio_counters(formula: &str) -> Option<(String, String)> {
    match expr::parse(formula).ok()? {
        Expr::Bin(BinOp::Div, a, b) => match (*a, *b) {
            (Expr::Ident(a), Expr::Ident(b))
                if lps_db::counters::COUNTERS.contains(&a.as_str())
                    && lps_db::counters::COUNTERS.contains(&b.as_str())
                    && !OUTCOME_COUNTERS.contains(&a.as_str()) =>
            {
                Some((a, b))
            }
            _ => None,
        },
        _ => None,
    }
}

fn spot_results(conn: &Connection, filter: &Filter, game_type: &str, a: &str, b: &str) -> rusqlite::Result<(Option<f64>, Option<f64>)> {
    let mut f = filter.clone();
    f.game_types = vec![game_type.to_string()];
    let w = f.to_sql();
    // Column names come from the compiled counter list, never from user text.
    let sql = format!(
        "SELECT SUM(CASE WHEN s.{a} = 1 THEN hp.net_won_bb END), SUM(CASE WHEN s.{a} = 1 THEN 1 ELSE 0 END),
                SUM(CASE WHEN s.{b} = 1 AND s.{a} = 0 THEN hp.net_won_bb END), SUM(CASE WHEN s.{b} = 1 AND s.{a} = 0 THEN 1 ELSE 0 END)
         FROM hand_player_stats s JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
         JOIN hands h ON h.id = s.hand_id JOIN players p ON p.id = hp.player_id WHERE {}",
        w.text
    );
    conn.query_row(&sql, params_from_iter(w.params), |r| {
        let per100 = |sum: Option<f64>, n: Option<i64>| match (sum, n) {
            (Some(s), Some(n)) if n >= IMPACT_MIN_HANDS => Some(s * 100.0 / n as f64),
            _ => None,
        };
        Ok((per100(r.get(0)?, r.get(1)?), per100(r.get(2)?, r.get(3)?)))
    })
}

/// Every target stat, per game type played.
pub fn find(conn: &Connection, filter: &Filter, stats: &[CompiledStat], targets: &TargetsFile) -> rusqlite::Result<Vec<LeakRow>> {
    let wanted: Vec<CompiledStat> = stats.iter().filter(|s| targets.target.contains_key(&s.id)).cloned().collect();
    let rows = query::report(conn, filter, &wanted, Group::GameType)?;
    let mut out = Vec::new();
    for row in &rows {
        for (id, target) in &targets.target {
            let Some(range) = target.range_for(&row.key) else { continue };
            let Some(st) = wanted.iter().find(|s| &s.id == id) else { continue };
            let Some(v) = row.stats.iter().find(|s| &s.id == id) else { continue };
            let is_pct = st.def.format.starts_with("pct");
            let (lo, hi) = match (v.value, is_pct) {
                (Some(p), true) => {
                    let (l, h) = wilson(p, v.sample);
                    (Some(l), Some(h))
                }
                (Some(p), false) => (Some(p), Some(p)),
                _ => (None, None),
            };
            let below = |x: f64| range.min.is_some_and(|m| x < m);
            let above = |x: f64| range.max.is_some_and(|m| x > m);
            let (status, direction) = match v.value {
                _ if v.sample < st.def.min_sample.unwrap_or(MIN_SAMPLE) => (Status::TooFew, None),
                None => (Status::TooFew, None),
                Some(x) if below(x) => (if hi.is_some_and(below) { Status::Leak } else { Status::Possible }, Some("low")),
                Some(x) if above(x) => (if lo.is_some_and(above) { Status::Leak } else { Status::Possible }, Some("high")),
                Some(_) => (Status::Ok, None),
            };
            let (done, not) = match ratio_counters(&st.def.formula) {
                Some((a, b)) if st.def.where_.is_none() => spot_results(conn, filter, &row.key, &a, &b)?,
                _ => (None, None),
            };
            out.push(LeakRow {
                stat: id.clone(),
                game_type: row.key.clone(),
                value: v.value,
                sample: v.sample,
                ci_low: lo,
                ci_high: hi,
                target: range,
                status,
                direction,
                too_low: target.too_low.clone(),
                too_high: target.too_high.clone(),
                bb100_when_done: done,
                bb100_when_not: not,
            });
        }
    }
    // Leaks first, then possible ones, then the rest.
    let rank = |s: Status| match s {
        Status::Leak => 0,
        Status::Possible => 1,
        Status::Ok => 2,
        Status::TooFew => 3,
    };
    out.sort_by_key(|r| rank(r.status));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_interval() {
        let (l, h) = wilson(0.5, 100);
        assert!((l - 0.4038).abs() < 1e-3 && (h - 0.5962).abs() < 1e-3);
        let (l, h) = wilson(0.0, 10);
        assert_eq!(l, 0.0);
        assert!(h > 0.25 && h < 0.31);
    }

    #[test]
    fn ratio_detection() {
        assert_eq!(ratio_counters("cnt_3bet / opp_3bet"), Some(("cnt_3bet".into(), "opp_3bet".into())));
        assert_eq!(ratio_counters("net_won_bb * 100 / cnt_hands"), None);
        assert_eq!(ratio_counters("cnt_wsd / cnt_wtsd"), None, "outcome stat");
    }
}
