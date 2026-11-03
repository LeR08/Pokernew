//! Data the user enters: player notes and colour labels, bankroll movements.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::DbError;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub text: String,
    pub auto: bool,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerNotes {
    pub player_id: i64,
    pub name: String,
    pub colour: Option<String>,
    pub notes: Vec<Note>,
}

fn player_id(conn: &Connection, room: &str, name: &str) -> Result<i64, DbError> {
    conn.query_row(
        "SELECT id FROM players WHERE room = ? AND name = ?",
        params![room, name],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| DbError::Other(format!("unknown player {name}")))
}

pub fn notes(conn: &Connection, room: &str, name: &str) -> Result<PlayerNotes, DbError> {
    let id = player_id(conn, room, name)?;
    let colour: Option<String> =
        conn.query_row("SELECT colour FROM players WHERE id = ?", [id], |r| {
            r.get(0)
        })?;
    let mut st = conn.prepare("SELECT id, text, auto, created_at FROM notes WHERE player_id = ? ORDER BY created_at DESC, id DESC")?;
    let notes = st
        .query_map([id], |r| {
            Ok(Note {
                id: r.get(0)?,
                text: r.get(1)?,
                auto: r.get(2)?,
                created_at: r.get(3)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(PlayerNotes {
        player_id: id,
        name: name.into(),
        colour,
        notes,
    })
}

pub fn add_note(
    conn: &Connection,
    room: &str,
    name: &str,
    text: &str,
    now: i64,
) -> Result<i64, DbError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DbError::Other("empty note".into()));
    }
    let id = player_id(conn, room, name)?;
    conn.execute(
        "INSERT INTO notes (player_id, text, auto, created_at) VALUES (?, ?, 0, ?)",
        params![id, text, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_note(conn: &Connection, note_id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM notes WHERE id = ?", [note_id])?;
    Ok(())
}

/// Colour label of a player ("red", "green"…; None clears it).
pub fn set_colour(
    conn: &Connection,
    room: &str,
    name: &str,
    colour: Option<&str>,
) -> Result<(), DbError> {
    let id = player_id(conn, room, name)?;
    conn.execute(
        "UPDATE players SET colour = ? WHERE id = ?",
        params![colour, id],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tx {
    #[serde(default)]
    pub id: i64,
    pub room: String,
    pub at: i64,
    /// deposit, withdrawal, bonus, adjustment
    pub kind: String,
    /// Always positive; the sign comes from `kind` (withdrawal = money out).
    pub amount_cents: i64,
    #[serde(default)]
    pub note: String,
}

pub fn list_tx(conn: &Connection) -> Result<Vec<Tx>, DbError> {
    let mut st = conn.prepare(
        "SELECT id, room, at, kind, amount_cents, note FROM bankroll_tx ORDER BY at DESC, id DESC",
    )?;
    let rows = st.query_map([], |r| {
        Ok(Tx {
            id: r.get(0)?,
            room: r.get(1)?,
            at: r.get(2)?,
            kind: r.get(3)?,
            amount_cents: r.get(4)?,
            note: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn add_tx(conn: &Connection, tx: &Tx) -> Result<i64, DbError> {
    if tx.amount_cents <= 0 {
        return Err(DbError::Other("amount must be positive".into()));
    }
    conn.execute(
        "INSERT INTO bankroll_tx (room, at, kind, amount_cents, note) VALUES (?, ?, ?, ?, ?)",
        params![tx.room, tx.at, tx.kind, tx.amount_cents, tx.note],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_tx(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM bankroll_tx WHERE id = ?", [id])?;
    Ok(())
}

/// Signed effect of a movement on the bankroll.
pub fn signed(tx: &Tx) -> i64 {
    if tx.kind == "withdrawal" {
        -tx.amount_cents
    } else {
        tx.amount_cents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_colours_and_movements() {
        let conn = crate::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO players (room, name) VALUES ('winamax', 'Villain')",
            [],
        )
        .unwrap();
        add_note(&conn, "winamax", "Villain", "  paie trop large  ", 10).unwrap();
        add_note(&conn, "winamax", "Villain", "bluffe la river", 20).unwrap();
        assert!(add_note(&conn, "winamax", "Villain", "  ", 30).is_err());
        set_colour(&conn, "winamax", "Villain", Some("red")).unwrap();
        let n = notes(&conn, "winamax", "Villain").unwrap();
        assert_eq!(n.colour.as_deref(), Some("red"));
        assert_eq!(
            n.notes.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(),
            ["bluffe la river", "paie trop large"]
        );
        delete_note(&conn, n.notes[0].id).unwrap();
        assert_eq!(notes(&conn, "winamax", "Villain").unwrap().notes.len(), 1);

        let dep = Tx {
            id: 0,
            room: "winamax".into(),
            at: 1,
            kind: "deposit".into(),
            amount_cents: 5000,
            note: String::new(),
        };
        add_tx(&conn, &dep).unwrap();
        add_tx(
            &conn,
            &Tx {
                kind: "withdrawal".into(),
                amount_cents: 2000,
                at: 2,
                ..dep.clone()
            },
        )
        .unwrap();
        assert!(add_tx(
            &conn,
            &Tx {
                amount_cents: 0,
                ..dep.clone()
            }
        )
        .is_err());
        assert!(add_tx(
            &conn,
            &Tx {
                kind: "theft".into(),
                ..dep.clone()
            }
        )
        .is_err());
        let all = list_tx(&conn).unwrap();
        assert_eq!(all.iter().map(signed).sum::<i64>(), 3000);
    }
}
