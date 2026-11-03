//! Writing hands and tournament summaries.

use std::collections::HashMap;

use lps_core_model::{cards_to_string, Hand, TournamentSummary};
use rusqlite::{params, OptionalExtension, Transaction};

use crate::counters::{compute, COUNTERS};
use crate::DbError;

/// Caches ids across a batch so each player/tournament is looked up once.
#[derive(Default)]
pub struct IdCache {
    players: HashMap<(String, String), (i64, bool)>,
    tournaments: HashMap<(String, String), i64>,
}

fn stats_sql() -> String {
    let cols = COUNTERS.join(", ");
    let marks = vec!["?"; COUNTERS.len() + 2].join(", ");
    format!("INSERT OR REPLACE INTO hand_player_stats (hand_id, player_id, {cols}) VALUES ({marks})")
}

fn player_id(tx: &Transaction<'_>, cache: &mut IdCache, room: &str, name: &str, is_hero: bool) -> Result<i64, DbError> {
    let key = (room.to_string(), name.to_string());
    if let Some(&(id, hero)) = cache.players.get(&key) {
        if is_hero && !hero {
            tx.prepare_cached("UPDATE players SET is_hero = 1 WHERE id = ?")?.execute([id])?;
            cache.players.insert(key, (id, true));
        }
        return Ok(id);
    }
    tx.prepare_cached("INSERT OR IGNORE INTO players (room, name, is_hero) VALUES (?, ?, ?)")?
        .execute(params![room, name, is_hero])?;
    let (id, hero): (i64, bool) = tx
        .prepare_cached("SELECT id, is_hero FROM players WHERE room = ? AND name = ?")?
        .query_row(params![room, name], |r| Ok((r.get(0)?, r.get(1)?)))?;
    let hero = if is_hero && !hero {
        tx.prepare_cached("UPDATE players SET is_hero = 1 WHERE id = ?")?.execute([id])?;
        true
    } else {
        hero
    };
    cache.players.insert(key, (id, hero));
    Ok(id)
}

