/**
 * Equity Lab charts (plain SVG).
 *
 * Colours: the 8 categorical slots of the dataviz reference palette, in fixed
 * order, one per player (player 1 = slot 1 everywhere: legend, table, curve).
 * Validated with validate_palette.js against our surfaces (light #ffffff, dark
 * #171a21): every check passes; three light slots are under 3:1 contrast, so
 * the legend carries each player's equity as text and the results table is
 * the table view (relief rule). Players 9 and 10 get no curve (no 9th hue);
 * their numbers stay in the table.
 */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { pct } from "./format";

const SERIES = {
  light: ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"],
  dark: ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181", "#008300", "#9085e9", "#e66767"],
};

export function seriesColour(i: number): string | undefined {
  const mode = document.documentElement.dataset.theme === "light" ? "light" : "dark";
  return SERIES[mode][i];
}

export interface Curve {
  name: string;
  /** (cumulative share of the range 0–1, equity 0–1), sorted by x. */
  points: [number, number][];
  /** Player index, for the colour. */
  index: number;
  label: string;
}

const W = 640;
const H = 240;
const PAD = { l: 44, r: 12, t: 12, b: 28 };
const px = (x: number) => PAD.l + x * (W - PAD.l - PAD.r);
const py = (y: number) => PAD.t + (1 - y) * (H - PAD.t - PAD.b);

/** y at x on a step curve. */
function at(points: [number, number][], x: number): number | null {
  if (!points.length) return null;
  let lo = 0;
  let hi = points.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (points[mid][0] < x) lo = mid + 1;
    else hi = mid;
  }
  return points[lo][1];
}

/** Equity distribution: each range's combos sorted from best to worst. */
export function EquityCurves({ curves }: { curves: Curve[] }) {
  const { t } = useTranslation();
  const [hover, setHover] = useState<number | null>(null);
  const shown = curves.filter((c) => 8 > c.index);
  const paths = useMemo(
    () => shown.map((c) => c.points.map(([x, y], i) => `${i ? "L" : "M"}${px(x).toFixed(1)},${py(y).toFixed(1)}`).join("")),
    [shown],
  );
  if (!shown.length) return null;
  const onMove = (e: React.PointerEvent<SVGSVGElement>) => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = ((e.clientX - r.left) / r.width) * W;
    const v = (x - PAD.l) / (W - PAD.l - PAD.r);
    setHover(v >= 0 && v <= 1 ? v : null);
  };
  return (
    <figure className="flex flex-col gap-d2" data-testid="equity-curves">
      <figcaption className="text-sm font-medium">{t("equity.curve_title")}</figcaption>
      <ul className="flex flex-wrap gap-d3 text-xs text-muted">
        {shown.map((c) => (
          <li key={c.index} className="flex items-center gap-1">
            <span className="inline-block h-1 w-3.5 rounded" style={{ background: seriesColour(c.index) }} aria-hidden />
            <span className="text-fg">{c.name}</span> {c.label}
          </li>
        ))}
      </ul>
      <div className="relative">
        <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label={t("equity.curve_title")} onPointerMove={onMove} onPointerLeave={() => setHover(null)}>
          {[0, 0.25, 0.5, 0.75, 1].map((y) => (
            <g key={y}>
              <line x1={PAD.l} x2={W - PAD.r} y1={py(y)} y2={py(y)} className="stroke-border" strokeWidth={1} />
              <text x={PAD.l - 6} y={py(y) + 3} textAnchor="end" className="fill-muted text-[10px]">
                {y * 100} %
              </text>
            </g>
          ))}
          {[0, 0.5, 1].map((x) => (
            <text key={x} x={px(x)} y={H - 8} textAnchor="middle" className="fill-muted text-[10px]">
              {x * 100} %
            </text>
          ))}
          {paths.map((d, i) => (
            <path key={shown[i].index} d={d} fill="none" stroke={seriesColour(shown[i].index)} strokeWidth={2} strokeLinejoin="round" />
          ))}
          {hover !== null && (
            <>
              <line x1={px(hover)} x2={px(hover)} y1={PAD.t} y2={H - PAD.b} className="stroke-muted" strokeWidth={1} />
              {shown.map((c) => {
                const y = at(c.points, hover);
                return y === null ? null : (
                  <circle key={c.index} cx={px(hover)} cy={py(y)} r={4} fill={seriesColour(c.index)} className="stroke-surface" strokeWidth={2} />
                );
              })}
            </>
          )}
        </svg>
        {hover !== null && (
          <div
            className="pointer-events-none absolute top-1 rounded border border-border bg-surface px-2 py-1 text-xs shadow"
            style={{ left: `${Math.min(70, hover * 100)}%` }}
            data-testid="curve-tooltip"
          >
            <div className="text-muted">{t("equity.curve_x", { p: Math.round(hover * 100) })}</div>
            {shown.map((c) => {
              const y = at(c.points, hover);
              return (
                <div key={c.index} className="flex items-center gap-1">
                  <span className="inline-block h-2 w-2 rounded-full" style={{ background: seriesColour(c.index) }} aria-hidden />
                  {c.name} <span className="ml-auto font-medium">{y === null ? "—" : pct(y)}</span>
                </div>
              );
            })}
          </div>
        )}
      </div>
      <p className="text-xs text-muted">{t("equity.curve_help")}</p>
    </figure>
  );
}

