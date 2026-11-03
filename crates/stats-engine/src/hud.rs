//! Data for the HUD: tables with recent hands, who sits where, their stats.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::defs::CompiledStat;
use crate::query::{self, Filter, Group, StatValue};

/// A table with hands since `since` (Unix seconds).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentTable {
    pub room: String,
    pub table_name: String,
    pub tournament_id: Option<String>,
    pub table_number: Option<u32>,
    pub max_seats: u8,
    pub last_hand_at: i64,
}

pub fn recent_tables(conn: &Connection, since: i64) -> rusqlite::Result<Vec<RecentTable>> {
    let mut st = conn.prepare(
        "SELECT h.room, h.table_name, t.room_tournament_id, MAX(h.table_number), MAX(h.max_seats), MAX(h.played_at)
         FROM hands h LEFT JOIN tournaments t ON t.id = h.tournament_id
         WHERE h.played_at >= ? GROUP BY h.room, h.table_name ORDER BY 6 DESC",
    )?;
    let rows = st.query_map([since], |r| {
        Ok(RecentTable {
            room: r.get(0)?,
            table_name: r.get(1)?,
            tournament_id: r.get(2)?,
            table_number: r.get(3)?,
            max_seats: r.get(4)?,
            last_hand_at: r.get(5)?,
        })
    })?;
    rows.collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudSeat {
    pub seat: u8,
    /// Display slot: 0 = hero (when the room rotates the hero to the bottom),
    /// then clockwise. Otherwise seat − 1.
    pub slot: u8,
    pub name: String,
    pub is_hero: bool,
    pub position: Option<String>,
    pub hands: i64,
    pub stats: Vec<StatValue>,
    pub colour: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudTable {
    pub room: String,
    pub table_name: String,
    pub max_seats: u8,
    pub last_hand_at: i64,
    pub seats: Vec<HudSeat>,
}

/// Slots for seats: hero first then clockwise (by seat number), or seat − 1.
pub fn slots(seats: &[(u8, bool)], max_seats: u8, rotate: bool) -> Vec<u8> {
    let hero = seats.iter().find(|(_, h)| *h).map(|(s, _)| *s);
    seats
        .iter()
        .map(|(s, _)| match (rotate, hero) {
            (true, Some(h)) => ((*s as i32 - h as i32).rem_euclid(max_seats.max(1) as i32)) as u8,
            _ => s.saturating_sub(1),
        })
        .collect()
}

/// (seat, name, is_hero, position, colour, player id)
type Person = (u8, String, bool, Option<String>, Option<String>, i64);

/// The players of the last hand at `table_name`, with their stats over all their hands in this room.
pub fn table(conn: &Connection, room: &str, table_name: &str, stats: &[CompiledStat], rotate: bool) -> rusqlite::Result<Option<HudTable>> {
    let last: Option<(i64, u8, i64)> = conn
        .query_row(
            "SELECT id, max_seats, played_at FROM hands WHERE room = ? AND table_name = ? ORDER BY played_at DESC, id DESC LIMIT 1",
            params![room, table_name],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    let Some((hand_id, max_seats, last_hand_at)) = last else { return Ok(None) };
    let mut st = conn.prepare(
        "SELECT hp.seat, p.name, hp.is_hero, hp.position, p.colour, p.id FROM hand_players hp JOIN players p ON p.id = hp.player_id
         WHERE hp.hand_id = ? ORDER BY hp.seat",
    )?;
    let mut people: Vec<Person> = st
        .query_map([hand_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<rusqlite::Result<_>>()?;
    // Players seated but not dealt in the last hand (new arrivals) get a panel too.
    let mut wst = conn.prepare_cached(
        "SELECT w.seat, p.name, 0, NULL, p.colour, p.id FROM hand_waiting w JOIN players p ON p.id = w.player_id WHERE w.hand_id = ?",
    )?;
    let waiting: Vec<Person> =
        wst.query_map([hand_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?.collect::<rusqlite::Result<_>>()?;
    people.extend(waiting);
    people.sort_by_key(|p| p.0);
    let sl = slots(&people.iter().map(|p| (p.0, p.2)).collect::<Vec<_>>(), max_seats, rotate);
    let mut seats = Vec::with_capacity(people.len());
    for ((seat, name, is_hero, position, colour, pid), slot) in people.into_iter().zip(sl) {
        let f = Filter { player: Some(name.clone()), rooms: vec![room.into()], ..Default::default() };
        let row = query::report(conn, &f, stats, Group::None)?.into_iter().next();
        let mut nst = conn.prepare_cached("SELECT text FROM notes WHERE player_id = ? ORDER BY auto, created_at DESC LIMIT 5")?;
        let notes: Vec<String> = nst.query_map([pid], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        seats.push(HudSeat {
            seat,
            slot,
            name,
            is_hero,
            position,
            hands: row.as_ref().map(|r| r.hands).unwrap_or(0),
            stats: row.map(|r| r.stats).unwrap_or_default(),
            colour,
            notes,
        });
    }
    Ok(Some(HudTable { room: room.into(), table_name: table_name.into(), max_seats, last_hand_at, seats }))
}

#[cfg(test)]
mod tests {
    use super::slots;

    #[test]
    fn hero_is_slot_zero_then_clockwise() {
        // 6-max, hero in seat 3; seats 1, 2, 3, 5, 6 occupied.
        let s = slots(&[(1, false), (2, false), (3, true), (5, false), (6, false)], 6, true);
        assert_eq!(s, [4, 5, 0, 2, 3]);
        assert_eq!(slots(&[(1, false), (3, true)], 6, false), [0, 2]);
        assert_eq!(slots(&[(1, false), (2, false)], 3, true), [0, 1], "no hero: seat order");
    }
}
