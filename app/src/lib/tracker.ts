/** Types of the tracker/replayer API (mirrors crates/stats-engine and core-model). */

export type GameType = "cash" | "mtt" | "sng" | "spin";
export type Group = "none" | "position" | "game_type" | "month" | "account" | "stake";

export interface Filter {
  from?: number | null;
  to?: number | null;
  gameTypes?: string[];
  rooms?: string[];
  accounts?: string[];
  positions?: string[];
  tournament?: number | null;
}

export interface StatValue {
  id: string;
  value: number | null;
  sample: number;
}

export interface ReportRow {
  key: string;
  hands: number;
  netBb: number;
  netCashCents: number;
  bbPer100: number | null;
  stats: StatValue[];
}

export type I18nText = string | { fr: string; en: string };

export interface ColourRange {
  min?: number;
  max?: number;
  colour: string;
}

export interface StatMeta {
  id: string;
  name: string;
  format: "pct0" | "pct1" | "dec1" | "dec2" | "int" | "bb100" | string;
  description: I18nText | null;
  colourRanges: ColourRange[];
  minSample: number | null;
  file: string;
}

export interface ReportResponse {
  rows: ReportRow[];
  stats: StatMeta[];
  problems: { id: string; file: string; message: string }[];
}

export interface Graph {
  x: number[];
  t: number[];
  totalBb: number[];
  showdownBb: number[];
  nonShowdownBb: number[];
  cashEur: number[];
  hands: number;
}

export interface TournamentRow {
  id: number;
  name: string;
  gameType: string;
  startedAt: number | null;
  hands: number;
  entries: number;
  costCents: number;
  wonCents: number | null;
  profitCents: number | null;
  finishPosition: number | null;
  registeredPlayers: number | null;
  netBb: number;
}

export interface TournamentTotals {
  tournaments: number;
  withResult: number;
  costCents: number;
  wonCents: number;
  profitCents: number;
  roi: number | null;
  itm: number | null;
  curveEur: number[];
  curveT: number[];
}

export interface TournamentsResponse {
  rows: TournamentRow[];
  totals: TournamentTotals;
}

export interface HandRow {
  handId: number;
  playedAt: number;
  gameType: string;
  tableName: string;
  account: string;
  position: string | null;
  holeCards: string;
  board: string;
  stackBb: number;
  netWon: number;
  netWonBb: number;
  unit: "cents" | "chips";
  showdown: boolean;
}

export interface HandsResponse {
  rows: HandRow[];
  total: number;
}

export interface StatDef {
  name: string;
  formula: string;
  where?: string;
  format?: string;
  sample?: string;
  min_sample?: number;
  description?: I18nText;
  colour_ranges?: ColourRange[];
}

export interface Preview {
  value: number | null;
  sample: number;
  hands: number;
}

// ---- Hand (replayer), snake_case as serialized by Rust ----
export type Street = "preflop" | "flop" | "turn" | "river";
export type ActionKind =
  | "post_ante"
  | "post_small_blind"
  | "post_big_blind"
  | "post_straddle"
  | "post_dead"
  | "fold"
  | "check"
  | "call"
  | "bet"
  | "raise";

export interface Action {
  player: number;
  street: Street;
  kind: ActionKind;
  amount: number;
  raise_to: number | null;
  is_all_in: boolean;
  pot_before: number;
}

export interface PlayerInHand {
  seat: number;
  name: string;
  stack: number;
  position: string | null;
  hole_cards: string[];
  is_hero: boolean;
  bounty_cents: number | null;
  invested: number;
  collected: number;
  went_to_showdown: boolean;
}

export interface Hand {
  room: string;
  hand_no: string;
  table_name: string;
  game_type: GameType;
  unit: "cents" | "chips";
  currency: string;
  small_blind: number;
  big_blind: number;
  ante: number;
  level: number | null;
  tournament: { id: string; name: string; buy_in_cents: number; fee_cents: number } | null;
  max_seats: number;
  button_seat: number;
  players: PlayerInHand[];
  board: string[];
  actions: Action[];
  total_pot: number;
  rake: number;
  showdown: boolean;
  played_at: number;
}
