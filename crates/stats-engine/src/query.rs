//! Report queries over the hero's hands.
//!
//! Every query starts from one row per (hand, hero player):
//! `hand_player_stats s ⋈ hand_players hp ⋈ hands h`, restricted by a
//! [`Filter`]. Stats are then single `SUM()/SUM()` aggregates.

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};

use crate::compile::Sql;
use crate::defs::CompiledStat;

/// What the user filtered on (all optional, AND-combined).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Filter {
    /// Unix seconds, inclusive.
    pub from: Option<i64>,
    /// Unix seconds, exclusive.
    pub to: Option<i64>,
    /// "cash", "mtt", "sng", "spin"; empty = all.
    pub game_types: Vec<String>,
    pub rooms: Vec<String>,
    /// Hero accounts (screen names); empty = every hero account.
    pub accounts: Vec<String>,
    pub positions: Vec<String>,
    /// Restrict to one tournament (database id).
    pub tournament: Option<i64>,
}

impl Filter {
    /// `WHERE …` for hero rows (aliases `hp`, `h`, `p` = players).
    pub fn to_sql(&self) -> Sql {
        let mut parts: Vec<String> = vec!["hp.is_hero = 1".into()];
        let mut params: Vec<Value> = Vec::new();
        if let Some(f) = self.from {
            parts.push("h.played_at >= ?".into());
            params.push(Value::Integer(f));
        }
        if let Some(t) = self.to {
            parts.push("h.played_at < ?".into());
            params.push(Value::Integer(t));
        }
        let mut list = |col: &str, values: &[String]| {
            if !values.is_empty() {
                parts.push(format!("{col} IN ({})", vec!["?"; values.len()].join(", ")));
                params.extend(values.iter().map(|v| Value::Text(v.clone())));
            }
        };
        list("h.game_type", &self.game_types);
        list("h.room", &self.rooms);
        list("p.name", &self.accounts);
        list("hp.position", &self.positions);
        if let Some(t) = self.tournament {
            parts.push("h.tournament_id = ?".into());
            params.push(Value::Integer(t));
        }
        Sql { text: parts.join(" AND "), params }
    }
}

const FROM: &str = "FROM hand_player_stats s
    JOIN hand_players hp ON hp.hand_id = s.hand_id AND hp.player_id = s.player_id
    JOIN hands h ON h.id = s.hand_id
    JOIN players p ON p.id = hp.player_id";

/// How rows are grouped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    None,
    Position,
    GameType,
    Month,
    Account,
    /// Game type + big blind (cash stakes; tournament levels are too many).
    Stake,
}

impl Group {
    fn sql(self) -> &'static str {
        match self {
            Group::None => "'all'",
            Group::Position => "COALESCE(hp.position, '?')",
            Group::GameType => "h.game_type",
            Group::Month => "strftime('%Y-%m', h.played_at, 'unixepoch')",
            Group::Account => "p.name",
            Group::Stake => "CASE WHEN h.game_type = 'cash' THEN 'cash:' || h.big_blind ELSE h.game_type END",
        }
    }
    fn order(self) -> &'static str {
        match self {
            Group::Position => {
                "CASE k WHEN 'UTG' THEN 1 WHEN 'UTG1' THEN 2 WHEN 'UTG2' THEN 3 WHEN 'MP' THEN 4 WHEN 'LJ' THEN 5
                 WHEN 'HJ' THEN 6 WHEN 'CO' THEN 7 WHEN 'BTN' THEN 8 WHEN 'SB' THEN 9 WHEN 'BB' THEN 10 ELSE 11 END"
            }
            _ => "k",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatValue {
    pub id: String,
    pub value: Option<f64>,
    pub sample: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRow {
    pub key: String,
    pub hands: i64,
    /// Sum of results in big blinds.
    pub net_bb: f64,
    /// Sum of cash-game results, in cents.
    pub net_cash_cents: i64,
    pub bb_per_100: Option<f64>,
    pub stats: Vec<StatValue>,
}

/// One row per group with every stat.
pub fn report(conn: &Connection, filter: &Filter, stats: &[CompiledStat], group: Group) -> rusqlite::Result<Vec<ReportRow>> {
    let w = filter.to_sql();
    let mut select = format!(
        "SELECT {} AS k, COUNT(*), SUM(hp.net_won_bb), SUM(CASE WHEN h.unit = 'cents' THEN hp.net_won ELSE 0 END)",
        group.sql()
    );
    let mut params: Vec<Value> = Vec::new();
    for st in stats {
        select.push_str(", ");
        select.push_str(&st.value.text);
        params.extend(st.value.params.iter().cloned());
        select.push_str(", ");
        select.push_str(&st.sample.text);
        params.extend(st.sample.params.iter().cloned());
    }
    let sql = format!("{select} {FROM} WHERE {} GROUP BY k ORDER BY {}", w.text, group.order());
    params.extend(w.params);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), |r| {
        let hands: i64 = r.get(1)?;
        let net_bb: f64 = r.get::<_, Option<f64>>(2)?.unwrap_or(0.0);
        let mut values = Vec::with_capacity(stats.len());
        for (i, st) in stats.iter().enumerate() {
            values.push(StatValue {
                id: st.id.clone(),
                value: r.get::<_, Option<f64>>(4 + 2 * i)?,
                sample: r.get::<_, Option<f64>>(5 + 2 * i)?.unwrap_or(0.0) as i64,
            });
        }
        Ok(ReportRow {
            key: r.get(0)?,
            hands,
            net_bb,
            net_cash_cents: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            bb_per_100: (hands > 0).then(|| net_bb * 100.0 / hands as f64),
            stats: values,
        })
    })?;
    rows.collect()
}

