/** Browser mock of the preflop solver: a small heads-up tree with plausible frequencies. */
import type { Backend } from "./backend";
import { BackendError } from "./backend";
import type { Act, PfNodeOut, PreflopSolution } from "./preflop";
import { RANKED_CELLS } from "./range";

const rank = new Map(RANKED_CELLS.map((c, i) => [c, i]));
/** Share of the 169 classes playing `act` given a threshold (top fraction). */
const top = (f: number) => Array.from({ length: 169 }, (_, c) => ((rank.get(c) ?? 168) / 169 < f ? 1 : 0));

export function mockPreflop(): Pick<Backend, "preflopSolve"> {
  return {
    async preflopSolve(config, target): Promise<PreflopSolution> {
      if (config.stacks.length < 2 || config.stacks.length > 6) throw new BackendError("between 2 and 6 players are needed");
      await new Promise((r) => setTimeout(r, 50));
      const n = config.stacks.length;
      const nodes: PfNodeOut[] = [];
      const add = (node: Omit<PfNodeOut, "id">) => (nodes.push({ ...node, id: nodes.length }), nodes.length - 1);
      const end = (winner: number) => add({ player: null, actions: [], children: [], end: { type: "fold", winner }, pot: 1.5, freq: [], reach: [], prob: 0.1 });
      const flop = (p: number, q: number, pot: number) => add({ player: null, actions: [], children: [], end: { type: "flop", p, q }, pot, freq: [], reach: [], prob: 0.1 });
      // root: first player; vs raise: next player
      const opener = 0;
      const resp = 1;
      const rootId = add({ player: opener, actions: [], children: [], end: null, pot: 1.5, freq: [], reach: new Array(169).fill(1), prob: 1 });
      const vsId = add({ player: resp, actions: [], children: [], end: null, pot: config.open + 1, freq: [], reach: top(0.45), prob: 0.45 });
      const acts: Act[] = [{ kind: "fold", to: n === 2 ? config.sb : 0 }, { kind: "raise", to: config.open }, { kind: "all_in", to: config.stacks[0] }];
      const raise = top(0.45);
      const jam = top(0.03);
      nodes[rootId].actions = acts;
      nodes[rootId].freq = [raise.map((r, c) => (r || jam[c] ? 0 : 1)), raise.map((r, c) => (jam[c] ? 0 : r)), jam];
      nodes[rootId].children = [end(n - 1), vsId, end(opener)];
      nodes[vsId].actions = [{ kind: "fold", to: 1 }, { kind: "call", to: config.open }, { kind: "raise", to: config.open * config.threeBet }];
      const three = top(0.08);
      const call = top(0.6);
      nodes[vsId].freq = [call.map((x) => 1 - x), call.map((x, c) => (three[c] ? 0 : x)), three];
      nodes[vsId].children = [end(opener), flop(opener, resp, config.open * 2), end(resp)];
      return { config, nodes, values: [0.12, -0.12], exploitability: Math.max(target / 2, 0.002), iterations: 250, history: [[10, 0.3], [100, 0.02], [250, 0.004]] };
    },
  };
}
