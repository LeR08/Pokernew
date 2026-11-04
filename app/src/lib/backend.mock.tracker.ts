/** Browser-mock data for the tracker and replayer (dev server, tests). */
import type { Backend } from "./backend";
import type { Finding, Graph, GridCell, Hand, HandRow, LeakRow, Note, ReportRow, StatMeta, Tx } from "./tracker";
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

const RANKS = "AKQJT98765432";

export function mockTracker(statsMeta: () => StatMeta[]): Pick<
  Backend,
  | "trackerReport"
  | "trackerGraph"
  | "trackerTournaments"
  | "trackerHands"
  | "trackerAccounts"
  | "handGet"
  | "statPreview"
  | "trackerSessions"
  | "sessionReview"
  | "trackerLeaks"
  | "trackerGrid"
  | "notesGet"
  | "noteAdd"
  | "noteDelete"
  | "playerColour"
  | "bankrollGet"
  | "bankrollTxList"
  | "bankrollTxAdd"
  | "bankrollTxDelete"
  | "exportCsv"
  | "trackerOpponents"
  | "autonotesRun"
  | "hudStatus"
  | "playModeGet"
  | "hudSetVisible"
  | "hudSetEdit"
  | "hudSetEditTable"
  | "hudSetTableHidden"
  | "hudOverlay"
  | "hudSaveSeat"
  | "onHudEvent"
