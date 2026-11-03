/** Types of the postflop solver commands (app/src-tauri/src/solver_commands.rs, crates/solver). */

export interface Sizes {
  bet: string;
  raise: string;
}
export interface StreetSizes {
  oop: Sizes;
  ip: Sizes;
  donk: string;
}
export interface Tree {
  flop: StreetSizes;
  turn: StreetSizes;
  river: StreetSizes;
  add_allin_threshold: number;
  force_allin_threshold: number;
  merging_threshold: number;
  max_raises: number;
}
export interface SpotConfig {
  oopRange: string;
  ipRange: string;
  board: string;
  pot: number;
  stack: number;
  rake: number;
  rakeCap: number;
  tree: Tree;
}
export interface SolveParams {
  targetPct: number;
  maxIterations: number;
  threads: number;
  compression: boolean;
  memoryCapMb: number;
}
export interface Lock {
  path: number[];
  strategy: number[];
}
export interface Estimate {
  memory: number;
  memoryCompressed: number;
  treeNodes: number;
  oopHands: number;
  ipHands: number;
}
export type JobState = "building" | "solving" | "paused" | "done" | "cancelled" | "failed";
export interface Progress {
  state: JobState;
  iteration: number;
  maxIterations: number;
  exploitabilityPct: number;
  history: [number, number, number][];
  elapsedMs: number;
  memoryMb: number;
  error: string | null;
}
export interface CacheEntry {
  hash: string;
  name: string;
  createdAt: number;
  spot: SpotConfig;
  locks: Lock[];
  exploitabilityPct: number;
  iterations: number;
  bytes: number;
}
export interface ActionInfo {
  kind: "fold" | "check" | "call" | "bet" | "raise" | "allin" | "chance";
  amount: number;
}
export interface HandInfo {
  combo: number;
  cards: string;
  weight: number;
  strategy: number[];
  ev: number;
  equity: number;
}
export interface ClassCell {
  cell: number;
  weight: number;
  freq: number[];
  ev: number;
  equity: number;
}
export interface PlayerView {
  hands: HandInfo[];
  grid: ClassCell[];
  ev: number;
  equity: number;
  eqr: number;
}
export interface NodeView {
  path: number[];
  board: string[];
  kind: "action" | "chance" | "terminal";
  player: 0 | 1 | null;
  actions: ActionInfo[];
  totals: number[];
  cards: string[];
  pot: number;
  bets: [number, number];
  locked: boolean;
  players: PlayerView[];
}

const s = (bet: string, raise: string): StreetSizes => ({ oop: { bet, raise }, ip: { bet, raise }, donk: "" });

export const DEFAULT_TREE: Tree = {
  flop: s("33%, 75%", "3x"),
  turn: s("33%, 75%", "3x"),
  river: s("33%, 75%", "3x"),
  add_allin_threshold: 1.5,
  force_allin_threshold: 0.15,
  merging_threshold: 0.1,
  max_raises: 1,
};

export const DEFAULT_PARAMS: SolveParams = { targetPct: 0.5, maxIterations: 1000, threads: 0, compression: true, memoryCapMb: 8192 };

export function actionLabel(a: ActionInfo, t: (k: string, o?: Record<string, unknown>) => string): string {
  return a.amount > 0 ? t(`solver.action.${a.kind}_n`, { n: +a.amount.toFixed(2) }) : t(`solver.action.${a.kind}`);
}
