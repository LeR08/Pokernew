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

// ---- Phase 2, part 2 ----
export interface Session {
  start: number;
  end: number;
  hands: number;
  netBb: number;
  netCashCents: number;
  tournaments: number;
  vpipHands: number;
  pfrHands: number;
}

export interface Finding {
  rule: string;
  kind: "review" | "good";
  handId: number;
  playedAt: number;
  holeCards: string;
  position: string | null;
  stackBb: number;
  netBb: number;
  detail: Record<string, string | number>;
}

export interface SessionReview {
  session: Session | null;
  findings: Finding[];
  vpip: number | null;
  pfr: number | null;
  vpipOverall: number | null;
  pfrOverall: number | null;
  tiltWarning: boolean;
  worstHand: [number, number] | null;
  bestHand: [number, number] | null;
}

export interface LeakRow {
  stat: string;
  gameType: string;
  value: number | null;
  sample: number;
  ciLow: number | null;
  ciHigh: number | null;
  target: { min?: number; max?: number };
  status: "ok" | "possible" | "leak" | "too_few";
  direction: "low" | "high" | null;
  tooLow: I18nText | null;
  tooHigh: I18nText | null;
  bb100WhenDone: number | null;
  bb100WhenNot: number | null;
}

export interface LeaksResponse {
  rows: LeakRow[];
  stats: StatMeta[];
}

export interface GridCell {
  class: string;
  hands: number;
  netBb: number;
  vpip: number;
  pfr: number;
}

export interface Note {
  id: number;
  text: string;
  auto: boolean;
  createdAt: number;
}

export interface PlayerNotes {
  playerId: number;
  name: string;
  colour: string | null;
  notes: Note[];
}

export interface Tx {
  id?: number;
  room: string;
  at: number;
  kind: "deposit" | "withdrawal" | "bonus" | "adjustment";
  amountCents: number;
  note: string;
}

export interface Bankroll {
  t: number[];
  balanceEur: number[];
  depositsCents: number;
  withdrawalsCents: number;
  cashCents: number;
  tournamentsCents: number;
  balanceCents: number;
  avgBuyInCents: number | null;
}
