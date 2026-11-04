/** Types of the preflop solver (crates/preflop, app/src-tauri/src/preflop_commands.rs). */

export interface PreflopConfig {
  /** bb, preflop order (SB and BB last). */
  stacks: number[];
  sb: number;
  bb: number;
  ante: number;
  open: number;
  threeBet: number;
  fourBet: number;
  allowLimp: boolean;
  allowAllin: boolean;
  realizationIp: number;
  realizationOop: number;
}

export type ActKind = "fold" | "check" | "call" | "limp" | "raise" | "all_in";

export interface Act {
  kind: ActKind;
  to: number;
}

export type End = { type: "fold"; winner: number } | { type: "showdown"; p: number; q: number } | { type: "flop"; p: number; q: number };

export interface PfNodeOut {
  id: number;
  player: number | null;
  actions: Act[];
  children: number[];
  end: End | null;
  pot: number;
  /** [action][class] */
  freq: number[][];
  /** acting player's own reach per class */
  reach: number[];
  prob: number;
}

export interface PreflopSolution {
  config: PreflopConfig;
  nodes: PfNodeOut[];
  values: number[];
  exploitability: number;
  iterations: number;
  history: [number, number][];
}

export const DEFAULT_PREFLOP: PreflopConfig = {
  stacks: [100, 100],
  sb: 0.5,
  bb: 1,
  ante: 0,
  open: 2.5,
  threeBet: 3,
  fourBet: 2.2,
  allowLimp: true,
  allowAllin: true,
  realizationIp: 1,
  realizationOop: 1,
};
