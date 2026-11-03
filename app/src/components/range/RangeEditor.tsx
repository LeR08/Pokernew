/**
 * Shared range editor: 13×13 grid (click or drag, mouse or touch), weight
 * brush, suit selection per cell, text notation, "top X %" slider, live combo
 * count (board and dead cards removed) and the range library.
 *
 * The value is range notation text; the grid edits it via formatRange().
 */
import i18n from "i18next";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  CELL_COMBOS,
  COMBOS,
  cellName,
  comboCount,
  comboStr,
  formatRange,
  liveCount,
  NUM_COMBOS,
  parseRange,
  RangeParseError,
  topPercent,
  type Card,
  type Weights,
} from "@/lib/range";
import { RangeLibrary } from "./RangeLibrary";

const SUIT_SYM: Record<string, string> = { c: "♣", d: "♦", h: "♥", s: "♠" };
const pretty = (combo: string) => combo.replace(/([cdhs])/g, (s) => SUIT_SYM[s]);

export function useRangeWeights(text: string): { weights: Weights; error: string | null } {
  return useMemo(() => {
    try {
      return { weights: parseRange(text), error: null };
    } catch (e) {
      return { weights: new Float32Array(NUM_COMBOS), error: e instanceof RangeParseError ? e.token : String(e) };
    }
  }, [text]);
}

