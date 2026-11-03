//! Winamax (winamax.fr) text hand histories.
//!
//! Written from the real files in `fixtures/winamax/` (MTT knockout, 6-max and
//! 3-max, November 2026). Format notes, all observed in those files:
//!
//! * hands are separated by two blank lines; a file ends with blank lines;
//! * header: `Winamax Poker - Tournament "<name>" buyIn: 0.45€ + 0.05€
//!   level: 5 - HandId: #<a>-<n>-<unix time> - Holdem no limit
//!   (<ante>/<sb>/<bb>) - 2026/11/02 18:34:43 UTC`;
//! * table: `Table: '<name>(<tournament id>)#<table no>' 6-max (real money)
//!   Seat #3 is the button`;
//! * seats: `Seat 1: <name> (<stack>, 0.74€ bounty)`;
//! * a busted small blind is **dead**: nobody posts it;
//! * raises read `raises <increment> to <total>`;
//! * there is no "uncalled bet returned" line: the uncalled part is included
//!   in what the bettor `collected` (sometimes as a "side pot"), and
//!   `Total pot` includes it — so `invested - collected` is the net result;
//! * `Total pot N | No rake` in tournaments.
//!
//! Cash games, Expresso and Go Fast are **not supported yet**: no sample
//! file has been provided. Their headers are rejected with a clear error.

pub mod summary;

use std::sync::LazyLock;

use lps_core_model::{
    assign_positions, card::parse_cards, Action, ActionKind, BettingLimit, Card, GameType, Hand, PlayerInHand,
    Street, TournamentRef, Unit, Variant,
};
use regex::Regex;

use crate::util::{parse_cents, parse_utc, strip_bom};
use crate::{Confidence, HandHistoryParser, ParseError, RawHand};

pub const ROOM: &str = "winamax";
const HEADER_PREFIX: &str = "Winamax Poker - ";

static HEADER_MTT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^Winamax Poker - Tournament "(?P<name>.*)" buyIn: (?P<bi>[0-9.]+)€ \+ (?P<fee>[0-9.]+)€ level: (?P<lvl>\d+) - HandId: #(?P<id>[0-9-]+) - Holdem no limit \((?P<ante>\d+)/(?P<sb>\d+)/(?P<bb>\d+)\) - (?P<date>\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}) UTC$"#,
    )
    .unwrap()
});
static TABLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Table: '(?P<table>.*)' (?P<max>\d+)-max \(real money\) Seat #(?P<btn>\d+) is the button$").unwrap()
});
static TABLE_TOURNEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>.*)\((?P<id>\d+)\)#(?P<no>\d+)$").unwrap());
static SEAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Seat (?P<seat>\d+): (?P<name>.+) \((?P<stack>\d+)(?:, (?P<bounty>[0-9.]+)€ bounty)?\)$").unwrap()
});
static DEALT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^Dealt to (?P<name>.+) \[(?P<cards>[^\]]+)\]$").unwrap());
static BOARD_LINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]").unwrap());
static TOTAL_POT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^Total pot (?P<pot>\d+) \| No rake$").unwrap());

/// The Winamax parser.
pub struct Winamax;

fn sniff_text(first_bytes: &[u8]) -> Option<&str> {
    let n = first_bytes.len().min(256);
    // Cut on a char boundary.
    let mut end = n;
    while end > 0 && std::str::from_utf8(&first_bytes[..end]).is_err() {
        end -= 1;
    }
    std::str::from_utf8(&first_bytes[..end]).ok().map(strip_bom)
}

