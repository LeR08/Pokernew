//! Bankroll curve: deposits/withdrawals + cash results + tournament results,
//! in time order, in euros.

use lps_db::user::{list_tx, signed};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bankroll {
    pub t: Vec<i64>,
    pub balance_eur: Vec<f64>,
    pub deposits_cents: i64,
    pub withdrawals_cents: i64,
    pub cash_cents: i64,
    pub tournaments_cents: i64,
    pub balance_cents: i64,
    /// Average total buy-in of the tournaments played (cents).
    pub avg_buy_in_cents: Option<i64>,
}

pub fn bankroll(conn: &Connection) -> Result<Bankroll, lps_db::DbError> {
    let mut events: Vec<(i64, i64, u8)> = Vec::new(); // (time, cents, kind 0=tx 1=cash 2=tourney)
    for tx in list_tx(conn)? {
        events.push((tx.at, signed(&tx), 0));
    }
    {
        // Cash results per hand.
        let mut st = conn.prepare(
            "SELECT h.played_at, hp.net_won FROM hand_players hp JOIN hands h ON h.id = hp.hand_id
             WHERE hp.is_hero = 1 AND h.unit = 'cents'",
        )?;
        for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
            let (t, c) = r?;
            events.push((t, c, 1));
        }
        // Tournament results (known from summaries).
        let mut st = conn.prepare(
            "SELECT COALESCE(started_at, 0),
                    COALESCE(won_cents, 0) + COALESCE(bounty_won_cents, 0) + ticket_won_cents - (buy_in_cents + fee_cents) * entries
             FROM tournaments WHERE has_summary = 1",
        )?;
        for r in st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))? {
            let (t, c) = r?;
            events.push((t, c, 2));
        }
    }
    events.sort_by_key(|e| e.0);
    let mut b = Bankroll::default();
    let mut bal = 0i64;
    for (t, c, k) in events {
        bal += c;
        match k {
            0 if c >= 0 => b.deposits_cents += c,
            0 => b.withdrawals_cents += -c,
            1 => b.cash_cents += c,
            _ => b.tournaments_cents += c,
        }
        if b.t.last() == Some(&t) {
            *b.balance_eur.last_mut().expect("same length") = bal as f64 / 100.0;
        } else {
            b.t.push(t);
            b.balance_eur.push(bal as f64 / 100.0);
        }
    }
    b.balance_cents = bal;
    b.avg_buy_in_cents = conn
        .query_row("SELECT AVG(buy_in_cents + fee_cents) FROM tournaments", [], |r| r.get::<_, Option<f64>>(0))?
        .map(|v| v.round() as i64);
    Ok(b)
}
