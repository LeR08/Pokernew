/**
 * Cards, the 1 326 combos and range notation — the same model and grammar as
 * crates/equity (src/combo.rs, src/range.rs). Both are tested against
 * crates/equity/data/range_cases.json.
 *
 * Card = rank * 4 + suit (rank 0 = deuce … 12 = ace, suit "cdhs").
 * Combo index for hi > lo: hi * (hi - 1) / 2 + lo.
 * Grid cell = row * 13 + col, row/col 0 = ace; pairs on the diagonal, suited
 * above it, offsuit below.
 */
import rankingJson from "../../../crates/equity/data/preflop_ranking.json";

export const RANKS = "23456789TJQKA";
export const SUITS = "cdhs";
export const NUM_COMBOS = 1326;

export type Card = number;
export type Weights = Float32Array;

export const rankOf = (c: Card) => c >> 2;
export const suitOf = (c: Card) => c & 3;
export const makeCard = (rank: number, suit: number): Card => rank * 4 + suit;

export function parseCard(s: string): Card | null {
  const t = s.trim().replace(/^10/, "T");
  if (t.length !== 2) return null;
  const r = RANKS.indexOf(t[0].toUpperCase());
  const su = SUITS.indexOf(t[1].toLowerCase());
  return r < 0 || su < 0 ? null : makeCard(r, su);
}

/** "AhKd 7c" → cards; null when invalid. */
export function parseCards(s: string): Card[] | null {
  const t = s.replace(/[\s,]/g, "").replace(/10/g, "T");
  if (t.length % 2) return null;
  const out: Card[] = [];
  for (let i = 0; i < t.length; i += 2) {
    const c = parseCard(t.slice(i, i + 2));
    if (c === null) return null;
    out.push(c);
  }
  return out;
}

export const cardStr = (c: Card) => RANKS[rankOf(c)] + SUITS[suitOf(c)];

export const comboIndex = (a: Card, b: Card) => {
  const [hi, lo] = a > b ? [a, b] : [b, a];
  return (hi * (hi - 1)) / 2 + lo;
};

/** combos[i] = [hi, lo] */
export const COMBOS: [Card, Card][] = (() => {
  const out: [Card, Card][] = new Array(NUM_COMBOS);
  for (let hi = 1; hi < 52; hi++) for (let lo = 0; lo < hi; lo++) out[comboIndex(hi, lo)] = [hi, lo];
  return out;
})();

export const comboStr = (i: number) => cardStr(COMBOS[i][0]) + cardStr(COMBOS[i][1]);

export function cellOf(i: number): number {
  const [hi, lo] = COMBOS[i];
  const r1 = 12 - rankOf(hi);
  const r2 = 12 - rankOf(lo);
  if (r1 === r2) return r1 * 13 + r1;
  return suitOf(hi) === suitOf(lo) ? Math.min(r1, r2) * 13 + Math.max(r1, r2) : Math.max(r1, r2) * 13 + Math.min(r1, r2);
}

export function cellName(cell: number): string {
  const row = Math.floor(cell / 13);
  const col = cell % 13;
  const r = (i: number) => RANKS[12 - i];
  if (row === col) return r(row) + r(row);
  return row < col ? `${r(row)}${r(col)}s` : `${r(col)}${r(row)}o`;
}

/** Combo indices of each grid cell. */
export const CELL_COMBOS: number[][] = (() => {
  const out: number[][] = Array.from({ length: 169 }, () => []);
  for (let i = 0; i < NUM_COMBOS; i++) out[cellOf(i)].push(i);
  return out;
})();

export const emptyRange = (): Weights => new Float32Array(NUM_COMBOS);

// ---- preflop ranking (top X %) ----

const CELL_BY_NAME = new Map(Array.from({ length: 169 }, (_, c) => [cellName(c), c]));
export const RANKED_CELLS: number[] = (rankingJson as { hand: string }[]).map((h) => CELL_BY_NAME.get(h.hand)!);

/** Same rule as crates/equity/src/ranking.rs: whole classes, nearest boundary. */
export function topClasses(pct: number): number {
  const target = (pct / 100) * NUM_COMBOS;
  let total = 0;
  for (let i = 0; i < RANKED_CELLS.length; i++) {
    const next = total + CELL_COMBOS[RANKED_CELLS[i]].length;
    if (next > target && next - target > target - total) return i;
    total = next;
  }
  return RANKED_CELLS.length;
}

