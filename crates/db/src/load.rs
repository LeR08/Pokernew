//! Rebuilds a [`Hand`] from the database (replayer, counter recompute).

use lps_core_model::{
    parse_cards, Action, ActionKind, BettingLimit, Card, GameType, Hand, PlayerInHand, Position,
    Street, TournamentRef, Unit, Variant,
};
use rusqlite::{params, Connection};

use crate::DbError;

fn compact_cards(s: &str) -> Vec<Card> {
    let spaced: Vec<String> = s
        .as_bytes()
        .chunks(2)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect();
    parse_cards(&spaced.join(" ")).unwrap_or_default()
}

fn game_type(s: &str) -> GameType {
    match s {
        "cash" => GameType::Cash,
        "sng" => GameType::Sng,
        "spin" => GameType::Spin,
        _ => GameType::Mtt,
    }
}

fn position(s: &str) -> Option<Position> {
    use Position::*;
    Some(match s {
        "UTG" => Utg,
        "UTG1" => Utg1,
        "UTG2" => Utg2,
        "MP" => Mp,
        "LJ" => Lj,
        "HJ" => Hj,
        "CO" => Co,
        "BTN" => Btn,
        "SB" => Sb,
        "BB" => Bb,
        _ => return None,
    })
}

fn street(s: &str) -> Street {
    match s {
        "flop" => Street::Flop,
        "turn" => Street::Turn,
        "river" => Street::River,
        _ => Street::Preflop,
    }
}

fn kind(s: &str) -> Result<ActionKind, DbError> {
    use ActionKind::*;
    Ok(match s {
        "post_ante" => PostAnte,
        "post_sb" => PostSmallBlind,
        "post_bb" => PostBigBlind,
        "post_straddle" => PostStraddle,
        "post_dead" => PostDead,
        "fold" => Fold,
        "check" => Check,
        "call" => Call,
        "bet" => Bet,
        "raise" => Raise,
        other => return Err(DbError::Corrupt(format!("unknown action '{other}'"))),
    })
}

/// Loads a stored hand. Also returns the player ids in `hand.players` order.
pub fn load_hand(conn: &Connection, hand_id: i64) -> Result<(Hand, Vec<i64>), DbError> {
    let mut hand = conn.query_row(
        "SELECT h.room, h.hand_no, h.table_name, h.game_type, h.unit, h.currency, h.small_blind, h.big_blind, h.ante,
                h.straddle, h.level, h.max_seats, h.button_seat, h.board, h.pot, h.rake, h.jackpot_fee, h.showdown,
                h.played_at, t.room_tournament_id, t.name, t.buy_in_cents, t.fee_cents, h.table_number
         FROM hands h LEFT JOIN tournaments t ON t.id = h.tournament_id WHERE h.id = ?",
        [hand_id],
        |r| {
            let tid: Option<String> = r.get(19)?;
            Ok(Hand {
                room: r.get(0)?,
                hand_no: r.get(1)?,
                table_name: r.get(2)?,
                game_type: game_type(&r.get::<_, String>(3)?),
                variant: Variant::Holdem,
                limit: BettingLimit::NoLimit,
                unit: if r.get::<_, String>(4)? == "cents" { Unit::Cents } else { Unit::Chips },
                currency: r.get(5)?,
                small_blind: r.get(6)?,
                big_blind: r.get(7)?,
                ante: r.get(8)?,
                straddle: r.get(9)?,
                level: r.get(10)?,
                tournament: match tid {
                    Some(id) => Some(TournamentRef {
                        id,
                        name: r.get(20)?,
                        buy_in_cents: r.get(21)?,
                        fee_cents: r.get(22)?,
                        table_number: r.get(23)?,
                    }),
                    None => None,
                },
                max_seats: r.get(11)?,
                button_seat: r.get(12)?,
                players: vec![],
                waiting: vec![],
                board: compact_cards(&r.get::<_, String>(13)?),
                actions: vec![],
                total_pot: r.get(14)?,
                rake: r.get(15)?,
                jackpot_fee: r.get(16)?,
                showdown: r.get(17)?,
                played_at: r.get(18)?,
            })
        },
    )?;
    let mut ids = Vec::new();
    {
        let mut st = conn.prepare_cached(
            "SELECT hp.player_id, hp.seat, p.name, hp.stack, hp.position, hp.hole_cards, hp.is_hero, hp.bounty_cents,
                    hp.invested, hp.collected, hp.went_to_showdown
             FROM hand_players hp JOIN players p ON p.id = hp.player_id WHERE hp.hand_id = ? ORDER BY hp.idx",
        )?;
        let rows = st.query_map([hand_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                PlayerInHand {
                    seat: r.get(1)?,
                    name: r.get(2)?,
                    stack: r.get(3)?,
                    position: r.get::<_, Option<String>>(4)?.as_deref().and_then(position),
                    hole_cards: compact_cards(&r.get::<_, String>(5)?),
                    is_hero: r.get(6)?,
                    bounty_cents: r.get(7)?,
                    invested: r.get(8)?,
                    collected: r.get(9)?,
                    went_to_showdown: r.get(10)?,
                },
            ))
        })?;
        for row in rows {
            let (id, p) = row?;
            ids.push(id);
            hand.players.push(p);
        }
    }
    {
        let mut st = conn.prepare_cached(
            "SELECT w.seat, p.name, w.stack FROM hand_waiting w JOIN players p ON p.id = w.player_id WHERE w.hand_id = ? ORDER BY w.seat",
        )?;
        let rows = st.query_map([hand_id], |r| {
            Ok(lps_core_model::WaitingPlayer {
                seat: r.get(0)?,
                name: r.get(1)?,
                stack: r.get(2)?,
            })
        })?;
        for row in rows {
            hand.waiting.push(row?);
        }
    }
    let mut st = conn.prepare_cached(
        "SELECT player_idx, street, action, amount, raise_to, is_all_in, pot_before FROM hand_actions WHERE hand_id = ? ORDER BY seq",
    )?;
    let rows = st.query_map(params![hand_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, bool>(5)?,
            r.get::<_, i64>(6)?,
        ))
    })?;
    for row in rows {
        let (p, s, k, amount, raise_to, is_all_in, pot_before) = row?;
        hand.actions.push(Action {
            player: p as usize,
            street: street(&s),
            kind: kind(&k)?,
            amount,
            raise_to,
            is_all_in,
            pot_before,
        });
    }
    Ok((hand, ids))
}
