/** Drill history: counts per grade and average EV loss per day. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/card";
import { getBackend } from "@/lib/backend";
import type { Grade, HistoryEntry } from "@/lib/trainer";
import { LineChart } from "../tracker/LineChart";
import { GRADE_CLASS } from "./common";

const GRADES: Grade[] = ["best", "correct", "inaccuracy", "mistake", "blunder"];

export function History() {
  const { t } = useTranslation();
  const [h, setH] = useState<HistoryEntry[]>([]);
  useEffect(() => {
    void getBackend()
      .then((b) => b.trainerHistory(5000))
      .then((x) => setH(x as unknown as HistoryEntry[]));
  }, []);
  const days = useMemo(() => {
    const m = new Map<string, { n: number; loss: number }>();
    for (const e of h.filter((x) => x.kind === "postflop")) {
      const d = new Date(e.at * 1000).toISOString().slice(0, 10);
      const x = m.get(d) ?? { n: 0, loss: 0 };
      x.n++;
      x.loss += e.lossBb;
      m.set(d, x);
    }
    return [...m.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [h]);
  const x = useMemo(() => days.map((_, i) => i + 1), [days]);
  const lines = useMemo(() => [{ name: t("trainer.hist.avg_loss"), data: days.map(([, v]) => v.loss / v.n) }], [days, t]);
  const fmt = useMemo(() => (v: number) => `${v.toFixed(2)} bb`, []);
  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d2 p-d4 text-sm" data-testid="hist-counts">
        <p>{t("trainer.hist.total", { n: h.length })}</p>
        <p className="flex flex-wrap gap-d3">
          {GRADES.map((g) => (
            <span key={g} className={GRADE_CLASS[g]}>
              {t(`trainer.grade.${g}`)} : {h.filter((e) => e.grade === g).length}
            </span>
          ))}
        </p>
      </Card>
      {days.length > 0 ? (
        <Card className="p-d4">
          <LineChart x={x} lines={lines} xLabel={t("trainer.hist.days")} format={fmt} testId="hist-chart" />
          <p className="text-xs text-muted">{days.map(([d, v]) => `${d} : ${v.n}`).join(" · ")}</p>
        </Card>
      ) : (
        <p className="text-sm text-muted">{t("trainer.hist.none")}</p>
      )}
    </div>
  );
}
