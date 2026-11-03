//! The normalized hand: one struct for every room.

use serde::{Deserialize, Serialize};

use crate::card::Card;
use crate::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameType {
    Cash,
    /// Multi-table tournament (includes knockout/bounty formats).
    Mtt,
    /// Sit & Go.
    Sng,
    /// Spin & Go / Expresso (lottery prize pool, 3-handed).
    Spin,
}

impl GameType {
    pub fn as_str(self) -> &'static str {
        match self {
            GameType::Cash => "cash",
            GameType::Mtt => "mtt",
            GameType::Sng => "sng",
            GameType::Spin => "spin",
        }
    }
    pub fn is_tournament(self) -> bool {
        self != GameType::Cash
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    Holdem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BettingLimit {
    NoLimit,
    PotLimit,
    FixedLimit,
}

/// What the integer amounts of a hand count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// Hundredths of the currency (cash games).
    Cents,
    /// Tournament chips.
    Chips,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
}

impl Street {
    pub fn as_str(self) -> &'static str {
        match self {
            Street::Preflop => "preflop",
            Street::Flop => "flop",
            Street::Turn => "turn",
            Street::River => "river",
        }
    }
    /// Board cards visible on this street.
    pub fn board_len(self) -> usize {
        match self {
            Street::Preflop => 0,
            Street::Flop => 3,
            Street::Turn => 4,
            Street::River => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    PostAnte,
    PostSmallBlind,
    PostBigBlind,
    PostStraddle,
    /// Dead blind / extra post (cash games).
    PostDead,
    Fold,
    Check,
    Call,
    Bet,
    Raise,
}

impl ActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionKind::PostAnte => "post_ante",
            ActionKind::PostSmallBlind => "post_sb",
            ActionKind::PostBigBlind => "post_bb",
            ActionKind::PostStraddle => "post_straddle",
            ActionKind::PostDead => "post_dead",
            ActionKind::Fold => "fold",
            ActionKind::Check => "check",
            ActionKind::Call => "call",
            ActionKind::Bet => "bet",
            ActionKind::Raise => "raise",
        }
    }
    pub fn is_post(self) -> bool {
        matches!(
            self,
            ActionKind::PostAnte | ActionKind::PostSmallBlind | ActionKind::PostBigBlind | ActionKind::PostStraddle | ActionKind::PostDead
        )
    }
    /// Voluntary aggressive action.
    pub fn is_aggressive(self) -> bool {
        matches!(self, ActionKind::Bet | ActionKind::Raise)
    }
}

/// One action. `amount` is what the player puts in the pot *with this
/// action* (a raise "60 to 100" after a 40 blind adds 60).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    /// Index into [`Hand::players`].
    pub player: usize,
    pub street: Street,
    pub kind: ActionKind,
    pub amount: i64,
    /// For raises: the total the player raised to on this street.
    pub raise_to: Option<i64>,
    pub is_all_in: bool,
    /// Pot (all streets, including antes) before this action.
    pub pot_before: i64,
}

impl Action {
    pub fn pot_after(&self) -> i64 {
        self.pot_before + self.amount
    }
}

/// A player dealt into the hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInHand {
    pub seat: u8,
    pub name: String,
    /// Stack at the start of the hand.
    pub stack: i64,
    pub position: Option<Position>,
    /// Hole cards when known (hero, or shown). Length is not fixed so that
    /// Omaha variants fit later.
    pub hole_cards: Vec<Card>,
    pub is_hero: bool,
    /// Knockout bounty on this player's head, in cents.
    pub bounty_cents: Option<i64>,
    /// Total put into the pot.
    pub invested: i64,
    /// Total collected from the pot(s).
    pub collected: i64,
    /// Reached showdown (shown or mucked at showdown).
    pub went_to_showdown: bool,
}

impl PlayerInHand {
    pub fn net_won(&self) -> i64 {
        self.collected - self.invested
    }
}

/// A player seated at the table but not dealt into this hand (just arrived,
/// waiting for the big blind…). Not part of the hand; kept for the HUD.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitingPlayer {
    pub seat: u8,
    pub name: String,
    pub stack: i64,
}

