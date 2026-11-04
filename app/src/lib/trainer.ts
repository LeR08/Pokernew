/** Types of the trainer commands (crates/trainer, app/src-tauri/src/trainer_commands.rs). */
import type { ActionInfo, SolveParams, Tree } from "./solver";

export type DrillMode = "node" | "street" | "hand";

export interface Step {
  who: string;
  label: string;
}

export interface Question {
  path: number[];
  history: Step[];
  board: string[];
  player: 0 | 1;
  pot: number;
  combo: string;
  hand: number;
  actions: ActionInfo[];
  nodeKey: string;
}

export interface Answer {
  chosen: number;
  freqs: number[];
  evs: number[];
  best: number;
  lossBb: number;
  lossPct: number;
  equity: number;
}

export type Grade = "best" | "correct" | "inaccuracy" | "mistake" | "blunder";

export interface Thresholds {
  correct: number;
  inaccuracy: number;
  mistake: number;
}

export const DEFAULT_THRESHOLDS: Thresholds = { correct: 1, inaccuracy: 3, mistake: 8 };

/** Same rule as crates/trainer/src/grade.rs. */
export function grade(lossPct: number, best: boolean, t: Thresholds): Grade {
  if (best || lossPct <= 0.05) return "best";
  if (lossPct <= t.correct) return "correct";
  if (lossPct <= t.inaccuracy) return "inaccuracy";
  if (lossPct <= t.mistake) return "mistake";
  return "blunder";
}

/** One answered question, as stored in the history. */
export interface HistoryEntry {
  at: number;
  kind: "postflop" | "preflop" | "pushfold";
  nodeKey: string;
  grade: Grade;
  lossBb: number;
  lossPct: number;
}

export interface Decision {
  street: string;
  nodeKey: string;
  played: string;
  best: string;
  lossBb: number;
  lossPct: number;
  approximate: boolean;
}

export interface HandAnalysis {
  heroCards: string;
  heroRange: string;
  villainRange: string;
  decisions: Decision[];
  totalLossBb: number;
  spot: { board: string; pot: number; stack: number };
}

export interface AnalysisRow {
  handId: number;
  playedAt: number;
  analysis: HandAnalysis | null;
  error: string | null;
}

export interface AnalysisProgress {
  total: number;
  done: number;
  running: boolean;
  waitingForTables: boolean;
  rows: AnalysisRow[];
}

export type AnalysisTree = Tree;
export type AnalysisParams = SolveParams;
