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
  who?: "hero" | "opponents";
  player?: string | null;
  handClasses?: string[];
  potTypes?: string[];
  stackMin?: number | null;
  stackMax?: number | null;
  street?: "flop" | "turn" | "river" | null;
  texture?: number | null;
  showdown?: boolean | null;
  allinPreflop?: boolean | null;
  buyInMin?: number | null;
  buyInMax?: number | null;
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

export interface OpponentsResponse {
  rows: ReportRow[];
  pool: ReportRow | null;
  stats: StatMeta[];
}

/** Flop texture flags (crates/core-model/src/derived.rs). */
export const TEXTURE = { paired: 1, monotone: 2, two_tone: 4, rainbow: 8, connected: 16, high: 32, low: 64 } as const;

const RANKS = "AKQJT98765432";
/** All 169 classes. */
export const ALL_CLASSES: string[] = (() => {
  const out: string[] = [];
  for (let i = 0; i < 13; i++)
    for (let j = 0; j < 13; j++) out.push(i === j ? RANKS[i] + RANKS[i] : i < j ? `${RANKS[i]}${RANKS[j]}s` : `${RANKS[j]}${RANKS[i]}o`);
  return out;
})();

export const HAND_GROUPS: Record<string, (c: string) => boolean> = {
  pairs: (c) => c.length === 2,
  suited: (c) => c.endsWith("s"),
  offsuit: (c) => c.endsWith("o"),
  broadway: (c) => "AKQJT".includes(c[0]) && "AKQJT".includes(c[1]),
  ace: (c) => c[0] === "A",
  suited_connectors: (c) => c.endsWith("s") && RANKS.indexOf(c[1]) - RANKS.indexOf(c[0]) === 1,
};

/** Parses "AKs, QQ, 72o" into valid classes (unknown entries ignored). */
export function parseClasses(text: string): string[] {
  const set = new Set(ALL_CLASSES);
  return text
    .split(/[\s,;]+/)
    .map((x) => (x.length === 3 ? x.slice(0, 2).toUpperCase() + x[2].toLowerCase() : x.toUpperCase()))
    .filter((x) => set.has(x));
}

// ---- Phase 3: HUD ----
export interface WindowInfo {
  id: number;
  title: string;
  class: string;
  process: string;
  client: { x: number; y: number; w: number; h: number };
  scale: number;
  minimized: boolean;
  visible: boolean;
  simulated: boolean;
}

export interface TableWindow {
  window: WindowInfo;
  table: { room: string; tableName: string; tournamentId: string | null; tableNumber: number | null; maxSeats: number; lastHandAt: number };
  matchedBy: string;
}

export interface HudStatus {
  supported: boolean;
  visible: boolean;
  edit: boolean;
  playMode: boolean;
  unlockIn: number;
  tables: TableWindow[];
  windows: WindowInfo[];
}

export interface HudSeat {
  seat: number;
  slot: number;
  name: string;
  isHero: boolean;
  position: string | null;
  hands: number;
  stats: StatValue[];
  colour: string | null;
  notes: string[];
}

export interface HudLayout {
  name: string;
  opacity?: number;
  font_scale?: number;
  show_hero?: boolean;
  min_hands?: number;
  rows: string[][];
  popup?: string[];
  seats: Record<string, [number, number][]>;
}

export interface OverlayData {
  table: { room: string; tableName: string; maxSeats: number; lastHandAt: number; seats: HudSeat[] } | null;
  layout: HudLayout;
  layoutId: string;
  edit: boolean;
  simulated: boolean;
  stats: StatMeta[];
}