> {
  let hudVisible = true;
  let hudEdit = false;
  let editTable: number | null = null;
  const hiddenTables = new Set<number>();
  const hudListeners = new Set<(k: "playmode" | "hud-refresh" | "hud-edit" | "hud-status", p: unknown) => void>();
  const savedSeats: Record<string, [number, number][]> = {};
  const notes = new Map<string, Note[]>();
  const colours = new Map<string, string | null>();
  const txs: Tx[] = [];
  let nextId = 1;
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
        { id: 1, name: "SPACE KO", gameType: "mtt", startedAt: 1793644201, hands: 176, entries: 1, costCents: 50, wonCents: 103, profitCents: 53, finishPosition: 203, registeredPlayers: 1771, netBb: 12.5, kind: "knockout", prizeCents: 50, bountyCents: 53, knockouts: 2 },
        { id: 2, name: "TRIDENT SPACE KO", gameType: "mtt", startedAt: 1793643300, hands: 24, entries: 1, costCents: 50, wonCents: 0, profitCents: -50, finishPosition: 149, registeredPlayers: 559, netBb: -12.5, kind: "knockout", prizeCents: 0, bountyCents: 0, knockouts: 0 },
        { id: 3, name: "Expresso Nitro", gameType: "spin", startedAt: 1743113860, hands: 12, entries: 1, costCents: 100, wonCents: null, profitCents: null, finishPosition: null, registeredPlayers: null, netBb: -15, kind: null, prizeCents: null, bountyCents: null, knockouts: 1 },
      ];
      return {
        rows,
        totals: { tournaments: 3, withResult: 2, costCents: 100, wonCents: 103, profitCents: 3, roi: 0.03, itm: 0.5, curveEur: [-0.5, 0.03], curveT: [1793643300, 1793644201],
          entries: 4, reentries: 1, bountiesCents: 53, knockouts: 3, avgFinishPct: 0.19, bestFinish: [203, 1771], wins: 0, top10: 0, avgBuyInCents: 50, biggestCashCents: 103,
          byKind: [{ key: "mtt·knockout", tournaments: 2, costCents: 100, wonCents: 103, profitCents: 3, roi: 0.03, itm: 0.5 }],
          byBuyIn: [{ key: "a:<1", tournaments: 2, costCents: 100, wonCents: 103, profitCents: 3, roi: 0.03, itm: 0.5 }] },
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
    async trackerSessions() {
      return [
        { start: 1793643300, end: 1793654895, hands: 200, netBb: 42.5, netCashCents: 0, tournaments: 2, vpipHands: 58, pfrHands: 41 },
        { start: 1793474056, end: 1793480000, hands: 88, netBb: -61.2, netCashCents: 0, tournaments: 1, vpipHands: 30, pfrHands: 19 },
      ];
    },
    async sessionReview(_f, start) {
      const base = { holeCards: "8hQd", position: "BB", stackBb: 12.5, playedAt: start + 60 };
      return {
        session: { start, end: start + 3600, hands: 200, netBb: 42.5, netCashCents: 0, tournaments: 2, vpipHands: 58, pfrHands: 41 },
        findings: <Finding[]>[
          { ...base, rule: "river_call_lost", kind: "review", handId: 1, netBb: -18.5, detail: { call: 6.8, lost: 18.5 } },
          { ...base, rule: "open_limp", kind: "review", handId: 2, netBb: -1, detail: {} },
          { ...base, rule: "steal_won", kind: "good", handId: 3, netBb: 1.6, detail: { won: 1.6 } },
          { ...base, rule: "big_pot", kind: "good", handId: 1, netBb: 21.0, detail: { won: 21.0 } },
        ],
        vpip: 0.29,
        pfr: 0.205,
        vpipOverall: 0.236,
        pfrOverall: 0.174,
        tiltWarning: false,
        worstHand: [1, -18.5],
        bestHand: [3, 21],
      };
    },
    async trackerLeaks() {
      const stats = statsMeta();
      return {
        stats,
        rows: <LeakRow[]>[
          { stat: "fold_to_steal", gameType: "mtt", value: 0.85, sample: 120, ciLow: 0.78, ciHigh: 0.9, target: { min: 0.55, max: 0.8 }, status: "leak", direction: "high", tooLow: null, tooHigh: { fr: "Tu abandonnes trop souvent tes blindes : défends plus souvent, surtout en big blind.", en: "You give up your blinds too often." }, bb100WhenDone: -50, bb100WhenNot: 12 },
          { stat: "three_bet", gameType: "mtt", value: 0.044, sample: 136, ciLow: 0.02, ciHigh: 0.09, target: { min: 0.05, max: 0.12 }, status: "possible", direction: "low", tooLow: { fr: "Tu sur-relances rarement.", en: "You rarely 3-bet." }, tooHigh: null, bb100WhenDone: 80, bb100WhenNot: -20 },
          { stat: "vpip", gameType: "mtt", value: 0.236, sample: 496, ciLow: 0.2, ciHigh: 0.27, target: { min: 0.18, max: 0.28 }, status: "ok", direction: null, tooLow: null, tooHigh: null, bb100WhenDone: 10, bb100WhenNot: -9 },
        ],
      };
    },
    async trackerGrid() {
      const r = rng(3);
      const cells: GridCell[] = [];
      for (let i = 0; i < 13; i++)
        for (let j = 0; j < 13; j++) {
          const cls = i === j ? RANKS[i] + RANKS[i] : i < j ? `${RANKS[i]}${RANKS[j]}s` : `${RANKS[j]}${RANKS[i]}o`;
          const hands = Math.round(r() * (i === j ? 8 : i < j ? 6 : 18));
          cells.push({ class: cls, hands, netBb: hands ? (r() - 0.5) * 40 - (i + j) * 0.6 : 0, vpip: Math.round(hands * 0.5), pfr: Math.round(hands * 0.3) });
        }
      return cells;
    },
    async notesGet(room, name) {
      return { playerId: 1, name, colour: colours.get(`${room}/${name}`) ?? null, notes: notes.get(`${room}/${name}`) ?? [] };
    },
    async noteAdd(room, name, text) {
      const k = `${room}/${name}`;
      const id = nextId++;
      notes.set(k, [{ id, text, auto: false, createdAt: Math.floor(Date.now() / 1000) }, ...(notes.get(k) ?? [])]);
      return id;
    },
    async noteDelete(id) {
      for (const [k, v] of notes) notes.set(k, v.filter((n) => n.id !== id));
    },
    async playerColour(room, name, colour) {
      colours.set(`${room}/${name}`, colour);
    },
    async bankrollGet(_account) {
      const dep = txs.filter((t) => t.kind !== "withdrawal").reduce((a, t) => a + t.amountCents, 0);
      const wd = txs.filter((t) => t.kind === "withdrawal").reduce((a, t) => a + t.amountCents, 0);
      const bal = dep - wd + 3;
      return { t: [1793474056, 1793654895], balanceEur: [(dep - wd) / 100, bal / 100], depositsCents: dep, withdrawalsCents: wd, cashCents: 0, tournamentsCents: 3, balanceCents: bal, avgBuyInCents: 75 };
    },
    async bankrollTxList() {
      return [...txs].sort((a, b) => b.at - a.at);
    },
    async bankrollTxAdd(tx) {
      const id = nextId++;
      txs.push({ ...tx, id });
      return id;
    },
    async bankrollTxDelete(id) {
      const i = txs.findIndex((t) => t.id === id);
      if (i >= 0) txs.splice(i, 1);
    },
    async trackerOpponents() {
      const stats = statsMeta();
      const mk = (key: string, hands: number, k: number) => ({ ...row(key, hands, k), key });
      return { stats, pool: mk("pool", 40000, 0.9), rows: [mk("Player10", 412, 1.4), mk("Player34", 288, 0.6), mk("Player27", 150, 1.1)] };
    },
    async autonotesRun() {
      const k = "winamax/Player10";
      notes.set(k, [{ id: nextId++, text: "Joue beaucoup de mains en suivant (VPIP 42 %, PFR 12 %)", auto: true, createdAt: Math.floor(Date.now() / 1000) }, ...(notes.get(k) ?? []).filter((n) => !n.auto)]);
      return 1;
    },
    async hudStatus() {
      // The browser mock pretends one Winamax table is open (window id 1).
      const h = HANDS[0];
      const tables = [
        {
          window: { id: 1, title: "Winamax TRIDENT SPACE KO(1195602081)(#0053)", class: "Chrome_WidgetWin_1", process: "Winamax.exe", client: { x: 0, y: 30, w: 700, h: 500 }, scale: 1, minimized: false, visible: true },
          table: { room: "winamax", tableName: h.table_name, tournamentId: "1195602081", tableNumber: 53, maxSeats: h.max_seats, lastHandAt: h.played_at },
          matchedBy: h.table_name,
        },
      ];
      return { supported: false, visible: hudVisible, edit: hudEdit, playMode: false, unlockIn: 0, tables, editTable, hidden: [...hiddenTables], windows: [] };
    },
    async playModeGet() {
      return false;
    },
    async hudSetVisible(v) {
      hudVisible = v;
      hudListeners.forEach((l) => l("hud-status", null));
    },
    async hudSetEdit(e) {
      hudEdit = e;
      hudListeners.forEach((l) => l("hud-edit", e));
    },
    async hudSetEditTable(id) {
      editTable = id;
      if (id === null) hudEdit = false;
      hudListeners.forEach((l) => l("hud-edit", null));
    },
    async hudSetTableHidden(id, hidden) {
      if (hidden) hiddenTables.add(id);
      else hiddenTables.delete(id);
      hudListeners.forEach((l) => l("hud-status", null));
    },
    async hudOverlay() {
      const h = HANDS[0];
      const stats = statsMeta();
      const layout = {
        name: "Standard",
        opacity: 0.85,
        min_hands: 20,
        show_hero: false,
        rows: [["vpip", "pfr", "three_bet", "hands"], ["af", "cbet_flop", "fold_to_cbet_flop", "wtsd"]],
        popup: ["vpip", "pfr", "three_bet", "fold_to_3bet", "steal", "wtsd", "af"],
        seats: { "3": savedSeats["3"] ?? [[0.42, 0.74], [0.04, 0.3], [0.78, 0.3]] },
      };
      const heroSeat = h.players.find((p) => p.is_hero)!.seat;
      return {
        layout,
        layoutId: "standard",
        edit: hudEdit || editTable === 1,
        windowId: 1,
        stats: stats.filter((m) => layout.rows.flat().includes(m.id) || layout.popup.includes(m.id)),
        table: {
          room: "winamax",
          tableName: h.table_name,
          maxSeats: h.max_seats,
          lastHandAt: h.played_at,
          seats: h.players.map((p, i) => ({
            seat: p.seat,
            slot: (((p.seat - heroSeat) % 3) + 3) % 3,
            name: p.name,
            isHero: p.is_hero,
            position: p.position,
            hands: [24, 288, 24][i] ?? 10,
            stats: stats.map((m) => ({ id: m.id, value: m.id in VALUES ? VALUES[m.id] * (0.8 + 0.15 * i) : 0.3, sample: 20 })),
            colour: i === 0 ? "red" : null,
            notes: i === 0 ? ["Relance large en SB", "Joue beaucoup de mains en suivant (VPIP 42 %, PFR 12 %)"] : [],
          })),
        },
      };
    },
    async hudSaveSeat(size, slot, x, y) {
      const s = savedSeats[String(size)] ?? [[0.42, 0.74], [0.04, 0.3], [0.78, 0.3]];
      s[slot] = [x, y];
      savedSeats[String(size)] = s;
    },
    onHudEvent(cb) {
      hudListeners.add(cb);
      return () => hudListeners.delete(cb);
    },
    async exportCsv(name) {
      return `(browser mock)/exports/${name}.csv`;
    },
    async statPreview(def) {
      if (!def.formula.trim()) throw new Error("formula: empty formula (position 0)");
      if (/[^a-z0-9_ ()/*+\-.<>=!',]/i.test(def.formula)) throw new Error("formula: unexpected character");
      return { value: 0.25, sample: 1234, hands: 7316 };
    },
  };
}
