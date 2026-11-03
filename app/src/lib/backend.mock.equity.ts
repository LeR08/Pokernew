/**
 * Browser mock of the Equity Lab commands (Vite dev server, Vitest, Playwright).
 * The numbers are plausible, not exact: the real maths live in crates/equity.
 */
import type { Backend } from "./backend";
import { BackendError } from "./backend";
import type { Breakdown, BreakdownRow, FlopRow } from "./equity";
import { DRAW_IDS, MADE_IDS } from "./equity";
import { cellOf, COMBOS, NUM_COMBOS, parseCards, RANKED_CELLS, rankOf, tryParseRange, type Card, type Weights } from "./range";

type EquityPart = Pick<
  Backend,
  "equityCalc" | "equityCancel" | "rangeBreakdown" | "rangeFilter" | "flopTexture" | "rangesList" | "rangeLoad" | "rangeSave" | "rangeDelete"
>;

const strength = (i: number) => 1 - RANKED_CELLS.indexOf(cellOf(i)) / 169;

function range(text: string): Weights {
  const w = tryParseRange(text);
  if (!w) throw new BackendError(`invalid range '${text}'`);
  return w;
}

function cards(s: string): Card[] {
  const c = parseCards(s);
  if (!c) throw new BackendError(`invalid cards '${s}'`);
  return c;
}

function live(w: Weights, dead: Card[]): number[] {
  const d = new Set(dead);
  const out: number[] = [];
  for (let i = 0; i < NUM_COMBOS; i++) if (w[i] > 0 && !d.has(COMBOS[i][0]) && !d.has(COMBOS[i][1])) out.push(i);
  return out;
}

/** Rough class: enough to exercise the UI. */
function mockClass(i: number, board: Card[]): string {
  const [a, b] = COMBOS[i].map(rankOf);
  const ranks = board.map(rankOf);
  const top = Math.max(...ranks);
  if (a === b) return ranks.includes(a) ? "set" : a > top ? "overpair" : "weak_pair";
  if (ranks.includes(a) && ranks.includes(b)) return "two_pair";
  if (a === top || b === top) return "top_pair";
  if (ranks.includes(a) || ranks.includes(b)) return "weak_pair";
  return a === 12 || b === 12 ? "ace_high" : "no_made_hand";
}

function breakdown(text: string, board: string, dead: string): Breakdown {
  const b = cards(board);
  const w = range(text);
  const ids = live(w, [...b, ...cards(dead)]);
  const made = new Map<string, number>();
  let over = 0;
  let total = 0;
  for (const i of ids) {
    const c = mockClass(i, b);
    made.set(c, (made.get(c) ?? 0) + w[i]);
    total += w[i];
    if ((c === "ace_high" || c === "no_made_hand") && Math.min(...COMBOS[i].map(rankOf)) > Math.max(...b.map(rankOf))) over += w[i];
  }
  const row = (id: string, n: number): BreakdownRow => ({ id, combos: n, share: total ? n / total : 0 });
  return {
    total,
    made: MADE_IDS.map((id) => row(id, made.get(id) ?? 0)),
    draws: DRAW_IDS.map((id) => row(id, id === "overcards" ? over : 0)),
  };
}

export function mockEquity(): EquityPart {
  const library = new Map<string, string>([["exemples/BTN_open", "22+,A2s+,K9s+,QTs+,J9s+,T9s,98s,ATo+,KJo+"]]);
  return {
    async equityCalc(req) {
      if (req.players.length < 2 || req.players.length > 10) throw new BackendError("between 2 and 10 players are needed");
      const board = cards(req.board);
      const dead = [...board, ...cards(req.dead)];
      const players = req.players.map((p) => {
        const w = range(p);
        const ids = live(w, dead);
        if (!ids.length) throw new BackendError("empty range");
        const n = ids.reduce((s, i) => s + w[i], 0);
        const score = ids.reduce((s, i) => s + w[i] * (0.2 + strength(i)), 0) / n;
        return { w, ids, n, score };
      });
      const sum = players.reduce((s, p) => s + p.score, 0);
      await new Promise((r) => setTimeout(r, 60));
      return {
        players: players.map((p) => {
          const equity = p.score / sum;
          return {
            equity,
            win: equity * 0.98,
            tie: equity * 0.02,
            combos: p.n,
            perCombo: p.ids.map((i) => ({ combo: i, weight: p.w[i], equity: Math.min(1, (equity * (0.2 + strength(i))) / p.score) })),
          };
        }),
        method: players.every((p) => p.ids.length === 1) ? "exhaustive" : "monte_carlo",
        samples: 1_712_304,
        elapsedMs: 42,
        margin: players.every((p) => p.ids.length === 1) ? 0 : 0.0009,
      };
    },
    async equityCancel() {},
    async rangeBreakdown(text, board, dead) {
      if (cards(board).length < 3) throw new BackendError("the board needs 3 to 5 cards");
      return breakdown(text, board, dead);
    },
    async rangeFilter(text, board, dead, made, draws) {
      const b = cards(board);
      const w = range(text);
      const out = new Array<number>(NUM_COMBOS).fill(0);
      const over = draws.includes("overcards");
      for (const i of live(w, [...b, ...cards(dead)])) {
        const c = mockClass(i, b);
        if (made.includes(c) || (over && (c === "ace_high" || c === "no_made_hand"))) out[i] = w[i];
      }
      return out;
    },
    async flopTexture(text, filter) {
      range(text);
      const flops: [string, number][] = filter.suits === "monotone" ? [["AsKsQs", 4], ["7s6s5s", 4]] : [["AsKdQc", 24], ["Ks7d2c", 24], ["7s7d2c", 12]];
      const rows: FlopRow[] = flops.map(([cards, weight]) => {
        const b = breakdown(text, cards, "");
        return { cards, weight, made: b.made.map((r) => r.share), draws: b.draws.map((r) => r.share) };
      });
      const n = rows.reduce((s, r) => s + r.weight, 0);
      const avg = (k: "made" | "draws", j: number) => rows.reduce((s, r) => s + r[k][j] * r.weight, 0) / n;
      const total = range(text).reduce((s, x) => s + x, 0);
      return {
        classes: rows.length,
        flops: n,
        made: MADE_IDS.map((id, j) => ({ id, share: avg("made", j), combos: avg("made", j) * total })),
        draws: DRAW_IDS.map((id, j) => ({ id, share: avg("draws", j), combos: avg("draws", j) * total })),
        rows,
      };
    },
    async rangesList() {
      return [...library.keys()].sort();
    },
    async rangeLoad(name) {
      const t = library.get(name);
      if (t === undefined) throw new BackendError(`${name} not found`);
      return t;
    },
    async rangeSave(name, text) {
      range(text);
      if (!name.trim() || name.split("/").some((s) => !s || s === "..")) throw new BackendError(`invalid range name '${name}'`);
      library.set(name.trim(), text.trim());
    },
    async rangeDelete(name) {
      library.delete(name);
    },
  };
}
