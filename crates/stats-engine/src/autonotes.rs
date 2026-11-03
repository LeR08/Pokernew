//! Auto-notes: rules over a player's stats, written in the stat expression
//! language, re-evaluated after imports (`notes/*.toml`, user-editable).
//!
//! ```toml
//! [rule.overfold_3bet]
//! text = { fr = "Se couche trop face aux 3-bets ({fold_to_3bet})", en = "Over-folds to 3-bets ({fold_to_3bet})" }
//! when = "fold_to_3bet >= 0.70"
//! min_sample = 30
//! ```
//!
//! Identifiers in `when` are **stat ids**; a rule applies to a player only
//! when every stat it uses has at least `min_sample` opportunities.
//! `{stat_id}` in the text is replaced by the formatted value.

use std::collections::HashMap;

use indexmap::IndexMap;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::defs::{CompiledStat, Text};
use crate::expr::{self, BinOp, Expr};
use crate::query::{self, Filter, Group};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub text: Text,
    pub when: String,
    #[serde(default = "default_min")]
    pub min_sample: i64,
}

fn default_min() -> i64 {
    30
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RulesFile {
    #[serde(default)]
    pub rule: IndexMap<String, Rule>,
}

fn idents(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Ident(n) => out.push(n.clone()),
        Expr::Neg(x) | Expr::Not(x) => idents(x, out),
        Expr::Bin(_, a, b) => {
            idents(a, out);
            idents(b, out);
        }
        Expr::Call(_, args) => args.iter().for_each(|a| idents(a, out)),
        Expr::Num(_) | Expr::Str(_) => {}
    }
}

/// Evaluates an expression over numbers (booleans are 0 / 1). None = unknown.
pub fn eval(e: &Expr, vars: &HashMap<String, f64>) -> Option<f64> {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    Some(match e {
        Expr::Num(n) => *n,
        Expr::Str(_) => return None,
        Expr::Ident(n) => *vars.get(n)?,
        Expr::Neg(x) => -eval(x, vars)?,
        Expr::Not(x) => b(eval(x, vars)? == 0.0),
        Expr::Bin(op, l, r) => {
            let (l, r) = (eval(l, vars)?, eval(r, vars)?);
            match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => {
                    if r == 0.0 {
                        return None;
                    }
                    l / r
                }
                BinOp::Eq => b(l == r),
                BinOp::Ne => b(l != r),
                BinOp::Lt => b(l < r),
                BinOp::Le => b(l <= r),
                BinOp::Gt => b(l > r),
                BinOp::Ge => b(l >= r),
                BinOp::And => b(l != 0.0 && r != 0.0),
                BinOp::Or => b(l != 0.0 || r != 0.0),
            }
        }
        Expr::Call(name, args) if name == "if" && args.len() == 3 => {
            if eval(&args[0], vars)? != 0.0 {
                eval(&args[1], vars)?
            } else {
                eval(&args[2], vars)?
            }
        }
        Expr::Call(..) => return None,
    })
}

fn format_value(v: f64, format: &str) -> String {
    if format.starts_with("pct") {
        format!("{:.0} %", v * 100.0)
    } else {
        format!("{v:.1}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AutoNote {
    pub player: String,
    pub rule: String,
    pub text: String,
}

/// Evaluates every rule on every opponent of `room`.
pub fn evaluate(
    conn: &Connection,
    room: &str,
    stats: &[CompiledStat],
    rules: &RulesFile,
    lang: &str,
) -> rusqlite::Result<Vec<AutoNote>> {
    let mut needed: Vec<String> = Vec::new();
    let mut parsed = Vec::new();
    for (id, r) in &rules.rule {
        let Ok(e) = expr::parse(&r.when) else {
            continue;
        };
        let mut ids = Vec::new();
        idents(&e, &mut ids);
        if ids.iter().all(|i| stats.iter().any(|s| &s.id == i)) {
            needed.extend(ids.iter().cloned());
            parsed.push((id, r, e, ids));
        }
    }
    needed.sort();
    needed.dedup();
    let wanted: Vec<CompiledStat> = stats
        .iter()
        .filter(|s| needed.contains(&s.id))
        .cloned()
        .collect();
    let min = rules.rule.values().map(|r| r.min_sample).min().unwrap_or(0);
    let filter = Filter {
        who: Some("opponents".into()),
        rooms: vec![room.into()],
        ..Default::default()
    };
    let rows = query::report_limited(conn, &filter, &wanted, Group::Player, min.max(1), i64::MAX)?;
    let mut out = Vec::new();
    for row in rows {
        for (id, rule, e, ids) in &parsed {
            let mut vars = HashMap::new();
            let mut enough = true;
            for i in ids {
                let v = row.stats.iter().find(|s| &s.id == i);
                match v {
                    Some(sv) if sv.sample >= rule.min_sample && sv.value.is_some() => {
                        vars.insert(i.clone(), sv.value.unwrap_or(0.0));
                    }
                    _ => enough = false,
                }
            }
            if !enough || eval(e, &vars).is_none_or(|x| x == 0.0) {
                continue;
            }
            let mut text = match &rule.text {
                Text::Plain(t) => t.clone(),
                Text::I18n { fr, en } => {
                    if lang == "fr" {
                        fr.clone()
                    } else {
                        en.clone()
                    }
                }
            };
            for i in ids {
                let fmt = wanted
                    .iter()
                    .find(|s| &s.id == i)
                    .map(|s| s.def.format.as_str())
                    .unwrap_or("pct0");
                text = text.replace(&format!("{{{i}}}"), &format_value(vars[i], fmt));
            }
            out.push(AutoNote {
                player: row.key.clone(),
                rule: (*id).clone(),
                text,
            });
        }
    }
    Ok(out)
}

/// Replaces all auto-notes of `room` by `notes`. Returns how many were written.
pub fn apply(
    conn: &mut Connection,
    room: &str,
    notes: &[AutoNote],
    now: i64,
) -> rusqlite::Result<usize> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM notes WHERE auto = 1 AND player_id IN (SELECT id FROM players WHERE room = ?)",
        [room],
    )?;
    {
        let mut st = tx.prepare(
            "INSERT INTO notes (player_id, text, auto, created_at, rule)
             SELECT id, ?, 1, ?, ? FROM players WHERE room = ? AND name = ?",
        )?;
        for n in notes {
            st.execute(params![n.text, now, n.rule, room, n.player])?;
        }
    }
    tx.commit()?;
    Ok(notes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluator() {
        let mut v = HashMap::new();
        v.insert("fold_to_3bet".to_string(), 0.75);
        v.insert("vpip".to_string(), 0.42);
        let e = |s: &str| eval(&expr::parse(s).unwrap(), &v);
        assert_eq!(e("fold_to_3bet >= 0.70"), Some(1.0));
        assert_eq!(e("vpip > 0.4 and fold_to_3bet < 0.5"), Some(0.0));
        assert_eq!(e("not (vpip < 0.3)"), Some(1.0));
        assert_eq!(e("missing > 1"), None);
        assert_eq!(e("vpip / 0"), None);
        assert_eq!(e("if(vpip > 0.4, 2, 3)"), Some(2.0));
    }
}
