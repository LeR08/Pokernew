/**
 * Pure replay logic: the table state after the first `step` actions.
 * Step 0 = before any post; step `actions.length` = last action;
 * step `actions.length + 1` = result (pots collected, cards shown).
 */
import type { Hand } from "@/lib/tracker";

export interface SeatState {
  stack: number;
  /** Chips in front of the player on the current street (antes excluded). */
  bet: number;
  folded: boolean;
  allIn: boolean;
  /** Last action label key, e.g. "raise". */
  last: string | null;
  showCards: boolean;
  collected: number;
}

export interface TableState {
  seats: SeatState[];
  /** Chips in the middle (previous streets and antes). */
  pot: number;
  board: string[];
  /** Index of the action just played, or -1. */
  current: number;
  finished: boolean;
}

export const totalSteps = (h: Hand) => h.actions.length + 1;

const BOARD_LEN = { preflop: 0, flop: 3, turn: 4, river: 5 } as const;

export function stateAt(h: Hand, step: number): TableState {
  const n = Math.max(0, Math.min(step, totalSteps(h)));
  const seats: SeatState[] = h.players.map((p) => ({
    stack: p.stack,
    bet: 0,
    folded: false,
    allIn: false,
    last: null,
    showCards: p.is_hero,
    collected: 0,
  }));
  let street: keyof typeof BOARD_LEN = "preflop";
  let middle = 0;
  const upto = Math.min(n, h.actions.length);
  for (let i = 0; i < upto; i++) {
    const a = h.actions[i];
    if (a.street !== street) {
      // New street: bets go to the middle.
      for (const s of seats) {
        middle += s.bet;
        s.bet = 0;
        s.last = null;
      }
      street = a.street;
    }
    const s = seats[a.player];
    s.stack -= a.amount;
    if (a.kind === "post_ante") middle += a.amount;
    else s.bet += a.amount;
    if (a.kind === "fold") s.folded = true;
    if (a.is_all_in || s.stack === 0) s.allIn = true;
    s.last = a.kind;
  }
  const finished = n > h.actions.length;
  let board = h.board.slice(0, BOARD_LEN[street]);
  if (finished) {
    for (const s of seats) {
      middle += s.bet;
      s.bet = 0;
    }
    board = h.board;
    h.players.forEach((p, i) => {
      seats[i].collected = p.collected;
      seats[i].stack += p.collected;
      seats[i].showCards = p.is_hero || p.hole_cards.length > 0;
    });
  }
  return { seats, pot: finished ? 0 : middle, board, current: upto - 1, finished };
}
