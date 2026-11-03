import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { stateAt, totalSteps } from "@/modules/replayer/replay";
import type { Hand } from "@/lib/tracker";

const file = join(__dirname, "fixtureHands.json");
const hands: Hand[] = existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : [];

describe("replayer", () => {
  it("has the fixture hands (run tools/export_fixture_hands.sh)", () => {
    expect(hands.length).toBeGreaterThanOrEqual(496);
  });

  it("replays every fixture hand to a consistent end state", () => {
    for (const h of hands) {
      const n = totalSteps(h);
      for (let step = 0; step <= n; step++) {
        const st = stateAt(h, step);
        for (const s of st.seats) expect(s.stack).toBeGreaterThanOrEqual(0);
        if (step === h.actions.length) {
          // Everything invested is on the table (pot + bets).
          const onTable = st.pot + st.seats.reduce((a, s) => a + s.bet, 0);
          expect(onTable, h.hand_no).toBe(h.total_pot);
        }
      }
      const end = stateAt(h, n);
      expect(end.finished).toBe(true);
      expect(end.board).toEqual(h.board);
      h.players.forEach((p, i) => {
        expect(end.seats[i].stack, `${h.hand_no} ${p.name}`).toBe(p.stack - p.invested + p.collected);
      });
    }
  });

  it("deals the board street by street", () => {
    const h = hands.find((x) => x.board.length === 5)!;
    const firstTurn = h.actions.findIndex((a) => a.street === "turn");
    expect(stateAt(h, firstTurn).board).toHaveLength(3);
    expect(stateAt(h, firstTurn + 1).board).toHaveLength(4);
  });
});
