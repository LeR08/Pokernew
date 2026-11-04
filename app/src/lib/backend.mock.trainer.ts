/** Browser mock of the trainer commands (synthetic questions and analyses). */
import type { Backend } from "./backend";
import { BackendError } from "./backend";
import type { AnalysisProgress, Question } from "./trainer";

type TrainerPart = Pick<Backend, "trainerDeal" | "trainerAnswer" | "trainerNext" | "trainerRecord" | "trainerHistory" | "analysisStart" | "analysisProgress" | "analysisCancel">;

export function mockTrainer(): TrainerPart {
  const history: Record<string, unknown>[] = [];
  let analysis: { ids: number[]; start: number; cancelled: boolean } | null = null;
  const question = (step: number): Question => ({
    path: Array.from({ length: step }, () => 0),
    history: step ? [{ who: "OOP", label: "check" }] : [],
    board: step > 1 ? ["Qs", "Jh", "2h", "7c"] : ["Qs", "Jh", "2h"],
    player: 0,
    pot: 5.5,
    combo: "AhKh",
    hand: 3,
    actions: [
      { kind: "check", amount: 0 },
      { kind: "bet", amount: 1.82 },
      { kind: "bet", amount: 4.13 },
    ],
    nodeKey: step > 1 ? "turn · OOP · first" : "flop · OOP · first",
  });
  return {
    async trainerDeal(hash) {
      if (!hash) throw new BackendError("no solution");
      return question(0);
    },
    async trainerAnswer(_h, q, chosen) {
      const evs = [2.1, 2.6, 2.4];
      const best = 1;
      return { chosen, freqs: [0.2, 0.7, 0.1], evs, best, lossBb: evs[best] - evs[chosen], lossPct: ((evs[best] - evs[chosen]) / q.pot) * 100, equity: 0.62 };
    },
    async trainerNext(_h, q) {
      return q.path.length >= 2 ? null : question(q.path.length + 2);
    },
    async trainerRecord(entry) {
      history.push(entry);
    },
    async trainerHistory(limit) {
      return history.slice(-limit);
    },
    async analysisStart(ids) {
      if (!ids.length) throw new BackendError("no hand");
      analysis = { ids, start: Date.now(), cancelled: false };
    },
    async analysisProgress(): Promise<AnalysisProgress | null> {
      if (!analysis) return null;
      const done = analysis.cancelled ? 0 : Math.min(analysis.ids.length, Math.floor((Date.now() - analysis.start) / 150));
      return {
        total: analysis.ids.length,
        done,
        running: !analysis.cancelled && done < analysis.ids.length,
        waitingForTables: false,
        rows: analysis.ids.slice(0, done).map((id, k) => ({
          handId: id,
          playedAt: 1793474056 + k * 60,
          analysis:
            k % 3 === 2
              ? null
              : {
                  heroCards: "AhKd",
                  heroRange: "cash 6max 100bb/BTN/open",
                  villainRange: "cash 6max 100bb/BB vs BTN/call",
                  spot: { board: "Qs7d2c", pot: 5.5, stack: 97.5 },
                  totalLossBb: k * 0.4,
                  decisions: [
                    { street: "flop", nodeKey: "flop · PFR · IP · first", played: "bet 1.82", best: "bet 1.82", lossBb: 0, lossPct: 0, approximate: false },
                    { street: "turn", nodeKey: "turn · PFR · IP · first", played: "check", best: "bet 4.5", lossBb: k * 0.4, lossPct: k * 2, approximate: false },
                  ],
                },
          error: k % 3 === 2 ? "not a heads-up pot seeing the flop with hero" : null,
        })),
      };
    },
    async analysisCancel() {
      if (analysis) analysis.cancelled = true;
    },
  };
}
