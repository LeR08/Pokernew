/** Browser-mock data for the tracker and replayer (dev server, tests). */
import type { Backend } from "./backend";
import type { Graph, Hand, HandRow, ReportRow, StatMeta } from "./tracker";
import mockHands from "./mockHands.json";

const HANDS = mockHands as unknown as Hand[];

/** Deterministic pseudo-random numbers. */
function rng(seed: number) {
  let s = seed;
  return () => {
    s = (s * 16807) % 2147483647;
    return s / 2147483647;
  };
}

const POSITIONS = ["UTG", "HJ", "CO", "BTN", "SB", "BB"];
const VALUES: Record<string, number> = {
  bb100: 4.2,
  vpip: 0.236,
  pfr: 0.174,
  three_bet: 0.054,
  fold_to_3bet: 0.62,
  steal: 0.38,
  fold_to_steal: 0.71,
  cbet_flop: 0.58,
  fold_to_cbet_flop: 0.44,
  wtsd: 0.41,
  wsd: 0.39,
  wwsf: 0.43,
  af: 2.3,
};

export function mockTracker(statsMeta: () => StatMeta[]): Pick<
  Backend,
  "trackerReport" | "trackerGraph" | "trackerTournaments" | "trackerHands" | "trackerAccounts" | "handGet" | "statPreview"
> {
  const row = (key: string, hands: number, k: number): ReportRow => ({
    key,
    hands,
    netBb: 120 * k,
    netCashCents: Math.round(310 * k),
    bbPer100: (120 * k * 100) / hands,
    stats: statsMeta().map((m) => ({
      id: m.id,
      value: m.id in VALUES ? VALUES[m.id] * (0.8 + 0.4 * k) : 0.2,
      sample: Math.round(hands * 0.4),
    })),
  });
  return {
    async trackerReport(filter, group) {
      const scale = filter.gameTypes?.includes("cash") ? 0.1 : 1;
      const stats = statsMeta();
      const rows =
        group === "position"
          ? POSITIONS.map((p, i) => row(p, Math.round(1200 * scale), [0.6, -0.4, 0.9, 1.3, -0.8, -1.1][i]))
          : [row("all", Math.round(7316 * scale), 1)];
      return { rows, stats, problems: [] };
    },
    async trackerGraph() {
      const r = rng(7);
      const g: Graph = { x: [], t: [], totalBb: [], showdownBb: [], nonShowdownBb: [], cashEur: [], hands: 7316 };
      let sd = 0;
      let nsd = 0;
      for (let i = 1; i <= 7316; i += 20) {
        sd += (r() - 0.47) * 30;
        nsd += (r() - 0.52) * 12;
        g.x.push(i);
        g.t.push(1735000000 + i * 600);
        g.showdownBb.push(sd);
        g.nonShowdownBb.push(nsd);
        g.totalBb.push(sd + nsd);
        g.cashEur.push(i > 7000 ? (i - 7000) / 100 : 0);
      }
      return g;
    },
    async trackerTournaments() {
      const rows = [
        { id: 1, name: "SPACE KO", gameType: "mtt", startedAt: 1793644201, hands: 176, entries: 1, costCents: 50, wonCents: 103, profitCents: 53, finishPosition: 203, registeredPlayers: 1771, netBb: 12.5 },
        { id: 2, name: "TRIDENT SPACE KO", gameType: "mtt", startedAt: 1793643300, hands: 24, entries: 1, costCents: 50, wonCents: 0, profitCents: -50, finishPosition: 149, registeredPlayers: 559, netBb: -12.5 },
        { id: 3, name: "Expresso Nitro", gameType: "spin", startedAt: 1743113860, hands: 12, entries: 1, costCents: 100, wonCents: null, profitCents: null, finishPosition: null, registeredPlayers: null, netBb: -15 },
      ];
      return {
        rows,
        totals: { tournaments: 3, withResult: 2, costCents: 100, wonCents: 103, profitCents: 3, roi: 0.03, itm: 0.5, curveEur: [-0.5, 0.03], curveT: [1793643300, 1793644201] },
      };
    },
    async trackerHands(_filter, limit, offset) {
      const rows: HandRow[] = HANDS.map((h, i) => {
        const hero = h.players.find((p) => p.is_hero)!;
        const net = hero.collected - hero.invested;
        return {
          handId: i + 1,
          playedAt: h.played_at,
          gameType: h.game_type,
          tableName: h.table_name,
          account: hero.name,
          position: hero.position,
          holeCards: hero.hole_cards.join(""),
          board: h.board.join(""),
          stackBb: hero.stack / h.big_blind,
          netWon: net,
          netWonBb: net / h.big_blind,
          unit: h.unit,
          showdown: hero.went_to_showdown,
        };
      });
      return { rows: rows.slice(offset, offset + limit), total: rows.length };
    },
    async trackerAccounts() {
      return ["GTOverBet", "Tantaculejen"];
    },
    async handGet(handId) {
      const h = HANDS[handId - 1];
      if (!h) throw new Error(`hand ${handId} not found`);
      return h;
    },
    async statPreview(def) {
      if (!def.formula.trim()) throw new Error("formula: empty formula (position 0)");
      if (/[^a-z0-9_ ()/*+\-.<>=!',]/i.test(def.formula)) throw new Error("formula: unexpected character");
      return { value: 0.25, sample: 1234, hands: 7316 };
    },
  };
}