export function RangeEditor({
  value,
  onChange,
  dead = [],
  testId = "range-editor",
}: {
  value: string;
  onChange: (text: string) => void;
  /** Board + dead cards, removed from the combo count. */
  dead?: Card[];
  testId?: string;
}) {
  const { t } = useTranslation();
  const { weights, error } = useRangeWeights(value);
  const [brush, setBrush] = useState(100);
  const [suitMode, setSuitMode] = useState(false);
  const [suitCell, setSuitCell] = useState<number | null>(null);
  const [top, setTop] = useState(0);
  const stroke = useRef<{ action: "set" | "clear"; done: Set<number>; w: Weights } | null>(null);
  const grid = useRef<HTMLDivElement>(null);

  const deadSet = useMemo(() => new Set(dead), [dead]);
  const live = liveCount(weights, dead);
  const total = comboCount(weights);

  const commit = (w: Weights) => onChange(formatRange(w));

  const cellFill = (cell: number) => {
    const cs = CELL_COMBOS[cell];
    return cs.reduce((s, i) => s + weights[i], 0) / cs.length;
  };

  const apply = (cell: number) => {
    const s = stroke.current;
    if (!s || s.done.has(cell)) return;
    s.done.add(cell);
    for (const i of CELL_COMBOS[cell]) s.w[i] = s.action === "set" ? brush / 100 : 0;
    commit(s.w);
  };

  const cellAt = (x: number, y: number): number | null => {
    const el = document.elementFromPoint(x, y)?.closest("[data-cell]");
    return el ? Number((el as HTMLElement).dataset.cell) : null;
  };

  const onDown = (e: React.PointerEvent, cell: number) => {
    if (e.button !== 0) return;
    e.preventDefault();
    if (suitMode) {
      setSuitCell(cell);
      return;
    }
    const full = CELL_COMBOS[cell].every((i) => Math.abs(weights[i] - brush / 100) < 1e-6);
    stroke.current = { action: full ? "clear" : "set", done: new Set(), w: Float32Array.from(weights) };
    grid.current?.setPointerCapture(e.pointerId);
    apply(cell);
  };

  const onMove = (e: React.PointerEvent) => {
    if (!stroke.current) return;
    const c = cellAt(e.clientX, e.clientY);
    if (c !== null) apply(c);
  };

  useEffect(() => {
    const up = () => (stroke.current = null);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
    return () => {
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
    };
  }, []);

  const toggleCombo = (i: number) => {
    const w = Float32Array.from(weights);
    w[i] = w[i] > 0 ? 0 : brush / 100;
    commit(w);
  };

  const setTopPct = (p: number) => {
    setTop(p);
    const w = new Float32Array(NUM_COMBOS);
    for (const i of topPercent(p)) w[i] = brush / 100;
    commit(w);
  };

  return (
    <div className="flex flex-col gap-d3" data-testid={testId}>
      <div className="flex flex-wrap gap-d4">
        <div
          ref={grid}
          className="grid w-full max-w-[34rem] touch-none select-none gap-px rounded-md bg-border p-px"
          style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }}
          onPointerMove={onMove}
          onContextMenu={(e) => {
            e.preventDefault();
            const c = cellAt(e.clientX, e.clientY);
            if (c !== null) setSuitCell(c);
          }}
          role="grid"
          aria-label={t("range.grid")}
        >
          {Array.from({ length: 169 }, (_, cell) => {
            const f = cellFill(cell);
            const row = Math.floor(cell / 13);
            const col = cell % 13;
            const kind = row === col ? "pair" : row < col ? "suited" : "offsuit";
            return (
              <div
                key={cell}
                data-cell={cell}
                data-testid="range-cell"
                data-name={cellName(cell)}
                data-fill={f.toFixed(2)}
                role="gridcell"
                aria-selected={f > 0}
                title={`${cellName(cell)} · ${Math.round(f * 100)} %`}
                onPointerDown={(e) => onDown(e, cell)}
                className={cn(
                  "relative flex aspect-square cursor-pointer items-center justify-center overflow-hidden text-[10px] font-medium sm:text-xs",
                  kind === "pair" ? "bg-surface-alt" : "bg-surface",
                  suitCell === cell && "ring-2 ring-inset ring-fg",
                )}
              >
                <span className="absolute inset-x-0 bottom-0 bg-primary" style={{ height: `${f * 100}%` }} aria-hidden />
                <span className={cn("relative", f > 0.5 ? "text-primary-fg" : "text-fg")}>{cellName(cell)}</span>
              </div>
            );
          })}
        </div>

        <div className="flex min-w-56 flex-1 flex-col gap-d3 text-sm">
          <p data-testid="range-count">
            <span className="text-lg font-semibold">{t("range.combos", { n: fmt(live) })}</span>{" "}
            <span className="text-muted">
              ({pct1(total / NUM_COMBOS)}{live !== total ? ` · ${t("range.blocked", { n: fmt(total - live) })}` : ""})
            </span>
          </p>
          <label className="flex flex-col gap-1">
            {t("range.brush", { n: brush })}
            <input type="range" min={5} max={100} step={5} value={brush} onChange={(e) => setBrush(Number(e.target.value))} data-testid="range-brush" />
          </label>
          <label className="flex flex-col gap-1">
            {t("range.top", { n: top })}
            <input
              type="range"
              min={0}
              max={100}
              step={0.5}
              value={top}
              onChange={(e) => setTopPct(Number(e.target.value))}
              data-testid="range-top"
            />
          </label>
          <div className="flex flex-wrap gap-d2">
            <Button size="sm" variant={suitMode ? "default" : "outline"} onClick={() => setSuitMode(!suitMode)} data-testid="range-suit-mode">
              {t("range.suits")}
            </Button>
            <Button size="sm" variant="outline" onClick={() => commit(new Float32Array(NUM_COMBOS).fill(1))}>
              {t("range.all")}
            </Button>
            <Button size="sm" variant="outline" onClick={() => onChange("")} data-testid="range-clear">
              {t("range.clear")}
            </Button>
          </div>
          {suitCell !== null && (
            <div className="rounded-md border border-border p-d2" data-testid="range-suit-panel">
              <div className="mb-1 flex items-center justify-between">
                <span className="font-medium">{cellName(suitCell)}</span>
                <button className="text-xs text-muted underline" onClick={() => setSuitCell(null)}>
                  {t("range.close")}
                </button>
              </div>
              <div className="flex flex-wrap gap-1">
                {CELL_COMBOS[suitCell].map((i) => {
                  const on = weights[i] > 0;
                  const blocked = COMBOS[i].some((c) => deadSet.has(c));
                  return (
                    <button
                      key={i}
                      data-testid="range-combo"
                      aria-pressed={on}
                      onClick={() => toggleCombo(i)}
                      className={cn(
                        "rounded border px-1.5 py-0.5 font-mono text-xs",
                        on ? "border-primary bg-primary text-primary-fg" : "border-border",
                        blocked && "line-through opacity-50",
                      )}
                    >
                      {pretty(comboStr(i))}
                      {on && weights[i] < 1 ? ` ${Math.round(weights[i] * 100)}%` : ""}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
          <RangeLibrary value={value} onLoad={onChange} />
        </div>
      </div>
      <label className="flex flex-col gap-1 text-sm">
        {t("range.text")}
        <textarea
          className={cn(
            "min-h-16 w-full rounded-md border bg-bg p-d2 font-mono text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary",
            error ? "border-danger" : "border-border",
          )}
          value={value}
          spellCheck={false}
          onChange={(e) => onChange(e.target.value)}
          placeholder="TT+, AJs+, KQs, AQo+, KQo:50%"
          data-testid="range-text"
          aria-invalid={!!error}
        />
        {error && (
          <span className="text-xs text-danger" data-testid="range-error">
            {t("range.invalid", { token: error })}
          </span>
        )}
      </label>
    </div>
  );
}

const fmt = (n: number) => (Number.isInteger(n) ? String(n) : n.toFixed(1));
const pct1 = (x: number) =>
  `${new Intl.NumberFormat(i18n.language === "fr" ? "fr-FR" : "en-GB", { maximumFractionDigits: 1, minimumFractionDigits: 1 }).format(x * 100)} %`;
