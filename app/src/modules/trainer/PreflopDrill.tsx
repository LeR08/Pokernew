/** Preflop drills from the chart library: a random situation and hand, graded by the chart's frequencies. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { comboStr, NUM_COMBOS, tryParseRange } from "@/lib/range";
import type { Grade } from "@/lib/trainer";
import { cn } from "@/lib/utils";
import { GRADE_CLASS, record } from "./common";

interface Situation {
  /** e.g. "BTN vs CO" or "UTG" */
  name: string;
  /** option label → library file */
  options: { id: string; file: string }[];
}

/** Groups a folder's files into situations: "X/open", "X vs Y/{3bet,call}", "X vs 3bet/{4bet,call}", "BB vs limp/raise". */
export function situationsOf(folder: string, names: string[]): Situation[] {
  const m = new Map<string, { id: string; file: string }[]>();
  for (const n of names) {
    if (!n.startsWith(`${folder}/`)) continue;
    const rest = n.slice(folder.length + 1);
    const k = rest.lastIndexOf("/");
    if (k < 0) continue;
    const sit = rest.slice(0, k);
    const opt = rest.slice(k + 1);
    m.set(sit, [...(m.get(sit) ?? []), { id: opt, file: n }]);
  }
  return [...m.entries()].map(([name, options]) => ({ name, options }));
}

function gradeByFreq(chosen: number, freqs: number[]): Grade {
  const max = Math.max(...freqs);
  if (chosen >= max - 1e-3) return "best";
  if (chosen >= 0.25) return "correct";
  if (chosen >= 0.05) return "inaccuracy";
  if (chosen > 0) return "mistake";
  return "blunder";
}

export function PreflopDrill() {
  const { t } = useTranslation();
  const [names, setNames] = useState<string[]>([]);
  const [folder, setFolder] = useState("");
  const [q, setQ] = useState<{ sit: Situation; combo: number; freqs: number[]; labels: string[] } | null>(null);
  const [chosen, setChosen] = useState<number | null>(null);
  const [stats, setStats] = useState({ n: 0, good: 0 });

  useEffect(() => {
    void getBackend()
      .then((b) => b.rangesList())
      .then((n) => {
        setNames(n);
        const f = [...new Set(n.filter((x) => x.includes("/")).map((x) => x.split("/")[0]))].sort();
        setFolder(f.find((x) => x.startsWith("cash")) ?? f[0] ?? "");
      });
  }, []);
  const folders = useMemo(() => [...new Set(names.filter((x) => x.includes("/")).map((x) => x.split("/")[0]))].sort(), [names]);
  const sits = useMemo(() => situationsOf(folder, names), [folder, names]);

  const deal = async () => {
    if (!sits.length) return;
    const b = await getBackend();
    for (let tries = 0; tries < 20; tries++) {
      const sit = sits[Math.floor(Math.random() * sits.length)];
      const ws = await Promise.all(sit.options.map(async (o) => tryParseRange(await b.rangeLoad(o.file)) ?? new Float32Array(NUM_COMBOS)));
      // hands that play here: any option > 0; one question in four is a hand that folds
      const playing: number[] = [];
      for (let i = 0; i < NUM_COMBOS; i++) if (ws.some((w) => w[i] > 0)) playing.push(i);
      if (!playing.length) continue;
      const combo = Math.random() < 0.25 ? Math.floor(Math.random() * NUM_COMBOS) : playing[Math.floor(Math.random() * playing.length)];
      const freqs = ws.map((w) => w[combo]);
      const fold = Math.max(0, 1 - freqs.reduce((a, x) => a + x, 0));
      const foldLabel = sit.name.startsWith("BB vs limp") ? "check" : "fold";
      setQ({ sit, combo, freqs: [...freqs, fold], labels: [...sit.options.map((o) => o.id), foldLabel] });
      setChosen(null);
      return;
    }
  };

  const choose = (k: number) => {
    if (!q || chosen !== null) return;
    setChosen(k);
    const g = gradeByFreq(q.freqs[k], q.freqs);
    setStats((s) => ({ n: s.n + 1, good: s.good + (g === "best" || g === "correct" ? 1 : 0) }));
    record({ kind: "preflop", nodeKey: `${folder} · ${q.sit.name}`, grade: g, lossBb: 0, lossPct: (1 - q.freqs[k]) * 100 });
  };
  const g = q && chosen !== null ? gradeByFreq(q.freqs[chosen], q.freqs) : null;
  const cards = q ? comboStr(q.combo) : "";

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-wrap items-end gap-d3 p-d4 text-sm">
        <label className="flex min-w-64 flex-col gap-1">
          {t("trainer.pre.folder")}
          <Select value={folder} onChange={(e) => setFolder(e.target.value)} data-testid="pre-folder">
            {folders.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </Select>
        </label>
        <span className="text-muted">{t("trainer.pre.count", { n: sits.length })}</span>
        <Button onClick={() => void deal()} disabled={!sits.length} data-testid="pre-start">
          {q ? t("trainer.new_question") : t("trainer.start")}
        </Button>
        <span className="text-xs text-muted" data-testid="pre-stats">
          {t("trainer.pre.stats", { good: stats.good, n: stats.n })}
        </span>
      </Card>
      {q && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="pre-question">
          <p className="text-sm font-medium">{t("trainer.pre.situation", { s: q.sit.name })}</p>
          <span className="flex gap-0.5">
            <PlayingCard card={cards.slice(0, 2)} />
            <PlayingCard card={cards.slice(2)} />
          </span>
          <div className="flex flex-wrap gap-d2">
            {q.labels.map((l, k) => (
              <Button key={l} variant={chosen === k ? "default" : "outline"} disabled={chosen !== null} onClick={() => choose(k)} data-testid="pre-action">
                {l}
              </Button>
            ))}
          </div>
          {g && (
            <div className="flex flex-col gap-d2" data-testid="pre-answer">
              <p className={cn("font-semibold", GRADE_CLASS[g])}>{t(`trainer.grade.${g}`)}</p>
              <p className="text-sm">{q.labels.map((l, k) => `${l} ${(q.freqs[k] * 100).toFixed(0)} %`).join(" · ")}</p>
              <Button className="self-start" onClick={() => void deal()} data-testid="pre-next">
                {t("trainer.new_question")}
              </Button>
            </div>
          )}
        </Card>
      )}
      <p className="text-xs text-muted">{t("trainer.pre.help")}</p>
    </div>
  );
}