/// Cumulative results, hand by hand.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    /// Hand number (1-based) of each point.
    pub x: Vec<i64>,
    /// Unix time of each point.
    pub t: Vec<i64>,
    pub total_bb: Vec<f64>,
    pub showdown_bb: Vec<f64>,
    pub non_showdown_bb: Vec<f64>,
    /// Cash games only, in euros.
    pub cash_eur: Vec<f64>,
    pub hands: i64,
}

/// Cumulative curves; long series are thinned to about `max_points` points
/// (the last point is always kept, so the curve ends on the exact total).
pub fn graph(conn: &Connection, filter: &Filter, max_points: usize) -> rusqlite::Result<Graph> {
    let w = filter.to_sql();
    let sql = format!(
        "SELECT h.played_at, hp.net_won_bb, hp.went_to_showdown, CASE WHEN h.unit = 'cents' THEN hp.net_won ELSE 0 END
         {FROM} WHERE {} ORDER BY h.played_at, h.id",
        w.text
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<(i64, f64, bool, i64)> = stmt
        .query_map(params_from_iter(w.params), |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let n = rows.len();
    let step = n.div_ceil(max_points.max(1)).max(1);
    let mut g = Graph { hands: n as i64, ..Default::default() };
    let (mut tot, mut sd, mut nsd, mut cash) = (0.0, 0.0, 0.0, 0i64);
    for (i, (t, bb, showdown, cents)) in rows.into_iter().enumerate() {
        tot += bb;
        if showdown {
            sd += bb;
        } else {
            nsd += bb;
        }
        cash += cents;
        if (i + 1) % step == 0 || i + 1 == n {
            g.x.push(i as i64 + 1);
            g.t.push(t);
            g.total_bb.push(tot);
            g.showdown_bb.push(sd);
            g.non_showdown_bb.push(nsd);
            g.cash_eur.push(cash as f64 / 100.0);
        }
    }
    Ok(g)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentRow {
    pub id: i64,
    pub name: String,
    pub game_type: String,
    pub started_at: Option<i64>,
    pub hands: i64,
    pub entries: i64,
    /// (buy-in + fee) × entries, cents.
    pub cost_cents: i64,
    /// Prize + bounties + tickets, cents; None without a summary.
    pub won_cents: Option<i64>,
    pub profit_cents: Option<i64>,
    pub finish_position: Option<i64>,
    pub registered_players: Option<i64>,
    pub net_bb: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentTotals {
    pub tournaments: i64,
    /// Tournaments with a summary (known result).
    pub with_result: i64,
    pub cost_cents: i64,
    pub won_cents: i64,
    pub profit_cents: i64,
    pub roi: Option<f64>,
    /// In the money: won something (prize, bounty or ticket).
    pub itm: Option<f64>,
    /// Cumulative profit by tournament, in start order (euros).
    pub curve_eur: Vec<f64>,
    pub curve_t: Vec<i64>,
}

/// Tournaments the hero played, newest first, and their totals.
pub fn tournaments(conn: &Connection, filter: &Filter) -> rusqlite::Result<(Vec<TournamentRow>, TournamentTotals)> {
    let w = filter.to_sql();
    let sql = format!(
        "SELECT t.id, t.name, t.game_type, COALESCE(t.started_at, MIN(h.played_at)), COUNT(*), t.entries,
                (t.buy_in_cents + t.fee_cents) * t.entries,
                CASE WHEN t.has_summary = 1 THEN COALESCE(t.won_cents, 0) + COALESCE(t.bounty_won_cents, 0) + t.ticket_won_cents END,
                t.finish_position, t.registered_players, SUM(hp.net_won_bb)
         {FROM} JOIN tournaments t ON t.id = h.tournament_id
         WHERE {} GROUP BY t.id ORDER BY 4 DESC",
        w.text
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows: Vec<TournamentRow> = stmt
        .query_map(params_from_iter(w.params), |r| {
            let cost: i64 = r.get(6)?;
            let won: Option<i64> = r.get(7)?;
            Ok(TournamentRow {
                id: r.get(0)?,
                name: r.get(1)?,
                game_type: r.get(2)?,
                started_at: r.get(3)?,
                hands: r.get(4)?,
                entries: r.get(5)?,
                cost_cents: cost,
                won_cents: won,
                profit_cents: won.map(|w| w - cost),
                finish_position: r.get(8)?,
                registered_players: r.get(9)?,
                net_bb: r.get::<_, Option<f64>>(10)?.unwrap_or(0.0),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut tot = TournamentTotals { tournaments: rows.len() as i64, ..Default::default() };
    let mut itm = 0;
    let mut chrono: Vec<&TournamentRow> = rows.iter().filter(|r| r.won_cents.is_some()).collect();
    chrono.sort_by_key(|r| r.started_at);
    let mut acc = 0i64;
    for r in chrono {
        let won = r.won_cents.unwrap_or(0);
        tot.with_result += 1;
        tot.cost_cents += r.cost_cents;
        tot.won_cents += won;
        if won > 0 {
            itm += 1;
        }
        acc += won - r.cost_cents;
        tot.curve_eur.push(acc as f64 / 100.0);
        tot.curve_t.push(r.started_at.unwrap_or(0));
    }
    tot.profit_cents = tot.won_cents - tot.cost_cents;
    tot.roi = (tot.cost_cents > 0).then(|| tot.profit_cents as f64 / tot.cost_cents as f64);
    tot.itm = (tot.with_result > 0).then(|| itm as f64 / tot.with_result as f64);
    Ok((rows, tot))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HandRow {
    pub hand_id: i64,
    pub played_at: i64,
    pub game_type: String,
    pub table_name: String,
    pub account: String,
    pub position: Option<String>,
    pub hole_cards: String,
    pub board: String,
    pub stack_bb: f64,
    pub net_won: i64,
    pub net_won_bb: f64,
    pub unit: String,
    pub showdown: bool,
}

/// One page of the hero's hands, newest first.
pub fn hands(conn: &Connection, filter: &Filter, limit: i64, offset: i64) -> rusqlite::Result<(Vec<HandRow>, i64)> {
    let w = filter.to_sql();
    let total: i64 = conn.query_row(&format!("SELECT COUNT(*) {FROM} WHERE {}", w.text), params_from_iter(w.params.iter()), |r| r.get(0))?;
    let sql = format!(
        "SELECT h.id, h.played_at, h.game_type, h.table_name, p.name, hp.position, hp.hole_cards, h.board, hp.stack_bb,
                hp.net_won, hp.net_won_bb, h.unit, hp.went_to_showdown
         {FROM} WHERE {} ORDER BY h.played_at DESC, h.id DESC LIMIT ? OFFSET ?",
        w.text
    );
    let mut params = w.params;
    params.push(Value::Integer(limit));
    params.push(Value::Integer(offset));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(params), |r| {
            Ok(HandRow {
                hand_id: r.get(0)?,
                played_at: r.get(1)?,
                game_type: r.get(2)?,
                table_name: r.get(3)?,
                account: r.get(4)?,
                position: r.get(5)?,
                hole_cards: r.get(6)?,
                board: r.get(7)?,
                stack_bb: r.get(8)?,
                net_won: r.get(9)?,
                net_won_bb: r.get(10)?,
                unit: r.get(11)?,
                showdown: r.get(12)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok((rows, total))
}

/// Hero accounts present in the database (for the filter bar).
pub fn hero_accounts(conn: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut st = conn.prepare("SELECT name FROM players WHERE is_hero = 1 ORDER BY name")?;
    let rows = st.query_map([], |r| r.get(0))?;
    rows.collect()
}