impl HandHistoryParser for Winamax {
    fn room_id(&self) -> &'static str {
        ROOM
    }

    fn sniff(&self, first_bytes: &[u8]) -> Confidence {
        match sniff_text(first_bytes) {
            Some(t) if t.starts_with("Winamax Poker - Tournament summary") => Confidence::No,
            Some(t) if t.starts_with(HEADER_PREFIX) => Confidence::Certain,
            _ => Confidence::No,
        }
    }

    fn sniff_summary(&self, first_bytes: &[u8]) -> Confidence {
        match sniff_text(first_bytes) {
            Some(t) if t.starts_with("Winamax Poker - Tournament summary") => Confidence::Certain,
            _ => Confidence::No,
        }
    }

    fn parse_summary(&self, text: &str) -> Result<lps_core_model::TournamentSummary, ParseError> {
        summary::parse(text)
    }

    fn split<'a>(&self, text: &'a str) -> Vec<RawHand<'a>> {
        // Hand starts = beginning of a line starting with the header prefix.
        let mut starts = Vec::new();
        // A leading BOM is skipped so the first header still starts "a line".
        let mut line_start = text.len() - strip_bom(text).len();
        for (i, b) in text.bytes().enumerate() {
            if i == line_start && text[i..].starts_with(HEADER_PREFIX) {
                starts.push(i);
            }
            if b == b'\n' {
                line_start = i + 1;
            }
        }
        let mut out = Vec::with_capacity(starts.len());
        for (k, &s) in starts.iter().enumerate() {
            let e = starts.get(k + 1).copied().unwrap_or(text.len());
            let chunk = &text[s..e];
            let body = chunk.trim_end_matches(['\r', '\n']);
            // Complete = has a summary and is followed by a blank line
            // (Winamax writes two blank lines after every hand).
            let after = &chunk[body.len()..];
            let blank_lines = after.matches('\n').count();
            let complete = body.contains("*** SUMMARY ***") && (k + 1 < starts.len() || blank_lines >= 2);
            out.push(RawHand { text: body, start: s, end: e, complete });
        }
        out
    }

    fn parse(&self, raw: &RawHand<'_>) -> Result<Hand, ParseError> {
        HandBuilder::default().parse(raw.text)
    }
}

#[derive(Default)]
struct HandBuilder {
    players: Vec<PlayerInHand>,
    /// Indices into `players`, longest names first (names may contain spaces).
    by_name_len: Vec<usize>,
    actions: Vec<Action>,
    pot: i64,
    /// Chips put in by each player on the current street (blinds count, antes don't).
    street_in: Vec<i64>,
    street: Option<Street>,
    board: Vec<Card>,
    sb_seat: Option<u8>,
    bb_seat: Option<u8>,
    showdown: bool,
    total_pot: Option<i64>,
}

enum Section {
    Seats,
    Blinds,
    Street(Street),
    Showdown,
    Summary,
}

