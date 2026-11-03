//! Hand-history parsers.
//!
//! Each room is one module implementing [`HandHistoryParser`]; adding a room
//! means adding one file and registering it in [`registry`]. Formats are
//! written **only from real sample files** stored in `fixtures/<room>/`,
//! never guessed: an unknown line is a [`ParseError`] that shows up in the
//! import log (with a "copy as test case" button), not a silent skip.

pub mod install;
pub mod util;
pub mod winamax;

use lps_core_model::{Hand, TournamentSummary};

/// How sure a parser is that a file is in its format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    No,
    Maybe,
    Certain,
}

/// One hand's text inside a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawHand<'a> {
    pub text: &'a str,
    /// Byte offset of the hand in the file text.
    pub start: usize,
    /// Byte offset right after the hand (and its separator).
    pub end: usize,
    /// False when the hand is still being written (end of a live file).
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("line {line_no}: {message} — «{line}»")]
pub struct ParseError {
    /// 1-based line number inside the hand (0 = whole hand).
    pub line_no: usize,
    pub line: String,
    pub message: String,
}

impl ParseError {
    pub fn new(line_no: usize, line: &str, message: impl Into<String>) -> Self {
        Self { line_no, line: line.to_string(), message: message.into() }
    }
    pub fn whole(message: impl Into<String>) -> Self {
        Self { line_no: 0, line: String::new(), message: message.into() }
    }
}

/// Parser plugin interface (see the spec, "Hand history import").
pub trait HandHistoryParser: Send + Sync {
    fn room_id(&self) -> &'static str;
    /// Format autodetection from the first bytes of a file.
    fn sniff(&self, first_bytes: &[u8]) -> Confidence;
    /// One file -> many hands.
    fn split<'a>(&self, text: &'a str) -> Vec<RawHand<'a>>;
    fn parse(&self, raw: &RawHand<'_>) -> Result<Hand, ParseError>;

    /// Tournament summary files (optional per room).
    fn sniff_summary(&self, _first_bytes: &[u8]) -> Confidence {
        Confidence::No
    }
    /// One summary per entry: a file may hold several (re-entries).
    fn parse_summaries(&self, _text: &str) -> Result<Vec<TournamentSummary>, ParseError> {
        Err(ParseError::whole("this room has no summary format"))
    }
}

/// What a file contains, according to the best parser.
pub enum Detected {
    Hands(&'static dyn HandHistoryParser),
    Summary(&'static dyn HandHistoryParser),
}

/// All parsers of this build.
pub fn registry() -> &'static [&'static dyn HandHistoryParser] {
    static WINAMAX: winamax::Winamax = winamax::Winamax;
    static ALL: [&dyn HandHistoryParser; 1] = [&WINAMAX];
    &ALL
}

pub fn parser_for_room(room: &str) -> Option<&'static dyn HandHistoryParser> {
    registry().iter().copied().find(|p| p.room_id() == room)
}

/// Picks the parser for a file from its first bytes.
pub fn detect(first_bytes: &[u8]) -> Option<Detected> {
    let mut best: Option<(Confidence, Detected)> = None;
    for &p in registry() {
        for (c, d) in [(p.sniff_summary(first_bytes), Detected::Summary(p)), (p.sniff(first_bytes), Detected::Hands(p))] {
            if c > Confidence::No && best.as_ref().is_none_or(|(b, _)| c > *b) {
                best = Some((c, d));
            }
        }
    }
    best.map(|(_, d)| d)
}
