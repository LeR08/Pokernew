//! The 13×13 starting-hand grid: the hero's results by hand class.
//! Row/column order is A K Q … 2; above the diagonal = suited, below = offsuit.

use std::collections::HashMap;

use lps_core_model::{parse_cards, preflop::hand_class};
use rusqlite::{params_from_iter, Connection};
use serde::Serialize;

use crate::query::Filter;

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub class: String,
    pub hands: i64,
    pub net_bb: f64,
    pub vpip: i64,
    pub pfr: i64,
}

const RANKS: &str = "AKQJT98765432";

/// Class at grid position (row, col).
pub fn class_at(row: usize, col: usize) -> String {
    let r: Vec<char> = RANKS.chars().collect();
    match row.cmp(&col) {
        std::cmp::Ordering::Equal => format!("{}{}", r[row], r[row]),
        std::cmp::Ordering::Less => format!("{}{}s", r[row], r[col]),
        std::cmp::Ordering::Greater => format!("{}{}o", r[col], r[row]),
    }
}

/// 169 cells, row by row.
pub fn grid(conn: &Connection, filter: &Filter) -> rusqlite::Result<Vec<Cell>> {
    let w = filter.to_sql();
    let sql = format!(
        "SELECT hp.hole_cards, hp.net_won_bb, s.cnt_vpip, s.cnt_pfr
         FROM hand_player_stats s JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
         JOIN hands h ON h.id = s.hand_id JOIN players p ON p.id = hp.player_id
         WHERE {} AND length(hp.hole_cards) = 4",
        w.text
    );
    let mut acc: HashMap<String, Cell> = HashMap::new();
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(params_from_iter(w.params), |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
    })?;
    for row in rows {
        let (cards, bb, vpip, pfr) = row?;
        let spaced = format!("{} {}", &cards[..2], &cards[2..]);
        let Some(class) = parse_cards(&spaced).ok().and_then(|c| hand_class(&c)) else { continue };
        let cell = acc.entry(class.clone()).or_insert_with(|| Cell { class, ..Default::default() });
        cell.hands += 1;
        cell.net_bb += bb;
        cell.vpip += vpip;
        cell.pfr += pfr;
    }
    let mut out = Vec::with_capacity(169);
    for row in 0..13 {
        for col in 0..13 {
            let class = class_at(row, col);
            out.push(acc.remove(&class).unwrap_or(Cell { class, ..Default::default() }));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::class_at;

    #[test]
    fn layout() {
        assert_eq!(class_at(0, 0), "AA");
        assert_eq!(class_at(0, 1), "AKs");
        assert_eq!(class_at(1, 0), "AKo");
        assert_eq!(class_at(12, 12), "22");
        assert_eq!(class_at(11, 12), "32s");
    }
}