impl HandBuilder {
    fn parse(mut self, text: &str) -> Result<Hand, ParseError> {
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r')).enumerate().map(|(i, l)| (i + 1, l));

        let (n, header) = lines.next().ok_or_else(|| ParseError::whole("empty hand"))?;
        let header = strip_bom(header);
        let h = HEADER_MTT.captures(header).ok_or_else(|| {
            let msg = if header.starts_with(HEADER_PREFIX) && !header.contains("Tournament") {
                "Winamax cash games are not supported yet (no sample file provided)"
            } else {
                "unrecognised Winamax header"
            };
            ParseError::new(n, header, msg)
        })?;
        let num = |name: &str| -> i64 { h[name].parse().unwrap_or(0) };
        let money = |name: &str| parse_cents(&h[name]).ok_or_else(|| ParseError::new(n, header, format!("bad amount in {name}")));
        let buy_in_cents = money("bi")?;
        let fee_cents = money("fee")?;
        let played_at = parse_utc(&h["date"]).ok_or_else(|| ParseError::new(n, header, "bad date"))?;
        let (ante, sb, bb) = (num("ante"), num("sb"), num("bb"));
        let level = h["lvl"].parse().ok();
        let hand_no = h["id"].to_string();
        let tourney_name = h["name"].to_string();

        let (n, tline) = lines.next().ok_or_else(|| ParseError::whole("missing table line"))?;
        let t = TABLE.captures(tline).ok_or_else(|| ParseError::new(n, tline, "unrecognised table line"))?;
        let table_name = t["table"].to_string();
        let max_seats: u8 = t["max"].parse().map_err(|_| ParseError::new(n, tline, "bad max seats"))?;
        let button_seat: u8 = t["btn"].parse().map_err(|_| ParseError::new(n, tline, "bad button seat"))?;
        let (tourney_id, table_number) = match TABLE_TOURNEY.captures(&table_name) {
            Some(c) => (c["id"].to_string(), c["no"].parse().ok()),
            None => return Err(ParseError::new(n, tline, "tournament table name without (id)#number")),
        };

        let mut section = Section::Seats;
        for (n, line) in lines {
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("*** ") {
                section = self.section(n, line, rest)?;
                continue;
            }
            match section {
                Section::Seats => {
                    let c = SEAT.captures(line).ok_or_else(|| ParseError::new(n, line, "expected a seat line"))?;
                    let bounty_cents = match c.name("bounty") {
                        Some(b) => Some(parse_cents(b.as_str()).ok_or_else(|| ParseError::new(n, line, "bad bounty"))?),
                        None => None,
                    };
                    self.players.push(PlayerInHand {
                        seat: c["seat"].parse().map_err(|_| ParseError::new(n, line, "bad seat"))?,
                        name: c["name"].to_string(),
                        stack: c["stack"].parse().map_err(|_| ParseError::new(n, line, "bad stack"))?,
                        position: None,
                        hole_cards: vec![],
                        is_hero: false,
                        bounty_cents,
                        invested: 0,
                        collected: 0,
                        went_to_showdown: false,
                    });
                }
                Section::Blinds => {
                    if let Some(c) = DEALT.captures(line) {
                        let i = self.player(n, line, &c["name"])?;
                        self.players[i].is_hero = true;
                        self.players[i].hole_cards =
                            parse_cards(&c["cards"]).map_err(|e| ParseError::new(n, line, e.to_string()))?;
                    } else {
                        self.action_line(n, line, Street::Preflop, true)?;
                    }
                }
                Section::Street(street) => {
                    self.action_line(n, line, street, false)?;
                }
                Section::Showdown => self.showdown_line(n, line)?,
                Section::Summary => self.summary_line(n, line)?,
            }
        }

        self.drop_players_not_dealt_in();
        let seats: Vec<u8> = self.players.iter().map(|p| p.seat).collect();
        for (p, pos) in self.players.iter_mut().zip(assign_positions(&seats, button_seat, self.sb_seat, self.bb_seat)) {
            p.position = pos;
        }
        let total_pot = self.total_pot.ok_or_else(|| ParseError::whole("missing 'Total pot' line"))?;

        let hand = Hand {
            room: ROOM.into(),
            hand_no,
            table_name,
            game_type: GameType::Mtt,
            variant: Variant::Holdem,
            limit: BettingLimit::NoLimit,
            unit: Unit::Chips,
            currency: "EUR".into(),
            small_blind: sb,
            big_blind: bb,
            ante,
            straddle: None,
            level,
            tournament: Some(TournamentRef {
                id: tourney_id,
                name: tourney_name,
                buy_in_cents,
                fee_cents,
                table_number,
            }),
            max_seats,
            button_seat,
            players: self.players,
            board: self.board,
            actions: self.actions,
            total_pot,
            rake: 0,
            jackpot_fee: 0,
            showdown: self.showdown,
            played_at,
        };
        let problems = hand.check();
        if !problems.is_empty() {
            return Err(ParseError::whole(format!("inconsistent hand: {}", problems.join("; "))));
        }
        Ok(hand)
    }

