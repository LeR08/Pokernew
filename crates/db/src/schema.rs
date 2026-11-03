//! Schema and migrations.
//!
//! Migrations are plain SQL scripts applied in order; `PRAGMA user_version`
//! stores how many ran. Counter columns of `hand_player_stats` are added
//! from [`crate::counters::COUNTERS`] after the migrations, so adding a
//! counter needs no hand-written migration (the new column is then filled
//! by a recompute).

use rusqlite::Connection;

use crate::counters::COUNTERS;
use crate::DbError;

const MIGRATIONS: &[&str] = &[
    // v1 — phase 1
    r#"
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE players (
    id INTEGER PRIMARY KEY,
    room TEXT NOT NULL,
    name TEXT NOT NULL,
    is_hero INTEGER NOT NULL DEFAULT 0,
    colour TEXT,
    UNIQUE (room, name)
);

CREATE TABLE tournaments (
    id INTEGER PRIMARY KEY,
    room TEXT NOT NULL,
    room_tournament_id TEXT NOT NULL,
    name TEXT NOT NULL,
    game_type TEXT NOT NULL,
    currency TEXT NOT NULL,
    buy_in_cents INTEGER NOT NULL,       -- prize pool part (prize + bounty)
    bounty_cents INTEGER,                -- bounty part, from the summary
    fee_cents INTEGER NOT NULL,
    registered_players INTEGER,
    mode TEXT, kind TEXT, speed TEXT,
    prize_pool_cents INTEGER,
    started_at INTEGER,
    duration_secs INTEGER,
    finish_position INTEGER,
    won_cents INTEGER,
    bounty_won_cents INTEGER,
    has_summary INTEGER NOT NULL DEFAULT 0,
    UNIQUE (room, room_tournament_id)
);

CREATE TABLE import_files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    room TEXT,
    kind TEXT,                           -- 'hands' | 'summary' | 'unknown'
    size INTEGER NOT NULL DEFAULT 0,
    mtime INTEGER NOT NULL DEFAULT 0,
    offset INTEGER NOT NULL DEFAULT 0,   -- bytes already imported (hand boundary)
    hands INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE hands (
    id INTEGER PRIMARY KEY,
    room TEXT NOT NULL,
    hand_no TEXT NOT NULL,
    played_at INTEGER NOT NULL,
    game_type TEXT NOT NULL,
    unit TEXT NOT NULL,
    currency TEXT NOT NULL,
    small_blind INTEGER NOT NULL,
    big_blind INTEGER NOT NULL,
    ante INTEGER NOT NULL,
    straddle INTEGER,
    level INTEGER,
    tournament_id INTEGER REFERENCES tournaments(id),
    table_name TEXT NOT NULL,
    table_number INTEGER,
    max_seats INTEGER NOT NULL,
    button_seat INTEGER NOT NULL,
    n_players INTEGER NOT NULL,
    board TEXT NOT NULL,                 -- "Ac4c8dJs9d"
    pot INTEGER NOT NULL,
    rake INTEGER NOT NULL,
    jackpot_fee INTEGER NOT NULL,
    showdown INTEGER NOT NULL,
    file_id INTEGER REFERENCES import_files(id),
    UNIQUE (room, hand_no)
);
CREATE INDEX hands_played_at ON hands (played_at);
CREATE INDEX hands_game_limit ON hands (game_type, big_blind);
CREATE INDEX hands_tournament ON hands (tournament_id);

CREATE TABLE hand_players (
    hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
    player_id INTEGER NOT NULL REFERENCES players(id),
    idx INTEGER NOT NULL,                -- index in the hand (action.player)
    seat INTEGER NOT NULL,
    position TEXT,
    stack INTEGER NOT NULL,
    stack_bb REAL NOT NULL,
    hole_cards TEXT NOT NULL,
    bounty_cents INTEGER,
    invested INTEGER NOT NULL,
    collected INTEGER NOT NULL,
    net_won INTEGER NOT NULL,
    net_won_bb REAL NOT NULL,
    allin_ev INTEGER,                    -- filled from phase 4 (equity)
    went_to_showdown INTEGER NOT NULL,
    is_hero INTEGER NOT NULL,
    played_at INTEGER NOT NULL,          -- copy of hands.played_at for the index
    PRIMARY KEY (hand_id, player_id)
) WITHOUT ROWID;
CREATE INDEX hand_players_player_time ON hand_players (player_id, played_at);
CREATE INDEX hand_players_position ON hand_players (position);

CREATE TABLE hand_actions (
    hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    player_idx INTEGER NOT NULL,
    street TEXT NOT NULL,
    action TEXT NOT NULL,
    amount INTEGER NOT NULL,
    raise_to INTEGER,
    is_all_in INTEGER NOT NULL,
    pot_before INTEGER NOT NULL,
    PRIMARY KEY (hand_id, seq)
) WITHOUT ROWID;

CREATE TABLE hand_player_stats (
    hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
    player_id INTEGER NOT NULL REFERENCES players(id),
    PRIMARY KEY (hand_id, player_id)
) WITHOUT ROWID;

CREATE TABLE import_errors (
    id INTEGER PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES import_files(id) ON DELETE CASCADE,
    byte_offset INTEGER NOT NULL,
    line_no INTEGER NOT NULL,
    line TEXT NOT NULL,
    message TEXT NOT NULL,
    raw TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (file_id, byte_offset)
);

CREATE TABLE sessions (
    id INTEGER PRIMARY KEY,
    room TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL,
    hands INTEGER NOT NULL,
    net_cents INTEGER NOT NULL
);

CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, colour TEXT);
CREATE TABLE hand_tags (
    hand_id INTEGER NOT NULL REFERENCES hands(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (hand_id, tag_id)
) WITHOUT ROWID;

CREATE TABLE notes (
    id INTEGER PRIMARY KEY,
    player_id INTEGER NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    text TEXT NOT NULL,
    auto INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);
"#,
    // v2 — re-entries: one tournament row, `entries` buy-ins.
    r#"
ALTER TABLE tournaments ADD COLUMN entries INTEGER NOT NULL DEFAULT 1;
-- The parser learned new formats: files that had errors are parsed again.
UPDATE import_files SET offset = 0, size = -1, errors = 0
    WHERE id IN (SELECT DISTINCT file_id FROM import_errors);
DELETE FROM import_errors;
"#,
    // v3 — satellites: value of tickets won.
    r#"
ALTER TABLE tournaments ADD COLUMN ticket_won_cents INTEGER NOT NULL DEFAULT 0;
"#,
    // v4 — bankroll: deposits, withdrawals, bonuses per room.
    r#"
CREATE TABLE bankroll_tx (
    id INTEGER PRIMARY KEY,
    room TEXT NOT NULL,
    at INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('deposit', 'withdrawal', 'bonus', 'adjustment')),
    amount_cents INTEGER NOT NULL,
    note TEXT NOT NULL DEFAULT ''
);
CREATE INDEX bankroll_tx_at ON bankroll_tx (at);
"#,
];

/// Connection pragmas: WAL so the UI can read while the importer writes.
pub fn configure(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA temp_store = MEMORY;
         PRAGMA cache_size = -32000;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(())
}

/// Applies pending migrations and adds missing counter columns.
/// Returns the counters that were added (their values must be recomputed).
pub fn migrate(conn: &mut Connection) -> Result<Vec<&'static str>, DbError> {
    let version: usize = conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    if version > MIGRATIONS.len() {
        return Err(DbError::TooNew { found: version, supported: MIGRATIONS.len() });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    let existing: Vec<String> = {
        let mut st = conn.prepare("SELECT name FROM pragma_table_info('hand_player_stats')")?;
        let rows = st.query_map([], |r| r.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    let fresh_db: bool = conn.query_row("SELECT NOT EXISTS (SELECT 1 FROM hands)", [], |r| r.get(0))?;
    let mut added = Vec::new();
    for &c in COUNTERS {
        if !existing.iter().any(|e| e == c) {
            conn.execute_batch(&format!("ALTER TABLE hand_player_stats ADD COLUMN {c} INTEGER NOT NULL DEFAULT 0"))?;
            if !fresh_db {
                added.push(c);
            }
        }
    }
    Ok(added)
}
