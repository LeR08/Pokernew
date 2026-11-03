//! Compiles stat expressions to SQL with bound parameters.
//!
//! A stat is a formula over **aggregated** values: every counter in the
//! formula becomes `SUM(counter)` over the selected hands, so
//! `cnt_vpip / cnt_hands` becomes
//! `SUM(s.cnt_vpip) * 1.0 / NULLIF(SUM(s.cnt_hands), 0)`.
//! An optional `where` condition is evaluated **per hand** and restricts the
//! sums: `SUM(CASE WHEN <where> THEN s.cnt_3bet ELSE 0 END)`.
//! Strings are always bound parameters, never pasted into the SQL.

use rusqlite::types::Value;

use crate::expr::{BinOp, Expr};

/// SQL text plus its parameters, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sql {
    pub text: String,
    pub params: Vec<Value>,
}

impl Sql {
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            params: vec![],
        }
    }
    fn push(&mut self, other: Sql) {
        self.text.push_str(&other.text);
        self.params.extend(other.params);
    }
    fn lit(&mut self, s: &str) {
        self.text.push_str(s);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
#[error("{0}")]
pub struct CompileError(pub String);

/// Per-hand fields usable in `where` conditions.
pub const ROW_FIELDS: &[(&str, &str, &str)] = &[
    (
        "position",
        "hp.position",
        "UTG, UTG1, UTG2, MP, LJ, HJ, CO, BTN, SB, BB",
    ),
    ("game_type", "h.game_type", "'cash', 'mtt', 'sng', 'spin'"),
    ("room", "h.room", "'winamax'"),
    (
        "big_blind",
        "h.big_blind",
        "big blind (cents in cash, chips in tournaments)",
    ),
    ("ante", "h.ante", "ante"),
    ("level", "h.level", "blind level (tournaments)"),
    ("max_seats", "h.max_seats", "table size: 2, 3, 6, 9…"),
    ("players", "h.n_players", "players dealt in"),
    ("stack_bb", "hp.stack_bb", "starting stack in big blinds"),
    ("showdown", "h.showdown", "1 if the hand reached showdown"),
    (
        "went_to_showdown",
        "hp.went_to_showdown",
        "1 if the player reached showdown",
    ),
    (
        "net_won_bb",
        "hp.net_won_bb",
        "result of the hand in big blinds",
    ),
];

/// Per-hand numbers that can also be summed in formulas.
pub const SUM_FIELDS: &[(&str, &str)] =
    &[("net_won_bb", "hp.net_won_bb"), ("net_won", "hp.net_won")];

fn counter_col(name: &str) -> Option<String> {
    lps_db::counters::COUNTERS
        .contains(&name)
        .then(|| format!("s.{name}"))
}

fn row_col(name: &str) -> Option<String> {
    ROW_FIELDS
        .iter()
        .find(|(n, _, _)| *n == name)
        .map(|(_, c, _)| (*c).to_string())
        .or_else(|| counter_col(name))
}

fn sum_col(name: &str) -> Option<String> {
    counter_col(name).or_else(|| {
        SUM_FIELDS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| (*c).to_string())
    })
}

/// Compiles a per-hand condition.
pub fn condition(e: &Expr) -> Result<Sql, CompileError> {
    let mut out = Sql::default();
    match e {
        Expr::Num(n) => {
            out.params.push(Value::Real(*n));
            out.lit("?");
        }
        Expr::Str(s) => {
            out.params.push(Value::Text(s.clone()));
            out.lit("?");
        }
        Expr::Ident(name) => {
            let col =
                row_col(name).ok_or_else(|| CompileError(format!("unknown field '{name}'")))?;
            out.lit(&col);
        }
        Expr::Neg(x) => {
            out.lit("(-");
            out.push(condition(x)?);
            out.lit(")");
        }
        Expr::Not(x) => {
            out.lit("(NOT ");
            out.push(condition(x)?);
            out.lit(")");
        }
        Expr::Bin(op, l, r) => {
            out.lit("(");
            out.push(condition(l)?);
            out.lit(&format!(" {} ", op.sql()));
            out.push(condition(r)?);
            out.lit(")");
        }
        Expr::Call(name, args) => out.push(call(name, args, &condition)?),
    }
    Ok(out)
}

