import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Select } from "@/components/ui/input";
import { bb, int } from "@/lib/format";
import type { GridCell } from "@/lib/tracker";
import { useFilter, useGrid } from "./api";

type Metric = "bb100" | "hands" | "net";

/**
 * 13×13 starting-hand grid. Results use the diverging pair of the dataviz
 * reference palette (red = losing ↔ gray 0 ↔ blue = winning); hand counts use
 * the single-hue blue ramp. Every cell has a hover title with exact values.
 */
const LOSE = "#e34948";
const WIN = "#2a78d6";
const MID_DARK = "#383835";
const MID_LIGHT = "#f0efec";

function fill(v: number, max: number, metric: Metric, dark: boolean): string {
  const mid = dark ? MID_DARK : MID_LIGHT;
  if (max <= 0) return mid;
  const t = Math.min(1, Math.abs(v) / max);
  const pole = metric === "hands" || v >= 0 ? WIN : LOSE;
  return `color-mix(in oklab, ${pole} ${Math.round(t * 100)}%, ${mid})`;
}

export function Grid() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const [metric, setMetric] = useState<Metric>("bb100");
  const cells = useGrid(useFilter()).data;
  if (!cells) return null;
  const dark = document.documentElement.dataset.theme !== "light";
  const value = (c: GridCell) =>
    metric === "hands" ? c.hands : metric === "net" ? c.netBb : c.hands >= 5 ? (c.netBb * 100) / c.hands : 0;
  const max = Math.max(...cells.map((c) => Math.abs(value(c))));
  return (
    <div className="flex flex-col gap-d3">
      <div className="flex flex-wrap items-center gap-d3 text-sm">
        <label className="flex items-center gap-d2">
          {t("grid.metric")}
          <Select className="w-52" value={metric} onChange={(e) => setMetric(e.target.value as Metric)} data-testid="grid-metric">
            <option value="bb100">{t("grid.bb100")}</option>
            <option value="net">{t("grid.net")}</option>
            <option value="hands">{t("grid.hands")}</option>
          </Select>
        </label>
        <span className="flex items-center gap-d2 text-xs text-muted" aria-hidden>
          {metric === "hands" ? (
            <>
              0 <span className="h-2 w-24 rounded" style={{ background: `linear-gradient(90deg, ${dark ? MID_DARK : MID_LIGHT}, ${WIN})` }} /> {t("grid.max")}
            </>
          ) : (
            <>
              {t("grid.losing")}
              <span className="h-2 w-32 rounded" style={{ background: `linear-gradient(90deg, ${LOSE}, ${dark ? MID_DARK : MID_LIGHT}, ${WIN})` }} />
              {t("grid.winning")}
            </>
          )}
        </span>
      </div>
      <div className="grid w-full max-w-3xl grid-cols-13 gap-[2px]" style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }} data-testid="hand-grid">
        {cells.map((c) => {
          const v = value(c);
          const strong = max > 0 && Math.abs(v) / max > 0.55;
          return (
            <div
              key={c.class}
              className="flex aspect-square flex-col items-center justify-center rounded-sm text-[10px] leading-tight"
              style={{ background: c.hands ? fill(v, max, metric, dark) : "transparent", color: strong ? "#ffffff" : undefined }}
              title={`${c.class} · ${t("grid.tooltip", { n: c.hands, net: bb(c.netBb, lang), bb100: c.hands ? int((c.netBb * 100) / c.hands, lang) : "—" })}`}
              data-class={c.class}
            >
              <span className="font-semibold">{c.class}</span>
              {c.hands > 0 && (metric !== "bb100" || c.hands >= 5) && (
                <span className="opacity-80">{metric === "hands" ? c.hands : int(v, lang)}</span>
              )}
            </div>
          );
        })}
      </div>
      <p className="text-xs text-muted">{t("grid.help")}</p>
    </div>
  );
}
