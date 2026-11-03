/**
 * Browser mock of the solver commands: a fake solve that progresses over a
 * second, and synthetic strategies (stronger hands bet more). Enough to
 * exercise the UI; the real engine is crates/solver.
 */
import type { Backend } from "./backend";
import { BackendError } from "./backend";
import type { Breakdown } from "./equity";
import { DRAW_IDS, MADE_IDS } from "./equity";
import { cellOf, comboStr, COMBOS, NUM_COMBOS, parseCards, RANKED_CELLS, tryParseRange } from "./range";
import type { ActionInfo, BatchProgress, CacheEntry, Lock, NodeView, PlayerView, Progress, SpotConfig } from "./solver";

type SolverPart = Pick<
  Backend,
  | "solverEstimate"
  | "solverStart"
  | "solverProgress"
  | "solverControl"
  | "solverFinish"
  | "solverCacheList"
  | "solverCacheLoad"
  | "solverCacheDelete"
  | "solverCacheRename"
  | "solverView"
  | "solverBreakdown"
  | "solverExport"
  | "solverLoadedSpot"
  | "solverFlops"
  | "solverBatchStart"
  | "solverBatchProgress"
  | "solverBatchCancel"
>;

const strength = (i: number) => 1 - RANKED_CELLS.indexOf(cellOf(i)) / 169;

function check(spot: SpotConfig) {
  const b = parseCards(spot.board);
  if (!b || b.length < 3 || b.length > 5) throw new BackendError("the board must have 3 to 5 different cards");
  if (!tryParseRange(spot.oopRange) || !tryParseRange(spot.ipRange)) throw new BackendError("OOP range: invalid range");
}

function playerView(spot: SpotConfig, who: 0 | 1, actions: number): PlayerView {
  const w = tryParseRange(who === 0 ? spot.oopRange : spot.ipRange)!;
  const dead = new Set(parseCards(spot.board) ?? []);
  const hands = [];
  const grid = Array.from({ length: 169 }, (_, cell) => ({ cell, weight: 0, freq: new Array(actions).fill(0), ev: 0, equity: 0 }));
  for (let i = 0; i < NUM_COMBOS; i++) {
    if (!(w[i] > 0) || dead.has(COMBOS[i][0]) || dead.has(COMBOS[i][1])) continue;
    const st = strength(i);
    const bet = actions > 1 ? Math.min(1, Math.max(0, (st - 0.4) * 2)) : 1;
    const strategy = actions > 1 ? [1 - bet, ...new Array(actions - 1).fill(bet / (actions - 1))] : [1];
    const h = { combo: i, cards: comboStr(i), weight: w[i], strategy, ev: st * spot.pot, equity: st };
    hands.push(h);
    const g = grid[cellOf(i)];
    g.weight += w[i];
    strategy.forEach((f, a) => (g.freq[a] += f * w[i]));
    g.ev += h.ev * w[i];
    g.equity += st * w[i];
  }
  for (const g of grid) if (g.weight > 0) (g.freq = g.freq.map((f) => f / g.weight)), (g.ev /= g.weight), (g.equity /= g.weight);
  return { hands, grid, ev: spot.pot / 2, equity: 0.5, eqr: 1 };
}

