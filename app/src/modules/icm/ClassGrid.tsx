/** 13×13 grid of one decision: share of each hand played (push or call), EV on hover. */
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cellName } from "@/lib/range";
import { cn } from "@/lib/utils";

export function ClassGrid({
  freq,
  evAct,
  evFold,
  fmt,
  act,
  testId = "class-grid",
}: {
  freq: number[];
  evAct: number[];
  evFold: number[];
  fmt: (x: number) => string;
  act: string;
  testId?: string;
}) {
  const { t } = useTranslation();
  const [hover, setHover] = useState<number | null>(null);
  return (
    <div className="flex flex-col gap-d2">
      <div className="grid w-full max-w-[34rem] gap-px rounded-md bg-border p-px" style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }} data-testid={testId} role="grid">
        {freq.map((f, c) => (
          <div
            key={c}
            role="gridcell"
            data-name={cellName(c)}
            data-freq={f.toFixed(2)}
            onPointerEnter={() => setHover(c)}
            onPointerLeave={() => setHover(null)}
            className={cn("relative flex aspect-square items-center justify-center overflow-hidden text-[10px] sm:text-xs", Math.floor(c / 13) === c % 13 ? "bg-surface-alt" : "bg-surface")}
            title={`${cellName(c)} · ${Math.round(f * 100)} %`}
          >
            <span className="absolute inset-x-0 bottom-0 bg-primary" style={{ height: `${f * 100}%` }} aria-hidden />
            <span className={cn("relative", f > 0.5 ? "text-primary-fg" : "text-fg")}>{cellName(c)}</span>
          </div>
        ))}
      </div>
      <p className="h-5 text-sm" data-testid="grid-ev">
        {hover !== null &&
          t("icm.pf.ev_line", { hand: cellName(hover), act, f: Math.round(freq[hover] * 100), a: fmt(evAct[hover]), b: fmt(evFold[hover]), d: fmt(evAct[hover] - evFold[hover]) })}
      </p>
    </div>
  );
}

/** Average weight of each class in a 1 326-combo range. */
export function classFreq(weights: Float32Array, cellCombos: number[][]): number[] {
  return cellCombos.map((cs) => cs.reduce((s, i) => s + weights[i], 0) / cs.length);
}
