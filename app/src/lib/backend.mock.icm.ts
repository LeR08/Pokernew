/**
 * Browser mock of the ICM commands. ICM itself is computed for real
 * (Malmuth-Harville, small); push/fold ranges are rough (top X % by preflop
 * ranking), enough to exercise the UI. The real solver lives in crates/icm.
 */
import type { Backend } from "./backend";
import { BackendError } from "./backend";
import type { Decision, IcmResult, PfNode, Review } from "./icm";
import { RANKED_CELLS } from "./range";

type IcmPart = Pick<Backend, "icmCalc" | "pushfoldSolve" | "icmReviewHand" | "icmReviewTournament">;

export function icmMh(stacks: number[], payouts: number[]): number[] {
  const n = stacks.length;
  const eq = new Array<number>(n).fill(0);
  const total = stacks.reduce((a, b) => a + b, 0);
  const rec = (used: number, rest: number, p: number, k: number) => {
    if (k >= payouts.length || k >= n || p === 0) return;
    for (let i = 0; i < n; i++) {
      if (used & (1 << i) || stacks[i] <= 0) continue;
      const q = (p * stacks[i]) / rest;
      eq[i] += q * payouts[k];
      rec(used | (1 << i), rest - stacks[i], q, k + 1);
    }
  };
  rec(0, total, 1, 0);
  // busted players share the last places (as in crates/icm)
  const live = stacks.filter((x) => x > 0).length;
  const dead = stacks.map((x, i) => (x <= 0 ? i : -1)).filter((i) => i >= 0);
  const share = payouts.slice(live, n).reduce((a, b) => a + b, 0) / Math.max(1, dead.length);
  for (const i of dead) eq[i] = share;
  return eq;
}

const top = (pct: number) => {
  const f = new Array<number>(169).fill(0);
  const k = Math.round((Math.max(0, Math.min(100, pct)) / 100) * 169);
  for (const c of RANKED_CELLS.slice(0, k)) f[c] = 1;
  return f;
};

export function mockIcm(): IcmPart {
  return {
    async icmCalc(stacks, payouts): Promise<IcmResult> {
      if (!stacks.length || stacks.length > 16) throw new BackendError("1 to 16 players with valid stacks");
      const icm = icmMh(stacks, payouts);
      const total = stacks.reduce((a, b) => a + b, 0);
      const pool = payouts.slice(0, stacks.length).reduce((a, b) => a + b, 0);
      const bubble = stacks.map((_, i) =>
        stacks.map((_, j) => {
          if (i === j) return 1;
          const x = Math.min(stacks[i], stacks[j]);
          const w = [...stacks];
          w[i] += x;
          w[j] -= x;
          const l = [...stacks];
          l[i] -= x;
          l[j] += x;
          const gain = icmMh(w, payouts)[i] - icm[i];
          const loss = icm[i] - icmMh(l, payouts)[i];
          return gain > 0 ? loss / gain : 999;
        }),
      );
      return { icm, chipEv: stacks.map((s) => (s / total) * pool), bubble, risk: bubble.map((r) => r.map((b) => b / (1 + b) - 0.5)), prizePool: pool };
    },
    async pushfoldSolve(spot, options) {
      const n = spot.stacks.length;
      if (n < 2 || n > 10) throw new BackendError("between 2 and 10 players are needed");
      await new Promise((r) => setTimeout(r, 50));
      const nodes: PfNode[] = [];
      const bbs = spot.stacks.map((s) => s / spot.bb);
      for (let p = 0; p < n - 1; p++) {
        const lock = options.locks?.find(([d]) => d.kind === "push" && d.p === p);
        const freq = lock ? lock[1] : top(Math.min(100, (120 / bbs[p]) * (2 / (n - p))));
        nodes.push({ decision: { kind: "push", p }, freq, evAct: freq.map((f) => (f ? 0.6 : -0.4)), evFold: new Array(169).fill(0), reach: 1 / (p + 1) });
        for (let q = p + 1; q < n; q++) {
          const d: Decision = { kind: "call", p, q };
          const l = options.locks?.find(([x]) => x.kind === "call" && x.p === p && x.q === q);
          const f = l ? l[1] : top(Math.min(100, 60 / Math.min(bbs[p], bbs[q])));
          nodes.push({ decision: d, freq: f, evAct: f.map((x) => (x ? 0.3 : -0.5)), evFold: new Array(169).fill(0), reach: 0.1 });
        }
      }
      const money = spot.payouts.length > 0;
      const values = money ? icmMh(spot.stacks, spot.payouts) : spot.stacks;
      return { nodes, values, exploitability: 0.001, iterations: 500, unit: money ? "money" : "chips" };
    },
    async icmReviewHand(): Promise<Review> {
      return {
        spot: { stacks: [1800, 2600, 1100], sb: 50, bb: 100, ante: 10, bbAnte: false, others: [], payouts: [50, 30, 20], bounties: [], fgsDepth: 0 },
        names: ["Tantaculejen", "Player2", "Player3"],
        positions: ["BTN", "SB", "BB"],
        hero: 0,
        pusher: null,
        heroCards: "Kh9d",
        heroClass: "K9o",
        played: "fold",
        best: "push",
        chipsAct: 1912,
        chipsFold: 1790,
        moneyAct: 31.4,
        moneyFold: 30.9,
        lossChips: 122,
        lossMoney: 0.5,
        effectiveBb: 18,
      };
    },
    async icmReviewTournament() {
      const r = await this.icmReviewHand(1, { payouts: [], others: [], useBounties: false });
      return [{ handId: 1, playedAt: 1793474056, level: 7, review: r }];
    },
  };
}
