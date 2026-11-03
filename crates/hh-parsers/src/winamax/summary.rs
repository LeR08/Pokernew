//! Winamax tournament summary files (`*_summary.txt`).
//!
//! Observed format (fixtures/winamax/*_summary.txt):
//!
//! ```text
//! Winamax Poker - Tournament summary : SPACE KO(1195603616)
//! Player : Tantaculejen
//! Buy-In : 0.20€ + 0.25€ + 0.05€        (prize + bounty + fee)
//! Registered players : 1771
//! Mode : tt
//! Type : knockout
//! Speed : normal
//! Flight ID : 0
//! Levels : Levels : [...]
//! Prizepool : 463.20€
//! Tournament started 2026/11/02 18:30:01 UTC
//! You played 2h 59min 6s
//! You finished in 203th place
//! You won 0.52€ + Bounty 0.51€          (absent when nothing was won)
//! ```
//!
//! A two-part buy-in (`prize + fee`, no bounty) has not been seen yet; it is
//! accepted because the meaning of the parts is unambiguous.

use std::sync::LazyLock;

use lps_core_model::TournamentSummary;
use regex::Regex;

use super::ROOM;
use crate::util::{parse_cents, parse_utc, strip_bom};
use crate::ParseError;

static TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Winamax Poker - Tournament summary : (?P<name>.*)\((?P<id>\d+)\)$").unwrap());
static WON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^You won (?P<won>[0-9.]+)€(?: \+ Bounty (?P<bounty>[0-9.]+)€)?$").unwrap());
static FINISHED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^You finished in (?P<pos>\d+)(?:st|nd|rd|th) place$").unwrap());
static PLAYED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^You played (?:(?P<h>\d+)h ?)?(?:(?P<m>\d+)min ?)?(?:(?P<s>\d+)s)?$").unwrap());

fn euros(n: usize, line: &str, s: &str) -> Result<i64, ParseError> {
    s.trim()
        .strip_suffix('€')
        .and_then(parse_cents)
        .ok_or_else(|| ParseError::new(n, line, format!("bad amount '{s}'")))
}

pub fn parse(text: &str) -> Result<TournamentSummary, ParseError> {
    let mut lines = strip_bom(text).lines().map(|l| l.trim_end_matches('\r')).enumerate().map(|(i, l)| (i + 1, l));
    let (n, first) = lines.next().ok_or_else(|| ParseError::whole("empty summary"))?;
    let t = TITLE.captures(first).ok_or_else(|| ParseError::new(n, first, "unrecognised summary title"))?;
    let mut s = TournamentSummary {
        room: ROOM.into(),
        id: t["id"].to_string(),
        name: t["name"].to_string(),
        hero: String::new(),
        buy_in_prize_cents: 0,
        buy_in_bounty_cents: 0,
        fee_cents: 0,
        registered_players: None,
        mode: None,
        kind: None,
        speed: None,
        prize_pool_cents: None,
        started_at: None,
        duration_secs: None,
        finish_position: None,
        won_cents: 0,
        bounty_won_cents: 0,
    };
    for (n, line) in lines {
        if line.trim().is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(" : ") {
            match key {
                "Player" => s.hero = value.to_string(),
                "Buy-In" => {
                    let parts: Vec<&str> = value.split(" + ").collect();
                    let cents: Vec<i64> = parts.iter().map(|p| euros(n, line, p)).collect::<Result<_, _>>()?;
                    match cents.as_slice() {
                        [prize, bounty, fee] => {
                            s.buy_in_prize_cents = *prize;
                            s.buy_in_bounty_cents = *bounty;
                            s.fee_cents = *fee;
                        }
                        [prize, fee] => {
                            s.buy_in_prize_cents = *prize;
                            s.fee_cents = *fee;
                        }
                        _ => return Err(ParseError::new(n, line, "unexpected buy-in format")),
                    }
                }
                "Registered players" => {
                    s.registered_players = Some(value.parse().map_err(|_| ParseError::new(n, line, "bad number"))?)
                }
                "Mode" => s.mode = Some(value.to_string()),
                "Type" => s.kind = Some(value.to_string()),
                "Speed" => s.speed = Some(value.to_string()),
                "Prizepool" => s.prize_pool_cents = Some(euros(n, line, value)?),
                "Flight ID" | "Levels" => {}
                _ => return Err(ParseError::new(n, line, format!("unknown summary key '{key}'"))),
            }
        } else if let Some(d) = line.strip_prefix("Tournament started ").and_then(|d| d.strip_suffix(" UTC")) {
            s.started_at = Some(parse_utc(d).ok_or_else(|| ParseError::new(n, line, "bad date"))?);
        } else if let Some(c) = PLAYED.captures(line.trim_end()) {
            let g = |k: &str| c.name(k).and_then(|m| m.as_str().parse::<i64>().ok()).unwrap_or(0);
            s.duration_secs = Some(g("h") * 3600 + g("m") * 60 + g("s"));
        } else if let Some(c) = FINISHED.captures(line) {
            s.finish_position = c["pos"].parse().ok();
        } else if let Some(c) = WON.captures(line) {
            s.won_cents = parse_cents(&c["won"]).ok_or_else(|| ParseError::new(n, line, "bad amount"))?;
            if let Some(b) = c.name("bounty") {
                s.bounty_won_cents = parse_cents(b.as_str()).ok_or_else(|| ParseError::new(n, line, "bad amount"))?;
            }
        } else {
            return Err(ParseError::new(n, line, "unknown summary line"));
        }
    }
    if s.hero.is_empty() {
        return Err(ParseError::whole("summary without 'Player'"));
    }
    Ok(s)
}