fn call(
    name: &str,
    args: &[Expr],
    f: &dyn Fn(&Expr) -> Result<Sql, CompileError>,
) -> Result<Sql, CompileError> {
    match (name, args) {
        ("if", [c, a, b]) => {
            let mut out = Sql::raw("(CASE WHEN ");
            out.push(condition_any(c, f)?);
            out.lit(" THEN ");
            out.push(f(a)?);
            out.lit(" ELSE ");
            out.push(f(b)?);
            out.lit(" END)");
            Ok(out)
        }
        ("if", _) => Err(CompileError(
            "if() takes 3 arguments: if(condition, then, else)".into(),
        )),
        _ => Err(CompileError(format!(
            "unknown function '{name}' (only if() exists)"
        ))),
    }
}

/// Inside `if()` the condition uses the same context as the surrounding formula.
fn condition_any(
    c: &Expr,
    f: &dyn Fn(&Expr) -> Result<Sql, CompileError>,
) -> Result<Sql, CompileError> {
    f(c)
}

/// Compiles an aggregated formula; `filter` is the stat's `where` condition.
pub fn formula(e: &Expr, filter: Option<&Sql>) -> Result<Sql, CompileError> {
    let rec = |x: &Expr| formula(x, filter);
    let mut out = Sql::default();
    match e {
        Expr::Num(n) => {
            out.params.push(Value::Real(*n));
            out.lit("?");
        }
        Expr::Str(_) => {
            return Err(CompileError(
                "text is only allowed in conditions (where)".into(),
            ))
        }
        Expr::Ident(name) => {
            let col = match sum_col(name) {
                Some(c) => c,
                None if row_col(name).is_some() => {
                    return Err(CompileError(format!(
                        "'{name}' is a per-hand field: use it in 'where', not in the formula"
                    )))
                }
                None => return Err(CompileError(format!("unknown counter '{name}'"))),
            };
            match filter {
                Some(w) => {
                    out.lit("SUM(CASE WHEN ");
                    out.push(w.clone());
                    out.lit(&format!(" THEN {col} ELSE 0 END)"));
                }
                None => out.lit(&format!("SUM({col})")),
            }
        }
        Expr::Neg(x) => {
            out.lit("(-");
            out.push(rec(x)?);
            out.lit(")");
        }
        Expr::Not(x) => {
            out.lit("(NOT ");
            out.push(rec(x)?);
            out.lit(")");
        }
        Expr::Bin(BinOp::Div, l, r) => {
            out.lit("(");
            out.push(rec(l)?);
            out.lit(" * 1.0 / NULLIF(");
            out.push(rec(r)?);
            out.lit(", 0))");
        }
        Expr::Bin(op, l, r) => {
            out.lit("(");
            out.push(rec(l)?);
            out.lit(&format!(" {} ", op.sql()));
            out.push(rec(r)?);
            out.lit(")");
        }
        Expr::Call(name, args) => out.push(call(name, args, &rec)?),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::parse;

    #[test]
    fn simple_ratio() {
        let s = formula(&parse("cnt_vpip / cnt_hands").unwrap(), None).unwrap();
        assert_eq!(
            s.text,
            "(SUM(s.cnt_vpip) * 1.0 / NULLIF(SUM(s.cnt_hands), 0))"
        );
        assert!(s.params.is_empty());
    }

    #[test]
    fn where_restricts_sums_and_binds_strings() {
        let w = condition(&parse("position = 'BB' and stack_bb > 20").unwrap()).unwrap();
        assert_eq!(w.text, "((hp.position = ?) AND (hp.stack_bb > ?))");
        let s = formula(&parse("cnt_3bet / opp_3bet").unwrap(), Some(&w)).unwrap();
        assert_eq!(s.text.matches("CASE WHEN").count(), 2);
        assert_eq!(s.params.len(), 4);
        assert_eq!(s.params[0], Value::Text("BB".into()));
    }

    #[test]
    fn injection_is_impossible() {
        let w = condition(
            &parse("position = 'BB''; DROP TABLE hands; --'").unwrap_or(Expr::Str("x".into())),
        );
        // Either a syntax error or a bound string: never raw SQL.
        if let Ok(w) = w {
            assert!(!w.text.contains("DROP"));
        }
    }

    #[test]
    fn helpful_errors() {
        assert!(formula(&parse("position").unwrap(), None)
            .unwrap_err()
            .0
            .contains("where"));
        assert!(formula(&parse("cnt_nope").unwrap(), None)
            .unwrap_err()
            .0
            .contains("unknown counter"));
        assert!(condition(&parse("nope = 1").unwrap())
            .unwrap_err()
            .0
            .contains("unknown field"));
        assert!(formula(&parse("max(1)").unwrap(), None).is_err());
        assert!(formula(&parse("'a'").unwrap(), None).is_err());
    }
}