    fn section(&mut self, n: usize, line: &str, rest: &str) -> Result<Section, ParseError> {
        let (title, cards) = match rest.split_once(" ***") {
            Some((t, c)) => (t, c.trim()),
            None => return Err(ParseError::new(n, line, "unrecognised section")),
        };
        let new_street = |s: &mut Self, street: Street| -> Result<Section, ParseError> {
            if s.by_name_len.is_empty() {
                s.index_names();
            }
            if street != Street::Preflop {
                // The last [...] holds the new card(s).
                let last = BOARD_LINE
                    .captures_iter(cards)
                    .last()
                    .ok_or_else(|| ParseError::new(n, line, "missing board cards"))?;
                let new = parse_cards(&last[1]).map_err(|e| ParseError::new(n, line, e.to_string()))?;
                s.board.extend(new);
                if s.board.len() != street.board_len() {
                    return Err(ParseError::new(n, line, "board size does not match street"));
                }
                s.street_in.iter_mut().for_each(|x| *x = 0);
            }
            s.street = Some(street);
            Ok(Section::Street(street))
        };
        match title {
            "ANTE/BLINDS" => {
                self.index_names();
                Ok(Section::Blinds)
            }
            "PRE-FLOP" => new_street(self, Street::Preflop),
            "FLOP" => new_street(self, Street::Flop),
            "TURN" => new_street(self, Street::Turn),
            "RIVER" => new_street(self, Street::River),
            "SHOW DOWN" => {
                self.showdown = true;
                Ok(Section::Showdown)
            }
            "SUMMARY" => Ok(Section::Summary),
            _ => Err(ParseError::new(n, line, format!("unknown section '{title}'"))),
        }
    }

    /// A player who just sat down is listed in the seats but not dealt in
    /// (no ante, no blind, no action, no cards): remove them from the hand.
    fn drop_players_not_dealt_in(&mut self) {
        let mut active = vec![false; self.players.len()];
        for a in &self.actions {
            active[a.player] = true;
        }
        for (i, p) in self.players.iter().enumerate() {
            if p.is_hero || !p.hole_cards.is_empty() || p.collected > 0 {
                active[i] = true;
            }
        }
        if active.iter().all(|&x| x) {
            return;
        }
        let mut remap = vec![usize::MAX; self.players.len()];
        let mut next = 0;
        for (i, &keep) in active.iter().enumerate() {
            if keep {
                remap[i] = next;
                next += 1;
            }
        }
        let mut i = 0;
        self.players.retain(|_| {
            i += 1;
            active[i - 1]
        });
        for a in &mut self.actions {
            a.player = remap[a.player];
        }
    }

    fn index_names(&mut self) {
        let mut idx: Vec<usize> = (0..self.players.len()).collect();
        idx.sort_by_key(|&i| std::cmp::Reverse(self.players[i].name.len()));
        self.by_name_len = idx;
        self.street_in = vec![0; self.players.len()];
    }

    fn player(&self, n: usize, line: &str, name: &str) -> Result<usize, ParseError> {
        self.players
            .iter()
            .position(|p| p.name == name)
            .ok_or_else(|| ParseError::new(n, line, format!("unknown player '{name}'")))
    }