export function mockSolver(): SolverPart {
  let job: { spot: SpotConfig; locks: Lock[]; name: string; start: number; paused: boolean; cancelled: boolean; max: number } | null = null;
  const cache: CacheEntry[] = [];
  const slots: (CacheEntry | null)[] = [null, null];
  let batch: { flops: string[]; start: number; cancelled: boolean } | null = null;
  const progress = (): Progress | null => {
    if (!job) return null;
    const t = Math.min(1, (Date.now() - job.start) / 900);
    const it = Math.round(t * 200);
    const e = 50 / (1 + it);
    const state = job.cancelled ? "cancelled" : job.paused ? "paused" : t >= 1 ? "done" : "solving";
    return {
      state,
      iteration: it,
      maxIterations: job.max,
      exploitabilityPct: e,
      history: Array.from({ length: Math.floor(it / 10) + 1 }, (_, k) => [k * 10, 50 / (1 + k * 10), k * 0.05] as [number, number, number]),
      elapsedMs: Date.now() - job.start,
      memoryMb: 512,
      error: null,
    };
  };
  const loaded = (slot: number) => {
    const e = slots[slot];
    if (!e) throw new BackendError("no solution loaded");
    return e;
  };
  const view = (spot: SpotConfig, locks: Lock[], path: number[]): NodeView => {
    const board = (parseCards(spot.board) ?? []).map((c) => "23456789TJQKA"[c >> 2] + "cdhs"[c & 3]);
    // depth 0: OOP check/bet; 1: IP (after check: check/bet, after bet: fold/call/raise); 2 (after check-check or bet-call): chance
    const base = { path, board, cards: [] as string[], pot: spot.pot, bets: [0, 0] as [number, number], locked: false, totals: [] as number[], players: [] as PlayerView[] };
    let actions: ActionInfo[];
    let player: 0 | 1;
    if (path.length === 0) {
      actions = [{ kind: "check", amount: 0 }, { kind: "bet", amount: +(spot.pot * 0.33).toFixed(2) }, { kind: "bet", amount: +(spot.pot * 0.75).toFixed(2) }];
      player = 0;
    } else if (path.length === 1) {
      actions = path[0] === 0 ? [{ kind: "check", amount: 0 }, { kind: "bet", amount: +(spot.pot * 0.5).toFixed(2) }] : [{ kind: "fold", amount: 0 }, { kind: "call", amount: 0 }, { kind: "raise", amount: +(spot.pot * 1.2).toFixed(2) }];
      player = 1;
    } else if (path.length === 2 && board.length < 5) {
      const dead = new Set(board);
      const cards = [];
      for (const r of "23456789TJQKA") for (const s of "cdhs") if (!dead.has(r + s)) cards.push(r + s);
      return { ...base, kind: "chance", player: null, actions: [], cards };
    } else {
      return { ...base, kind: "terminal", player: null, actions: [] };
    }
    const players = [playerView(spot, 0, player === 0 ? actions.length : 0), playerView(spot, 1, player === 1 ? actions.length : 0)];
    const me = players[player];
    const ws = me.grid.reduce((a, g) => a + g.weight, 0);
    const totals = actions.map((_, a) => me.grid.reduce((s, g) => s + g.weight * (g.freq[a] ?? 0), 0) / ws);
    const locked = locks.some((l) => l.path.join() === path.join());
    return { ...base, kind: "action", player, actions, totals, players, locked };
  };
  return {
    async solverEstimate(spot) {
      check(spot);
      return { memory: 6.4e9, memoryCompressed: 3.2e9, treeNodes: 860, oopHands: 179, ipHands: 264 };
    },
    async solverStart(spot, params, locks, name) {
      check(spot);
      if (job && ["solving", "paused", "building"].includes(progress()!.state)) throw new BackendError("a solve is already running");
      job = { spot, locks, name, start: Date.now(), paused: false, cancelled: false, max: params.maxIterations };
    },
    async solverProgress() {
      return progress();
    },
    async solverControl(action) {
      if (!job) throw new BackendError("no solve running");
      if (action === "pause") job.paused = true;
      if (action === "resume") job.paused = false;
      if (action === "cancel") job.cancelled = true;
    },
    async solverFinish() {
      const p = progress();
      if (!job || p?.state !== "done") throw new BackendError("the solve is not finished");
      const e: CacheEntry = {
        hash: Math.random().toString(16).slice(2, 18).padEnd(16, "0"),
        name: job.name,
        createdAt: Math.floor(Date.now() / 1000),
        spot: job.spot,
        locks: job.locks,
        exploitabilityPct: p.exploitabilityPct,
        iterations: p.iteration,
        bytes: 123_456_789,
      };
      cache.unshift(e);
      slots[0] = e;
      job = null;
      return e;
    },
    async solverCacheList() {
      return [...cache];
    },
    async solverCacheLoad(hash, slot) {
      const e = cache.find((x) => x.hash === hash);
      if (!e) throw new BackendError("unknown solution");
      slots[slot] = e;
      return e.spot;
    },
    async solverCacheDelete(hash) {
      const i = cache.findIndex((x) => x.hash === hash);
      if (i >= 0) cache.splice(i, 1);
      slots.forEach((s, k) => s?.hash === hash && (slots[k] = null));
    },
    async solverCacheRename(hash, name) {
      const e = cache.find((x) => x.hash === hash);
      if (e) e.name = name;
    },
    async solverView(slot, path) {
      const e = loaded(slot);
      return view(e.spot, e.locks, path);
    },
    async solverBreakdown(slot, path, player): Promise<Breakdown> {
      const v = view(loaded(slot).spot, [], path);
      const total = v.players[player]?.hands.reduce((a, h) => a + h.weight, 0) ?? 0;
      return {
        total,
        made: MADE_IDS.map((id, k) => ({ id, combos: k === 9 ? total / 4 : k === 14 ? (total * 3) / 4 : 0, share: k === 9 ? 0.25 : k === 14 ? 0.75 : 0 })),
        draws: DRAW_IDS.map((id) => ({ id, combos: 0, share: 0 })),
      };
    },
    async solverExport(_slot, path, format) {
      return `(browser mock)/exports/solver-${path.join("-") || "root"}.${format}`;
    },
    async solverFlops(filter) {
      const all = ["AsKdQc", "Ks7d2c", "7s7d2c", "AsKsQs", "9h8h7d", "Th6d2s"];
      return filter.suits === "monotone" ? ["AsKsQs"] : filter.pairing === "paired" ? ["7s7d2c"] : all;
    },
    async solverBatchStart(spot, _params, flops) {
      check({ ...spot, board: flops[0] ?? "" });
      batch = { flops, start: Date.now(), cancelled: false };
    },
    async solverBatchProgress(): Promise<BatchProgress | null> {
      if (!batch) return null;
      const done = batch.cancelled ? 0 : Math.min(batch.flops.length, Math.floor((Date.now() - batch.start) / 200));
      const rows = batch.flops.slice(0, done).map((flop, k) => ({
        flop,
        hash: `batch${k}`.padEnd(16, "0"),
        exploitabilityPct: 0.4,
        actions: [{ kind: "check", amount: 0 }, { kind: "bet", amount: 1.82 }, { kind: "bet", amount: 4.13 }] as ActionInfo[],
        totals: [0.5 - k * 0.05, 0.3 + k * 0.03, 0.2 + k * 0.02],
        evOop: 2.4 + k * 0.1,
        equityOop: 0.48,
      }));
      return { total: batch.flops.length, done, current: done < batch.flops.length ? batch.flops[done] : null, running: !batch.cancelled && done < batch.flops.length, rows, errors: [] };
    },
    async solverBatchCancel() {
      if (batch) batch.cancelled = true;
    },
    async solverLoadedSpot(slot) {
      const e = slots[slot];
      return e ? [e.hash, e.spot] : null;
    },
  };
}
