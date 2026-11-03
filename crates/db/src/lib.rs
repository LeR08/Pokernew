//! Local SQLite database: schema, counters, import pipeline, backups.
//!
//! One file (`database.sqlite`, WAL mode). The importer owns the only
//! writer connection; the UI reads through separate connections.

pub mod backup;
pub mod counters;
pub mod import;
pub mod load;
pub mod schema;
pub mod user;
pub mod watch;
pub mod write;

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("database version {found} is newer than this app supports ({supported})")]
    TooNew { found: usize, supported: usize },
    #[error("corrupt database: {0}")]
    Corrupt(String),
    #[error("{0}")]
    Other(String),
}

/// Bumped whenever the parsers learn a new format: files that had errors are
/// then parsed again automatically on the next start.
pub const PARSER_VERSION: i64 = 3;

fn requeue_errors_if_parsers_changed(conn: &Connection) -> Result<(), DbError> {
    let stored: Option<i64> = conn
        .query_row("SELECT value FROM meta WHERE key = 'parser_version'", [], |r| r.get::<_, String>(0))
        .ok()
        .and_then(|v| v.parse().ok());
    if stored != Some(PARSER_VERSION) {
        let n = import::retry_errors(conn)?;
        if n > 0 {
            log::info!("parsers updated: {n} file(s) with errors will be parsed again");
        }
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('parser_version', ?)",
            [PARSER_VERSION.to_string()],
        )?;
    }
    Ok(())
}

/// Opens (creating if needed) and migrates the database. Counters added
/// since the last run are recomputed for every stored hand.
pub fn open(path: &Path) -> Result<Connection, DbError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut conn = Connection::open(path)?;
    schema::configure(&conn)?;
    let added = schema::migrate(&mut conn)?;
    requeue_errors_if_parsers_changed(&conn)?;
    if !added.is_empty() {
        log::info!("new counters {added:?}: recomputing");
        recompute_counters(&mut conn)?;
    }
    Ok(conn)
}

/// A read-only-ish second connection for UI queries.
pub fn open_reader(path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open(path)?;
    schema::configure(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> Result<Connection, DbError> {
    let mut conn = Connection::open_in_memory()?;
    schema::configure(&conn)?;
    schema::migrate(&mut conn)?;
    Ok(conn)
}

/// Recomputes every counter of every hand from the stored actions.
pub fn recompute_counters(conn: &mut Connection) -> Result<usize, DbError> {
    let ids: Vec<i64> = {
        let mut st = conn.prepare("SELECT id FROM hands ORDER BY id")?;
        let rows = st.query_map([], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    for chunk in ids.chunks(import::BATCH) {
        let tx = conn.transaction()?;
        for &id in chunk {
            let (hand, pids) = load::load_hand(&tx, id)?;
            write::write_counters(&tx, id, &pids, &hand)?;
        }
        tx.commit()?;
    }
    Ok(ids.len())
}

/// Totals for the import screen.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub hands: i64,
    pub hero_hands: i64,
    pub players: i64,
    pub tournaments: i64,
    pub files: i64,
    pub errors: i64,
    pub first_hand_at: Option<i64>,
    pub last_hand_at: Option<i64>,
}

pub fn stats(conn: &Connection) -> Result<DbStats, DbError> {
    let one = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0));
    let (first, last): (Option<i64>, Option<i64>) =
        conn.query_row("SELECT MIN(played_at), MAX(played_at) FROM hands", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(DbStats {
        hands: one("SELECT COUNT(*) FROM hands")?,
        hero_hands: one("SELECT COUNT(*) FROM hand_players WHERE is_hero = 1")?,
        players: one("SELECT COUNT(*) FROM players")?,
        tournaments: one("SELECT COUNT(*) FROM tournaments")?,
        files: one("SELECT COUNT(*) FROM import_files WHERE kind IN ('hands', 'summary')")?,
        errors: one("SELECT COUNT(*) FROM import_errors")?,
        first_hand_at: first,
        last_hand_at: last,
    })
}

#[cfg(test)]
mod tests;