    /// Splits "<player name> <rest>" using the known names (longest first).
    fn split_player<'l>(&self, n: usize, line: &'l str) -> Result<(usize, &'l str), ParseError> {
        for &i in &self.by_name_len {
            if let Some(rest) = line.strip_prefix(self.players[i].name.as_str()).and_then(|r| r.strip_prefix(' ')) {
                return Ok((i, rest));
            }
        }
        Err(ParseError::new(n, line, "line does not start with a known player"))
    }

    fn push(&mut self, player: usize, street: Street, kind: ActionKind, amount: i64, raise_to: Option<i64>, is_all_in: bool) {
        self.actions.push(Action { player, street, kind, amount, raise_to, is_all_in, pot_before: self.pot });
        self.pot += amount;
        self.players[player].invested += amount;
        if kind != ActionKind::PostAnte {
            self.street_in[player] += amount;
        }
    }

    fn action_line(&mut self, n: usize, line: &str, street: Street, blinds: bool) -> Result<(), ParseError> {
        let (p, rest) = self.split_player(n, line)?;
        let (rest, all_in) = match rest.strip_suffix(" and is all-in") {
            Some(r) => (r, true),
            None => (rest, false),
        };
        let amount = |s: &str| s.parse::<i64>().map_err(|_| ParseError::new(n, line, "bad amount"));
        let seat = self.players[p].seat;
        if blinds {
            if let Some(a) = rest.strip_prefix("posts ante ") {
                self.push(p, street, ActionKind::PostAnte, amount(a)?, None, all_in);
            } else if let Some(a) = rest.strip_prefix("posts small blind ") {
                self.sb_seat.get_or_insert(seat);
                self.push(p, street, ActionKind::PostSmallBlind, amount(a)?, None, all_in);
            } else if let Some(a) = rest.strip_prefix("posts big blind ") {
                self.bb_seat.get_or_insert(seat);
                self.push(p, street, ActionKind::PostBigBlind, amount(a)?, None, all_in);
            } else {
                return Err(ParseError::new(n, line, "unknown post"));
            }
            return Ok(());
        }
        match rest {
            "folds" => self.push(p, street, ActionKind::Fold, 0, None, false),
            "checks" => self.push(p, street, ActionKind::Check, 0, None, false),
            _ => {
                if let Some(a) = rest.strip_prefix("calls ") {
                    self.push(p, street, ActionKind::Call, amount(a)?, None, all_in);
                } else if let Some(a) = rest.strip_prefix("bets ") {
                    self.push(p, street, ActionKind::Bet, amount(a)?, None, all_in);
                } else if let Some(r) = rest.strip_prefix("raises ") {
                    let (_incr, to) = r.split_once(" to ").ok_or_else(|| ParseError::new(n, line, "raise without 'to'"))?;
                    let to = amount(to)?;
                    let added = to - self.street_in[p];
                    if added <= 0 {
                        return Err(ParseError::new(n, line, "raise does not add chips"));
                    }
                    self.push(p, street, ActionKind::Raise, added, Some(to), all_in);
                } else if let Some(a) = rest.strip_prefix("collected ") {
                    // Everyone else folded: no showdown section.
                    self.collect(n, line, p, a)?;
                } else {
                    return Err(ParseError::new(n, line, "unknown action"));
                }
            }
        }
        Ok(())
    }

    fn collect(&mut self, n: usize, line: &str, p: usize, rest: &str) -> Result<(), ParseError> {
        let (amt, from) = rest.split_once(" from ").ok_or_else(|| ParseError::new(n, line, "collected without 'from'"))?;
        let ok = from == "pot" || from == "main pot" || from.strip_prefix("side pot ").is_some_and(|k| k.parse::<u32>().is_ok());
        if !ok {
            return Err(ParseError::new(n, line, "unknown pot"));
        }
        self.players[p].collected += amt.parse::<i64>().map_err(|_| ParseError::new(n, line, "bad amount"))?;
        Ok(())
    }

    fn showdown_line(&mut self, n: usize, line: &str) -> Result<(), ParseError> {
        let (p, rest) = self.split_player(n, line)?;
        if let Some(r) = rest.strip_prefix("shows [") {
            let (cards, _desc) = r.split_once(']').ok_or_else(|| ParseError::new(n, line, "bad shows line"))?;
            let cards = parse_cards(cards).map_err(|e| ParseError::new(n, line, e.to_string()))?;
            let pl = &mut self.players[p];
            if !pl.hole_cards.is_empty() && pl.hole_cards != cards {
                return Err(ParseError::new(n, line, "shown cards differ from dealt cards"));
            }
            pl.hole_cards = cards;
            pl.went_to_showdown = true;
            Ok(())
        } else if let Some(r) = rest.strip_prefix("collected ") {
            self.collect(n, line, p, r)
        } else {
            Err(ParseError::new(n, line, "unknown showdown line"))
        }
    }

    fn summary_line(&mut self, n: usize, line: &str) -> Result<(), ParseError> {
        if let Some(c) = TOTAL_POT.captures(line) {
            self.total_pot = Some(c["pot"].parse().map_err(|_| ParseError::new(n, line, "bad pot"))?);
        } else if line.starts_with("Total pot ") {
            return Err(ParseError::new(n, line, "unsupported pot/rake format (no sample yet)"));
        } else if line.starts_with("Board: [") || line.starts_with("Seat ") {
            // Redundant with the action lines.
        } else {
            return Err(ParseError::new(n, line, "unknown summary line"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