export function topPercent(pct: number): number[] {
  return RANKED_CELLS.slice(0, topClasses(pct)).flatMap((c) => CELL_COMBOS[c]);
}

// ---- parsing ----

export class RangeParseError extends Error {
  constructor(public token: string) {
    super(`invalid range token '${token}'`);
  }
}

function parseWeight(s: string): number | null {
  let t = s.trim();
  const pct = t.endsWith("%");
  if (pct) t = t.slice(0, -1).trim();
  if (!/^\d+(\.\d+)?$|^\.\d+$/.test(t)) return null;
  const v = Number(t);
  const w = pct || v > 1 ? v / 100 : v;
  return w >= 0 && w <= 1 ? w : null;
}

const rank = (c: string) => RANKS.indexOf(c.toUpperCase());

function combosOf(r1: number, r2: number, suited: boolean | null): number[] {
  const out: number[] = [];
  for (let s1 = 0; s1 < 4; s1++)
    for (let s2 = 0; s2 < 4; s2++) {
      if (r1 === r2 && s1 >= s2) continue;
      if (suited !== null && (s1 === s2) !== suited) continue;
      if (r1 !== r2 || s1 !== s2) out.push(comboIndex(makeCard(r1, s1), makeCard(r2, s2)));
    }
  return out;
}

function handClass(s: string): [number, number, boolean | null] | null {
  if (s.length < 2 || s.length > 3) return null;
  const a = rank(s[0]);
  const b = rank(s[1]);
  if (a < 0 || b < 0) return null;
  let suited: boolean | null = null;
  if (s.length === 3) {
    const x = s[2].toLowerCase();
    if (a === b || (x !== "s" && x !== "o")) return null;
    suited = x === "s";
  }
  return [Math.max(a, b), Math.min(a, b), suited];
}

function expand(tok: string): number[] | null {
  const lower = tok.toLowerCase();
  if (lower === "random" || lower === "any") return COMBOS.map((_, i) => i);
  if (tok.endsWith("%")) {
    const p = tok.slice(0, -1);
    if (!/^\d+(\.\d+)?$/.test(p)) return null;
    const pct = Number(p);
    return pct <= 100 ? topPercent(pct) : null;
  }
  if (tok.length === 4) {
    const a = parseCard(tok.slice(0, 2));
    const b = parseCard(tok.slice(2));
    if (a !== null && b !== null && a !== b && tok[1] === tok[1].toLowerCase() && /[a-z]/.test(tok[1])) return [comboIndex(a, b)];
  }
  if (tok.endsWith("+")) {
    const c = handClass(tok.slice(0, -1));
    if (!c) return null;
    const [hi, lo, s] = c;
    const out: number[] = [];
    if (hi === lo) for (let r = hi; r < 13; r++) out.push(...combosOf(r, r, null));
    else for (let k = lo; k < hi; k++) out.push(...combosOf(hi, k, s));
    return out;
  }
  const dash = tok.indexOf("-");
  if (dash > 0) {
    const x = handClass(tok.slice(0, dash));
    const y = handClass(tok.slice(dash + 1));
    if (!x || !y || x[2] !== y[2]) return null;
    const out: number[] = [];
    if (x[0] === x[1] && y[0] === y[1]) {
      for (let r = Math.min(x[0], y[0]); r <= Math.max(x[0], y[0]); r++) out.push(...combosOf(r, r, null));
    } else if (x[0] === y[0] && x[0] !== x[1] && y[0] !== y[1]) {
      for (let k = Math.min(x[1], y[1]); k <= Math.max(x[1], y[1]); k++) out.push(...combosOf(x[0], k, x[2]));
    } else return null;
    return out;
  }
  const c = handClass(tok);
  return c ? combosOf(c[0], c[1], c[2]) : null;
}

/** Parses range notation; throws RangeParseError on the first bad token. */
export function parseRange(text: string): Weights {
  const w = emptyRange();
  for (const tok of text.split(/[,\s]+/).filter(Boolean)) {
    const colon = tok.indexOf(":");
    let weight = 1;
    let body = tok;
    if (colon >= 0) {
      const v = parseWeight(tok.slice(colon + 1));
      if (v === null) throw new RangeParseError(tok);
      weight = v;
      body = tok.slice(0, colon);
    }
    const combos = expand(body);
    if (!combos) throw new RangeParseError(tok);
    for (const i of combos) w[i] = weight;
  }
  return w;
}

