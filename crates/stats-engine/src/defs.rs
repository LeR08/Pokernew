//! Stat definitions (`stats/*.toml`), as the user writes them.
//!
//! ```toml
//! [stat.three_bet_bb]
//! name = "3B BB"
//! formula = "cnt_3bet / opp_3bet"
//! where = "position = 'BB'"
//! format = "pct1"
//! min_sample = 20
//! description = { fr = "…", en = "…" }
//! colour_ranges = [{ max = 0.05, colour = "blue" }, { min = 0.12, colour = "red" }]
//! ```

use serde::{Deserialize, Serialize};

use crate::compile::{self, CompileError, Sql};
use crate::expr::{self, BinOp, Expr, SyntaxError};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Plain(String),
    I18n { fr: String, en: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColourRange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    pub colour: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatDef {
    pub name: String,
    pub formula: String,
    #[serde(rename = "where", default, skip_serializing_if = "Option::is_none")]
    pub where_: Option<String>,
    /// pct0, pct1, dec1, dec2, int, bb100
    #[serde(default = "default_format")]
    pub format: String,
    /// Sample size formula; default = the formula's denominator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_sample: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<Text>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colour_ranges: Vec<ColourRange>,
}

fn default_format() -> String {
    "pct1".into()
}

/// A stats file: `[stat.<id>]` tables, in file order (the order stats are shown).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsFile {
    #[serde(default)]
    pub stat: indexmap::IndexMap<String, StatDef>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatError {
    #[error("formula: {0}")]
    Formula(SyntaxError),
    #[error("where: {0}")]
    Where(SyntaxError),
    #[error("sample: {0}")]
    Sample(SyntaxError),
    #[error("{0}")]
    Compile(CompileError),
}

/// A stat ready to query.
#[derive(Debug, Clone)]
pub struct CompiledStat {
    pub id: String,
    pub def: StatDef,
    pub value: Sql,
    pub sample: Sql,
}

pub fn compile(id: &str, def: &StatDef) -> Result<CompiledStat, StatError> {
    let f = expr::parse(&def.formula).map_err(StatError::Formula)?;
    let w = match &def.where_ {
        Some(w) if !w.trim().is_empty() => {
            Some(compile::condition(&expr::parse(w).map_err(StatError::Where)?).map_err(StatError::Compile)?)
        }
        _ => None,
    };
    let sample_expr = match &def.sample {
        Some(s) => expr::parse(s).map_err(StatError::Sample)?,
        None => match &f {
            Expr::Bin(BinOp::Div, _, den) => (**den).clone(),
            _ => Expr::Ident("cnt_hands".into()),
        },
    };
    Ok(CompiledStat {
        id: id.to_string(),
        def: def.clone(),
        value: compile::formula(&f, w.as_ref()).map_err(StatError::Compile)?,
        sample: compile::formula(&sample_expr, w.as_ref()).map_err(StatError::Compile)?,
    })
}
