/** Quiz: random push/fold spots (chip EV), you choose, scored against the equilibrium. */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { getBackend } from "@/lib/backend";
import { positionNames, type Spot } from "@/lib/icm";
import { cardStr, CELL_COMBOS, COMBOS, cellName } from "@/lib/range";

interface Q {
  spot: Spot;
  pos: string[];
  hero: number;
  pusher: number | null;
  cards: string;
  cell: number;
  act: number;
  fold: number;
}

async function makeQuestion(): Promise<Q> {
  const n = 2 + Math.floor(Math.random() * 5);
  const stacks = Array.from({ length: n }, () => Math.round((3 + Math.random() * 12) * 2) / 2);
  const spot: Spot = { stacks, sb: 0.5, bb: 1, ante: Math.random() < 0.5 ? 0 : 0.1, bbAnte: false, others: [], payouts: [], bounties: [], fgsDepth: 0 };
  const sol = await (await getBackend()).pushfoldSolve(spot, { maxIterations: 800, target: 0.002 });
  const pushNodes = sol.nodes.filter((x) => x.reach > 0.02);
  const node = pushNodes[Math.floor(Math.random() * pushNodes.length)] ?? sol.nodes[0];
  const combo = Math.floor(Math.random() * 1326);
  const cell = CELL_COMBOS.findIndex((cs) => cs.includes(combo));
  const d = node.decision;
  return {
    spot,
    pos: positionNames(n),
    hero: d.kind === "push" ? d.p : d.q,
    pusher: d.kind === "push" ? null : d.p,
    cards: cardStr(COMBOS[combo][0]) + cardStr(COMBOS[combo][1]),
    cell,
    act: node.evAct[cell],
    fold: node.evFold[cell],
  };
}

export function QuizTab() {
  const { t } = useTranslation();
  const [q, setQ] = useState<Q | null>(null);
  const [answer, setAnswer] = useState<boolean | null>(null);
  const [score, setScore] = useState({ good: 0, total: 0, lost: 0 });
  const [busy, setBusy] = useState(false);

  const next = () => {
    setBusy(true);
    setAnswer(null);
    void makeQuestion()
      .then(setQ)
      .finally(() => setBusy(false));
  };
  const pick = (act: boolean) => {
    if (!q || answer !== null) return;
    setAnswer(act);
    const best = q.act > q.fold;
    const lost = Math.max(0, act ? q.fold - q.act : q.act - q.fold);
    setScore((s) => ({ good: s.good + (act === best ? 1 : 0), total: s.total + 1, lost: s.lost + lost }));
  };
  const best = q ? q.act > q.fold : false;

  return (
    <Card className="flex flex-col gap-d3 p-d4" data-testid="quiz">
      <p className="text-sm text-muted">{t("icm.quiz.help")}</p>
      <p className="text-sm" data-testid="quiz-score">
        {t("icm.quiz.score", { good: score.good, total: score.total, lost: score.lost.toFixed(2) })}
      </p>
      {!q && (
        <Button className="self-start" onClick={next} disabled={busy} data-testid="quiz-start">
          {t("icm.quiz.start")}
        </Button>
      )}
      {q && (
        <>
          <table className="max-w-md text-sm">
            <tbody>
              {q.spot.stacks.map((s, i) => (
                <tr key={i} className={i === q.hero ? "font-semibold" : ""}>
                  <td className="py-0.5 pr-d3 text-muted">{q.pos[i]}</td>
                  <td className="tabular-nums">{s} bb</td>
                  <td className="pl-d2 text-xs text-muted">
                    {i === q.hero ? t("icm.quiz.you") : i === q.pusher ? t("icm.review.shoves") : q.pusher === null && i < q.hero ? t("icm.quiz.folds") : ""}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          <div className="flex items-center gap-d3">
            <span className="flex gap-0.5">
              <PlayingCard card={q.cards.slice(0, 2)} />
              <PlayingCard card={q.cards.slice(2)} />
            </span>
            <span className="text-sm text-muted">{q.spot.ante ? t("icm.quiz.ante", { a: q.spot.ante }) : t("icm.quiz.no_ante")}</span>
          </div>
          <div className="flex gap-d2">
            <Button onClick={() => pick(true)} disabled={answer !== null} data-testid="quiz-act">
              {q.pusher === null ? t("icm.pf.push") : t("icm.pf.call")}
            </Button>
            <Button variant="outline" onClick={() => pick(false)} disabled={answer !== null} data-testid="quiz-fold">
              {t("icm.review.choice.fold")}
            </Button>
          </div>
          {answer !== null && (
            <div className="flex flex-col gap-d2" data-testid="quiz-result">
              <p className={answer === best ? "font-semibold text-success" : "font-semibold text-warning"}>
                {answer === best ? t("icm.quiz.right") : t("icm.quiz.wrong")}{" "}
                {t("icm.quiz.detail", { hand: cellName(q.cell), a: q.act.toFixed(2), f: q.fold.toFixed(2) })}
              </p>
              <Button className="self-start" onClick={next} disabled={busy} data-testid="quiz-next">
                {t("icm.quiz.next")}
              </Button>
            </div>
          )}
        </>
      )}
    </Card>
  );
}