fn tournament_id(tx: &Transaction<'_>, cache: &mut IdCache, hand: &Hand) -> Result<Option<i64>, DbError> {
    let Some(t) = &hand.tournament else { return Ok(None) };
    let key = (hand.room.clone(), t.id.clone());
    if let Some(&id) = cache.tournaments.get(&key) {
        return Ok(Some(id));
    }
    tx.prepare_cached(
        "INSERT OR IGNORE INTO tournaments (room, room_tournament_id, name, game_type, currency, buy_in_cents, fee_cents)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )?
    .execute(params![hand.room, t.id, t.name, hand.game_type.as_str(), hand.currency, t.buy_in_cents, t.fee_cents])?;
    let id: i64 = tx
        .prepare_cached("SELECT id FROM tournaments WHERE room = ? AND room_tournament_id = ?")?
        .query_row(params![hand.room, t.id], |r| r.get(0))?;
    cache.tournaments.insert(key, id);
    Ok(Some(id))
}

/// Inserts one hand. Returns `false` when it was already in the database
/// (same room + hand number): nothing is written then.
pub fn insert_hand(tx: &Transaction<'_>, cache: &mut IdCache, hand: &Hand, file_id: Option<i64>) -> Result<bool, DbError> {
    let tid = tournament_id(tx, cache, hand)?;
    let inserted = tx
        .prepare_cached(
            "INSERT OR IGNORE INTO hands (room, hand_no, played_at, game_type, unit, currency, small_blind, big_blind, ante,
                straddle, level, tournament_id, table_name, table_number, max_seats, button_seat, n_players, board, pot,
                rake, jackpot_fee, showdown, file_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?
        .execute(params![
            hand.room,
            hand.hand_no,
            hand.played_at,
            hand.game_type.as_str(),
            match hand.unit {
                lps_core_model::Unit::Cents => "cents",
                lps_core_model::Unit::Chips => "chips",
            },
            hand.currency,
            hand.small_blind,
            hand.big_blind,
            hand.ante,
            hand.straddle,
            hand.level,
            tid,
            hand.table_name,
            hand.tournament.as_ref().and_then(|t| t.table_number),
            hand.max_seats,
            hand.button_seat,
            hand.players.len() as i64,
            cards_to_string(&hand.board),
            hand.total_pot,
            hand.rake,
            hand.jackpot_fee,
            hand.showdown,
            file_id,
        ])?;
    if inserted == 0 {
        return Ok(false);
    }
    let hand_id = tx.last_insert_rowid();
    let bb = hand.big_blind.max(1) as f64;
    let mut ids = Vec::with_capacity(hand.players.len());
    for (idx, p) in hand.players.iter().enumerate() {
        let pid = player_id(tx, cache, &hand.room, &p.name, p.is_hero)?;
        ids.push(pid);
        tx.prepare_cached(
            "INSERT INTO hand_players (hand_id, player_id, idx, seat, position, stack, stack_bb, hole_cards, bounty_cents,
                invested, collected, net_won, net_won_bb, went_to_showdown, is_hero, played_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?
        .execute(params![
            hand_id,
            pid,
            idx as i64,
            p.seat,
            p.position.map(|x| x.as_str()),
            p.stack,
            p.stack as f64 / bb,
            cards_to_string(&p.hole_cards),
            p.bounty_cents,
            p.invested,
            p.collected,
            p.net_won(),
            p.net_won() as f64 / bb,
            p.went_to_showdown,
            p.is_hero,
            hand.played_at,
        ])?;
    }
    {
        let mut st = tx.prepare_cached(
            "INSERT INTO hand_actions (hand_id, seq, player_idx, street, action, amount, raise_to, is_all_in, pot_before)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        for (seq, a) in hand.actions.iter().enumerate() {
            st.execute(params![
                hand_id,
                seq as i64,
                a.player as i64,
                a.street.as_str(),
                a.kind.as_str(),
                a.amount,
                a.raise_to,
                a.is_all_in,
                a.pot_before
            ])?;
        }
    }
    write_counters(tx, hand_id, &ids, hand)?;
    write_derived(tx, hand_id, &ids, hand)?;
    Ok(true)
}

/// (Re)writes the counters row of every player of a stored hand.
pub fn write_counters(tx: &Transaction<'_>, hand_id: i64, player_ids: &[i64], hand: &Hand) -> Result<(), DbError> {
    let sql = stats_sql();
    let mut st = tx.prepare_cached(&sql)?;
    for (pid, c) in player_ids.iter().zip(compute(hand)) {
        let mut values: Vec<i64> = Vec::with_capacity(COUNTERS.len() + 2);
        values.push(hand_id);
        values.push(*pid);
        values.extend(c.values());
        st.execute(rusqlite::params_from_iter(values))?;
    }
    Ok(())
}

/// Derived columns used by the advanced filters.
pub fn write_derived(tx: &Transaction<'_>, hand_id: i64, player_ids: &[i64], hand: &Hand) -> Result<(), DbError> {
    let d = lps_core_model::derived::derive(hand);
    tx.prepare_cached("UPDATE hands SET pf_raises = ?, players_flop = ?, flop_texture = ? WHERE id = ?")?
        .execute(params![d.pf_raises, d.players_flop, d.flop_texture, hand_id])?;
    let mut st = tx.prepare_cached("UPDATE hand_players SET hand_class = ? WHERE hand_id = ? AND player_id = ?")?;
    for (pid, p) in player_ids.iter().zip(&hand.players) {
        st.execute(params![lps_core_model::preflop::hand_class(&p.hole_cards), hand_id, pid])?;
    }
    Ok(())
}

/// Inserts or completes a tournament from its summary file. `entries` holds
/// one summary per entry (re-entries): the last one gives the final result,
/// winnings and bounties are summed, and `entries` buy-ins were paid.
pub fn upsert_summary(tx: &Transaction<'_>, entries: &[TournamentSummary]) -> Result<(), DbError> {
    let Some(s) = entries.last() else { return Ok(()) };
    let won: i64 = entries.iter().map(|e| e.won_cents).sum();
    let bounty_won: i64 = entries.iter().map(|e| e.bounty_won_cents).sum();
    let ticket_won: i64 = entries.iter().map(|e| e.ticket_won_cents).sum();
    // Same rule as the hand parser (Expresso = Spin & Go format).
    let game_type = if s.name.starts_with("Expresso") { "spin" } else { "mtt" };
    let duration: Option<i64> = entries.iter().map(|e| e.duration_secs).sum();
    tx.execute(
        "INSERT INTO tournaments (room, room_tournament_id, name, game_type, currency, buy_in_cents, bounty_cents, fee_cents,
            registered_players, mode, kind, speed, prize_pool_cents, started_at, duration_secs, finish_position,
            won_cents, bounty_won_cents, has_summary, entries, ticket_won_cents)
         VALUES (?1, ?2, ?3, ?19, 'EUR', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 1, ?17, ?18)
         ON CONFLICT (room, room_tournament_id) DO UPDATE SET
            name = excluded.name, buy_in_cents = excluded.buy_in_cents, bounty_cents = excluded.bounty_cents,
            fee_cents = excluded.fee_cents, registered_players = excluded.registered_players, mode = excluded.mode,
            kind = excluded.kind, speed = excluded.speed, prize_pool_cents = excluded.prize_pool_cents,
            started_at = excluded.started_at, duration_secs = excluded.duration_secs,
            finish_position = excluded.finish_position, won_cents = excluded.won_cents,
            bounty_won_cents = excluded.bounty_won_cents, has_summary = 1, entries = excluded.entries,
            ticket_won_cents = excluded.ticket_won_cents",
        params![
            s.room,
            s.id,
            s.name,
            s.buy_in_prize_cents + s.buy_in_bounty_cents,
            s.buy_in_bounty_cents,
            s.fee_cents,
            s.registered_players,
            s.mode,
            s.kind,
            s.speed,
            s.prize_pool_cents,
            entries.first().and_then(|e| e.started_at),
            duration,
            s.finish_position,
            won,
            bounty_won,
            entries.len() as i64,
            ticket_won,
            game_type,
        ],
    )?;
    Ok(())
}

/// Looks up a hand id.
pub fn hand_id(tx: &rusqlite::Connection, room: &str, hand_no: &str) -> Result<Option<i64>, DbError> {
    Ok(tx
        .query_row("SELECT id FROM hands WHERE room = ? AND hand_no = ?", params![room, hand_no], |r| r.get(0))
        .optional()?)
}