export function tryParseRange(text: string): Weights | null {
  try {
    return parseRange(text);
  } catch {
    return null;
  }
}

// ---- counting ----

export function comboCount(w: Weights): number {
  let n = 0;
  for (let i = 0; i < NUM_COMBOS; i++) n += w[i];
  return n;
}

/** Weighted combos that do not touch `dead` cards. */
export function liveCount(w: Weights, dead: Card[]): number {
  const d = new Set(dead);
  let n = 0;
  for (let i = 0; i < NUM_COMBOS; i++) if (w[i] > 0 && !d.has(COMBOS[i][0]) && !d.has(COMBOS[i][1])) n += w[i];
  return n;
}

// ---- formatting ----

const fmtWeight = (w: number) => {
  const p = Math.round(w * 10000) / 100;
  return p >= 100 ? "" : `:${p}%`;
};

/** Runs of consecutive numbers (descending input) → [[start, end], …]. */
function runs(xs: number[]): [number, number][] {
  const out: [number, number][] = [];
  for (const x of xs) {
    const last = out[out.length - 1];
    if (last && last[1] - 1 === x) last[1] = x;
    else out.push([x, x]);
  }
  return out;
}

/**
 * Compact notation: whole cells grouped as `TT+`, `99-66`, `ATs+`, `A5s-A2s`,
 * `AK` (suited + offsuit); partial cells as single combos; weights as `:50%`.
 */
export function formatRange(w: Weights): string {
  // weight of each full cell, or null when the cell is mixed
  const cellW: (number | null)[] = CELL_COMBOS.map((cs) => (cs.every((i) => w[i] === w[cs[0]]) ? w[cs[0]] : null));
  if (cellW.every((x) => x === 1)) return "random";
  const tokens: string[] = [];
  const weights = [...new Set(cellW.filter((x): x is number => x !== null && x > 0))].sort((a, b) => b - a);
  const R = (r: number) => RANKS[r];
  for (const wt of weights) {
    const suf = fmtWeight(wt);
    const has = (hi: number, lo: number, s: boolean) => {
      const cell = s ? (12 - hi) * 13 + (12 - lo) : (12 - lo) * 13 + (12 - hi);
      return cellW[cell] === wt;
    };
    // pairs
    const pairs: number[] = [];
    for (let r = 12; r >= 0; r--) if (cellW[(12 - r) * 14] === wt) pairs.push(r);
    for (const [a, b] of runs(pairs)) {
      tokens.push((a === 12 && a !== b ? `${R(b)}${R(b)}+` : a === b ? `${R(a)}${R(a)}` : `${R(a)}${R(a)}-${R(b)}${R(b)}`) + suf);
    }
    for (let hi = 12; hi >= 1; hi--) {
      for (const kind of ["", "s", "o"] as const) {
        const ks: number[] = [];
        for (let lo = hi - 1; lo >= 0; lo--) {
          const s = has(hi, lo, true);
          const o = has(hi, lo, false);
          if (kind === "" ? s && o : kind === "s" ? s && !o : o && !s) ks.push(lo);
        }
        for (const [a, b] of runs(ks)) {
          const h = R(hi);
          if (a === hi - 1 && a !== b) tokens.push(`${h}${R(b)}${kind}+${suf}`);
          else if (a === b) tokens.push(`${h}${R(a)}${kind}${suf}`);
          else tokens.push(`${h}${R(a)}${kind}-${h}${R(b)}${kind}${suf}`);
        }
      }
    }
  }
  // mixed cells: single combos
  for (let cell = 0; cell < 169; cell++) {
    if (cellW[cell] !== null) continue;
    for (const i of CELL_COMBOS[cell]) if (w[i] > 0) tokens.push(comboStr(i) + fmtWeight(w[i]));
  }
  return tokens.join(",");
}

/** A hand ("AhKd") or a range, as typed in a player field. */
export const isSingleHand = (s: string) => /^[2-9TJQKA][cdhs][2-9TJQKA][cdhs]$/i.test(s.trim());