/// Tournament the hand belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TournamentRef {
    pub id: String,
    pub name: String,
    /// Buy-in going to the prize pool (prize + bounty part), in cents.
    pub buy_in_cents: i64,
    /// Fee (rake) part of the buy-in, in cents.
    pub fee_cents: i64,
    pub table_number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hand {
    pub room: String,
    /// Room hand id, unique per room.
    pub hand_no: String,
    pub table_name: String,
    pub game_type: GameType,
    pub variant: Variant,
    pub limit: BettingLimit,
    pub unit: Unit,
    /// ISO 4217 code of the money involved (buy-in or cash stakes), e.g. "EUR".
    pub currency: String,
    pub small_blind: i64,
    pub big_blind: i64,
    pub ante: i64,
    pub straddle: Option<i64>,
    /// Blind level number (tournaments).
    pub level: Option<u32>,
    pub tournament: Option<TournamentRef>,
    pub max_seats: u8,
    pub button_seat: u8,
    pub players: Vec<PlayerInHand>,
    /// Seated but not dealt in.
    #[serde(default)]
    pub waiting: Vec<WaitingPlayer>,
    /// Board cards in order (flop, turn, river).
    pub board: Vec<Card>,
    pub actions: Vec<Action>,
    /// Total pot as reported by the room (includes uncalled bets when the
    /// room reports it that way, as Winamax does).
    pub total_pot: i64,
    pub rake: i64,
    pub jackpot_fee: i64,
    pub showdown: bool,
    /// Unix time, seconds (UTC).
    pub played_at: i64,
}

impl Hand {
    pub fn hero(&self) -> Option<&PlayerInHand> {
        self.players.iter().find(|p| p.is_hero)
    }

    pub fn player_index(&self, name: &str) -> Option<usize> {
        self.players.iter().position(|p| p.name == name)
    }

    /// Streets that were actually reached (board dealt).
    pub fn last_street(&self) -> Street {
        match self.board.len() {
            0..=2 => Street::Preflop,
            3 => Street::Flop,
            4 => Street::Turn,
            _ => Street::River,
        }
    }

    /// Consistency checks every parser must pass. Returns the list of
    /// violated rules (empty = consistent).
    pub fn check(&self) -> Vec<String> {
        let mut errs = Vec::new();
        let invested: i64 = self.players.iter().map(|p| p.invested).sum();
        let collected: i64 = self.players.iter().map(|p| p.collected).sum();
        if invested != collected + self.rake + self.jackpot_fee {
            errs.push(format!(
                "money not conserved: invested {invested} != collected {collected} + rake {} + jackpot {}",
                self.rake, self.jackpot_fee
            ));
        }
        if self.total_pot != invested {
            errs.push(format!("total pot {} != sum invested {invested}", self.total_pot));
        }
        for p in &self.players {
            if p.invested > p.stack {
                errs.push(format!("{} invested {} > stack {}", p.name, p.invested, p.stack));
            }
            if p.position.is_none() {
                errs.push(format!("{} has no position", p.name));
            }
        }
        if self.players.iter().filter(|p| p.is_hero).count() > 1 {
            errs.push("more than one hero".into());
        }
        if !matches!(self.board.len(), 0 | 3 | 4 | 5) {
            errs.push(format!("board has {} cards", self.board.len()));
        }
        let mut seen = std::collections::HashSet::new();
        for c in self.board.iter().chain(self.players.iter().flat_map(|p| p.hole_cards.iter())) {
            if !seen.insert(*c) {
                errs.push(format!("card {c} appears twice"));
            }
        }
        errs
    }
}

/// Result line of a tournament for the hero (from a room's summary file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TournamentSummary {
    pub room: String,
    pub id: String,
    pub name: String,
    pub hero: String,
    /// Part of the buy-in going to the regular prize pool, in cents.
    pub buy_in_prize_cents: i64,
    /// Part of the buy-in going to bounties (knockout), in cents.
    pub buy_in_bounty_cents: i64,
    pub fee_cents: i64,
    pub registered_players: Option<u32>,
    /// Room-specific labels, kept verbatim ("tt", "knockout", "turbo"…).
    pub mode: Option<String>,
    pub kind: Option<String>,
    pub speed: Option<String>,
    pub prize_pool_cents: Option<i64>,
    pub started_at: Option<i64>,
    pub duration_secs: Option<i64>,
    pub finish_position: Option<u32>,
    pub won_cents: i64,
    pub bounty_won_cents: i64,
    /// Value of tickets won (satellites), in cents.
    pub ticket_won_cents: i64,
    /// This entry busted during late registration (a re-entry may follow).
    pub late_registration: bool,
}
