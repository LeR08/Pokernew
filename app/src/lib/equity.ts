/** Types of the Equity Lab commands (app/src-tauri/src/equity_commands.rs). */

export type EquityMode = "auto" | "exhaustive" | "monte_carlo";

export interface CalcRequest {
  /** One hand ("AhKd") or range per player. */
  players: string[];
  board: string;
  dead: string;
  mode: EquityMode;
  /** 95 % half-width (0.001 = ±0.1 %). */
  precision?: number;
  maxMs?: number;
}

export interface ComboEquity {
  combo: number;
  weight: number;
  equity: number;
}

export interface PlayerResult {
  equity: number;
  win: number;
  tie: number;
  combos: number;
  perCombo: ComboEquity[];
}

export interface EquityResult {
  players: PlayerResult[];
  method: Exclude<EquityMode, "auto">;
  samples: number;
  elapsedMs: number;
  margin: number;
}

export interface BreakdownRow {
  id: string;
  combos: number;
  share: number;
}

export interface Breakdown {
  total: number;
  made: BreakdownRow[];
  draws: BreakdownRow[];
}

export const MADE_IDS = [
  "straight_flush",
  "quads",
  "full_house",
  "flush",
  "straight",
  "set",
  "trips",
  "two_pair",
  "overpair",
  "top_pair",
  "pp_below_top",
  "middle_pair",
  "weak_pair",
  "ace_high",
  "no_made_hand",
] as const;

export const DRAW_IDS = ["flush_draw", "oesd", "gutshot", "backdoor_flush", "backdoor_straight", "overcards"] as const;

export interface FlopFilter {
  suits?: "monotone" | "two_tone" | "rainbow" | null;
  pairing?: "unpaired" | "paired" | "trips" | null;
  highMin?: number | null;
  highMax?: number | null;
  connected?: boolean | null;
}

export interface FlopRow {
  cards: string;
  weight: number;
  /** Shares in the order of MADE_IDS / DRAW_IDS. */
  made: number[];
  draws: number[];
}

export interface TextureReport {
  classes: number;
  flops: number;
  made: BreakdownRow[];
  draws: BreakdownRow[];
  rows: FlopRow[];
}
