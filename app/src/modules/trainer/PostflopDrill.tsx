/** Postflop drills from the solution cache: node, street or full-hand mode, timed, weak spots. */
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { actionLabel, type CacheEntry } from "@/lib/solver";
import { grade, type Answer, type DrillMode, type Grade, type Question } from "@/lib/trainer";
import { cn } from "@/lib/utils";
import { GRADE_CLASS, record, useThresholds } from "./common";

export function PostflopDrill() {
  const { t } = useTranslation();
  const th = useThresholds();
  const [list, setList] = useState<CacheEntry[]>([]);
  const [sel, setSel] = useState("");
  const [family, setFamily] = useState(false);
  const [mode, setMode] = useState<DrillMode>("node");
  const [hero, setHero] = useState<"" | "0" | "1">("");
  const [timed, setTimed] = useState(0);
  const [weak, setWeak] = useState(false);
  const [q, setQ] = useState<Question | null>(null);
  const [hash, setHash] = useState("");
  const [ans, setAns] = useState<Answer | null>(null);
  const [left, setLeft] = useState(0);
  const [stats, setStats] = useState({ n: 0, loss: 0, grades: {} as Partial<Record<Grade, number>> });
  const [error, setError] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    void getBackend()
      .then((b) => b.solverCacheList())
      .then((l) => {
        setList(l);
        if (l[0]) setSel(l[0].hash);
      });
  }, []);

  const pool = () => {
    const base = list.find((x) => x.hash === sel);
    if (!base) return [];
    if (!family) return [base];
    const same = (x: CacheEntry) => x.spot.oopRange === base.spot.oopRange && x.spot.ipRange === base.spot.ipRange && x.spot.pot === base.spot.pot && x.spot.stack === base.spot.stack && x.spot.board.length === base.spot.board.length;
    return list.filter(same);
  };

  const deal = () => {
    const p = pool();
    if (!p.length) return;
    const e = p[Math.floor(Math.random() * p.length)];
    setError(null);
    setAns(null);
    void getBackend()
      .then((b) => b.trainerDeal(e.hash, mode, hero === "" ? null : (Number(hero) as 0 | 1), weak))
      .then((x) => {
        setHash(e.hash);
        setQ(x);
        startTimer();
      })
      .catch((x: Error) => setError(x.message));
  };

  const startTimer = () => {
    if (timer.current) clearInterval(timer.current);
    if (!timed) return;
    setLeft(timed);
    timer.current = setInterval(() => setLeft((s) => s - 1), 1000);
  };
  useEffect(() => () => void (timer.current && clearInterval(timer.current)), []);

  const choose = (k: number) => {
    if (!q || ans) return;
    if (timer.current) clearInterval(timer.current);
    void getBackend()
      .then((b) => b.trainerAnswer(hash, q, k))
      .then((a) => {
        setAns(a);
        const g = grade(a.lossPct, a.chosen === a.best, th);
        setStats((s) => ({ n: s.n + 1, loss: s.loss + a.lossBb, grades: { ...s.grades, [g]: (s.grades[g] ?? 0) + 1 } }));
        record({ kind: "postflop", nodeKey: q.nodeKey, grade: g, lossBb: a.lossBb, lossPct: a.lossPct });
      });
  };

  // timed out: the worst action is recorded
  useEffect(() => {
    if (timed && q && !ans && left <= 0 && timer.current) {
      clearInterval(timer.current);
      timer.current = null;
      void getBackend()
        .then((b) => b.trainerAnswer(hash, q, 0))
        .then((a) => choose(a.evs.indexOf(Math.min(...a.evs))));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [left]);

  const next = () => {
    if (mode === "hand" && q && ans) {
      void getBackend()
        .then((b) => b.trainerNext(hash, q, ans.chosen))
        .then((n) => {
          setAns(null);
          if (n) {
            setQ(n);
            startTimer();
          } else deal();
        });
    } else deal();
  };

  const g = ans ? grade(ans.lossPct, ans.chosen === ans.best, th) : null;
  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4 text-sm">
        {list.length === 0 ? (
          <p className="text-muted" data-testid="drill-empty">
            {t("trainer.post.no_solution")}
          </p>
        ) : (
          <div className="flex flex-wrap items-end gap-d3">
            <label className="flex min-w-64 flex-col gap-1">
              {t("trainer.post.solution")}
              <Select value={sel} onChange={(e) => setSel(e.target.value)} data-testid="drill-solution">
                {list.map((x) => (
                  <option key={x.hash} value={x.hash}>
                    {x.name}
                  </option>
                ))}
              </Select>
            </label>
            <label className="flex items-center gap-1">
              <input type="checkbox" checked={family} onChange={(e) => setFamily(e.target.checked)} />
              {t("trainer.post.family")}
            </label>
            <label className="flex flex-col gap-1">
              {t("trainer.post.mode")}
              <Select className="w-44" value={mode} onChange={(e) => setMode(e.target.value as DrillMode)} data-testid="drill-mode">
                <option value="node">{t("trainer.post.modes.node")}</option>
                <option value="street">{t("trainer.post.modes.street")}</option>
                <option value="hand">{t("trainer.post.modes.hand")}</option>
              </Select>
            </label>
            <label className="flex flex-col gap-1">
              {t("trainer.post.side")}
              <Select className="w-32" value={hero} onChange={(e) => setHero(e.target.value as "" | "0" | "1")}>
                <option value="">{t("trainer.post.any")}</option>
                <option value="0">{t("trainer.post.oop")}</option>
                <option value="1">{t("trainer.post.ip")}</option>
              </Select>
            </label>
            <label className="flex flex-col gap-1">
              {t("trainer.post.timed")}
              <Input type="number" min={0} max={120} className="w-20" value={timed} onChange={(e) => setTimed(Number(e.target.value))} />
            </label>
            <label className="flex items-center gap-1">
              <input type="checkbox" checked={weak} onChange={(e) => setWeak(e.target.checked)} data-testid="drill-weak" />
              {t("trainer.post.weak")}
            </label>
            <Button onClick={deal} data-testid="drill-start">
              {q ? t("trainer.new_question") : t("trainer.start")}
            </Button>
          </div>
        )}
        {error && <p className="text-danger">{error}</p>}
        <p className="text-xs text-muted" data-testid="drill-stats">
          {t("trainer.session", { n: stats.n, avg: stats.n ? (stats.loss / stats.n).toFixed(2) : "0", best: (stats.grades.best ?? 0) + (stats.grades.correct ?? 0) })}
        </p>
      </Card>

      {q && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="drill-question">
          <div className="flex flex-wrap items-center gap-d3 text-sm">
            <span className="flex gap-0.5">
              {q.board.map((c) => (
                <PlayingCard key={c} card={c} />
              ))}
            </span>
            <span>{t("solver.viewer.pot", { pot: q.pot.toFixed(2) })}</span>
            <span className="font-medium">{t("trainer.post.you_are", { who: q.player === 0 ? "OOP" : "IP" })}</span>
            <span className="flex gap-0.5" data-testid="drill-combo">
              <PlayingCard card={q.combo.slice(0, 2)} />
              <PlayingCard card={q.combo.slice(2)} />
            </span>
            {timed > 0 && !ans && <span className={cn("tabular-nums", left <= 3 && "text-danger")}>{t("trainer.post.left", { s: Math.max(0, left) })}</span>}
          </div>
          {q.history.length > 0 && <p className="text-xs text-muted">{q.history.map((h) => `${h.who === "card" ? "→" : h.who} ${h.label}`).join(" · ")}</p>}
          <div className="flex flex-wrap gap-d2">
            {q.actions.map((a, k) => (
              <Button key={k} variant={ans?.chosen === k ? "default" : "outline"} disabled={!!ans} onClick={() => choose(k)} data-testid="drill-action">
                {actionLabel(a, t)}
              </Button>
            ))}
          </div>
          {ans && g && (
            <div className="flex flex-col gap-d2" data-testid="drill-answer">
              <p className={cn("text-lg font-semibold", GRADE_CLASS[g])} data-testid="drill-grade">
                {t(`trainer.grade.${g}`)} · {t("trainer.post.loss", { bb: ans.lossBb.toFixed(2), pct: ans.lossPct.toFixed(1) })}
              </p>
              <table className="max-w-lg text-sm">
                <thead className="text-left text-xs text-muted">
                  <tr>
                    <th>{t("trainer.post.action")}</th>
                    <th className="text-right">{t("trainer.post.gto")}</th>
                    <th className="text-right">{t("trainer.post.ev")}</th>
                  </tr>
                </thead>
                <tbody>
                  {q.actions.map((a, k) => (
                    <tr key={k} className={cn(k === ans.best && "font-semibold", k === ans.chosen && "text-primary")}>
                      <td className="py-0.5">{actionLabel(a, t)}</td>
                      <td className="text-right tabular-nums">{(ans.freqs[k] * 100).toFixed(1)} %</td>
                      <td className="text-right tabular-nums">{ans.evs[k].toFixed(2)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              <p className="text-xs text-muted">{t("trainer.post.equity", { e: (ans.equity * 100).toFixed(1) })}</p>
              <Button className="self-start" onClick={next} data-testid="drill-next">
                {mode === "hand" ? t("trainer.post.continue") : t("trainer.new_question")}
              </Button>
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
