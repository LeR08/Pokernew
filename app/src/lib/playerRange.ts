/** Ranges estimated from a tracked player's stats, and which one fits a line played preflop. */
import type { Hand, ReportRow } from "./tracker";
import { NUM_COMBOS, topPercent } from "./range";

export type RangeKind = "pfr" | "call" | "three_bet" | "vpip";
export const RANGE_KINDS: RangeKind[] = ["pfr", "call", "three_bet", "vpip"];

/** The top X % of hands for the player's PFR, cold call (VPIP − PFR), 3-bet or VPIP. */
export function estimatedRange(row: ReportRow, kind: RangeKind): Float32Array | null {
  const v = (id: string) => row.stats.find((s) => s.id === id)?.value ?? null;
  const w = new Float32Array(NUM_COMBOS);
  const set = (pct: number, val: number) => topPercent(Math.min(100, Math.max(0, pct * 100))).forEach((i) => (w[i] = val));
  const vpip = v("vpip");
  const pfr = v("pfr");
  const tb = v("three_bet");
  if (kind === "vpip" && vpip != null) set(vpip, 1);
  else if (kind === "pfr" && pfr != null) set(pfr, 1);
  else if (kind === "three_bet" && tb != null) set(tb, 1);
  else if (kind === "call" && vpip != null && pfr != null) {
    set(vpip, 1);
    topPercent(Math.max(0, pfr * 100)).forEach((i) => (w[i] = 0));
  } else return null;
  return w;
}

/**
 * Which estimated range fits what player `idx` did preflop in `hand`:
 * re-raise = 3-bet range, first raise = open (PFR), call = calling range,
 * nothing voluntary (big blind check) = VPIP range.
 */
export function preflopKind(hand: Hand, idx: number): RangeKind {
  let raises = 0;
  let kind: RangeKind = "vpip";
  for (const a of hand.actions) {
    if (a.street !== "preflop") continue;
    if (a.kind === "raise" || a.kind === "bet") {
      if (a.player === idx) kind = raises >= 1 ? "three_bet" : "pfr";
      raises += 1;
    } else if (a.kind === "call" && a.player === idx && kind === "vpip") kind = "call";
  }
  return kind;
}