/** Histogram of a range's combos by equity bucket (one series: slot 1). */
export function EquityHistogram({ buckets }: { buckets: number[] }) {
  const { t } = useTranslation();
  const [hover, setHover] = useState<number | null>(null);
  const max = Math.max(...buckets, 1e-9);
  const total = buckets.reduce((a, b) => a + b, 0);
  const bw = (W - PAD.l - PAD.r) / buckets.length;
  return (
    <figure className="flex flex-col gap-d2" data-testid="equity-histogram">
      <figcaption className="text-sm font-medium">{t("equity.hist_title")}</figcaption>
      <div className="relative">
        <svg viewBox={`0 0 ${W} ${H}`} className="w-full" role="img" aria-label={t("equity.hist_title")}>
          <line x1={PAD.l} x2={W - PAD.r} y1={py(0)} y2={py(0)} className="stroke-border" />
          {buckets.map((v, i) => {
            const h = (v / max) * (H - PAD.t - PAD.b);
            const x = PAD.l + i * bw + 1;
            const y = py(0) - h;
            return (
              <g key={i} onPointerEnter={() => setHover(i)} onPointerLeave={() => setHover(null)}>
                <rect x={PAD.l + i * bw} y={PAD.t} width={bw} height={H - PAD.t - PAD.b} fill="transparent" />
                {h > 0 && (
                  <path
                    d={`M${x},${py(0)} V${y + Math.min(4, h)} Q${x},${y} ${x + Math.min(4, h)},${y} H${x + bw - 2 - Math.min(4, h)} Q${x + bw - 2},${y} ${x + bw - 2},${y + Math.min(4, h)} V${py(0)} Z`}
                    fill={seriesColour(0)}
                    opacity={hover === null || hover === i ? 1 : 0.6}
                  />
                )}
                <text x={x + bw / 2 - 1} y={H - 8} textAnchor="middle" className="fill-muted text-[10px]">
                  {i * 10}–{i * 10 + 10}
                </text>
              </g>
            );
          })}
        </svg>
        {hover !== null && (
          <div
            className="pointer-events-none absolute top-1 rounded border border-border bg-surface px-2 py-1 text-xs shadow"
            style={{ left: `${Math.min(75, hover * 10)}%` }}
            data-testid="hist-tooltip"
          >
            {t("equity.hist_tip", { from: hover * 10, to: hover * 10 + 10 })}:{" "}
            <span className="font-medium">
              {buckets[hover].toFixed(1)} combos ({total ? pct(buckets[hover] / total) : "0 %"})
            </span>
          </div>
        )}
      </div>
    </figure>
  );
}
