/** Types of the ICM & push/fold commands (app/src-tauri/src/icm_commands.rs, crates/icm). */

export interface IcmResult {
  icm: number[];
  chipEv: number[];
  bubble: number[][];
  risk: number[][];
  prizePool: number;
}

export interface Spot {
  /** Chips in preflop order: [first to act, …, SB, BB]. */
  stacks: number[];
  sb: number;
  bb: number;
  ante: number;
  bbAnte: boolean;
  others: number[];
  /** Empty: chip EV. */
  payouts: number[];
  bounties: number[];
  fgsDepth: number;
}

export type Decision = { kind: "push"; p: number } | { kind: "call"; p: number; q: number };

export interface SolveOptions {
  maxIterations?: number;
  target?: number;
  locks?: [Decision, number[]][];
}

export interface PfNode {
  decision: Decision;
  freq: number[];
  evAct: number[];
  evFold: number[];
  reach: number;
}

export interface Solution {
  nodes: PfNode[];
  values: number[];
  exploitability: number;
  iterations: number;
  unit: "chips" | "money";
}

export type Choice = "push" | "call" | "fold";

export interface Review {
  spot: Spot;
  names: string[];
  positions: string[];
  hero: number;
  pusher: number | null;
  heroCards: string;
  heroClass: string;
  played: Choice;
  best: Choice;
  chipsAct: number;
  chipsFold: number;
  moneyAct: number | null;
  moneyFold: number | null;
  lossChips: number;
  lossMoney: number | null;
  effectiveBb: number;
}

export interface ReviewRequest {
  payouts: number[];
  others: number[];
  useBounties: boolean;
  bountyShare?: number;
  maxBb?: number;
  fgsDepth?: number;
}

export interface BatchRow {
  handId: number;
  playedAt: number;
  level: number | null;
  review: Review;
}

/** A saved payout structure (payouts/*.toml). */
export interface Payouts {
  name: string;
  players?: number;
  unit: "money" | "percent";
  prize_pool?: number;
  places: number[];
}

/** Prizes in money. */
export function prizes(p: Payouts, prizePool?: number): number[] {
  if (p.unit === "money") return p.places;
  const pool = prizePool ?? p.prize_pool ?? 100;
  return p.places.map((x) => (x / 100) * pool);
}

/** Position names in preflop order for n players (heads-up: SB, BB). */
export function positionNames(n: number): string[] {
  const tail = ["BTN", "CO", "HJ", "LJ", "MP", "UTG2", "UTG1", "UTG"];
  if (n === 2) return ["SB", "BB"];
  return [...tail.slice(0, n - 2).reverse(), "SB", "BB"];
}

export const decisionKey = (d: Decision) => (d.kind === "push" ? `push-${d.p}` : `call-${d.p}-${d.q}`);
