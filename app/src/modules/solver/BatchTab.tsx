/** Batch: the current spot (ranges, pot, tree) over a set of flops, with a report per flop and per texture. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import type { FlopFilter } from "@/lib/equity";
import { actionLabel, type BatchProgress, type BatchRow } from "@/lib/solver";
import { useSolverUi } from "./store";
import { actionColour } from "./Viewer";

/** Texture group of a flop: suits × pairing. */
function texture(flop: string): string {
  const suits = new Set([flop[1], flop[3], flop[5]]).size;
  const ranks = new Set([flop[0], flop[2], flop[4]]).size;
  return `${suits === 1 ? "monotone" : suits === 2 ? "two_tone" : "rainbow"}${ranks < 3 ? "_paired" : ""}`;
}

/** Evenly spread sample of at most n items. */
const sample = <T,>(xs: T[], n: number) => (n <= 0 || xs.length <= n ? xs : Array.from({ length: n }, (_, k) => xs[Math.floor((k * xs.length) / n)]));

export function BatchTab() {
  const { t } = useTranslation();
  const ui = useSolverUi();
  const [filter, setFilter] = useState<FlopFilter>({});
  const [max, setMax] = useState(25);
  const [flops, setFlops] = useState<string[]>([]);
  const [p, setP] = useState<BatchProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getBackend()
      .then((b) => b.solverFlops(filter))
      .then(setFlops);
  }, [filter]);
  useEffect(() => {
    let alive = true;
    const tick = () =>
      void getBackend()
        .then((b) => b.solverBatchProgress())
        .then((x) => alive && setP(x));
    tick();
    const h = setInterval(tick, 700);
    return () => {
      alive = false;
      clearInterval(h);
    };
  }, []);

  const chosen = sample(flops, max);
  const start = () => {
    setError(null);
    void getBackend()
      .then((b) => b.solverBatchStart(ui.spot, ui.params, chosen, ui.name.trim() || t("solver.batch.default_name")))
      .catch((e: Error) => setError(e.message));
  };
  const groups = useMemo(() => {
    const m = new Map<string, BatchRow[]>();
    for (const r of p?.rows ?? []) m.set(texture(r.flop), [...(m.get(texture(r.flop)) ?? []), r]);
    return [...m.entries()];
  }, [p]);
  const sel = (k: keyof FlopFilter, v: string) => setFilter({ ...filter, [k]: v === "" ? null : k === "connected" ? v === "yes" : v });

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <p className="text-sm text-muted">{t("solver.batch.help")}</p>
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex flex-col gap-1">
            {t("equity.tex.suits")}
            <Select className="w-40" value={filter.suits ?? ""} onChange={(e) => sel("suits", e.target.value)} data-testid="batch-suits">
              <option value="">{t("equity.tex.any")}</option>
              <option value="monotone">{t("equity.tex.monotone")}</option>
              <option value="two_tone">{t("equity.tex.two_tone")}</option>
              <option value="rainbow">{t("equity.tex.rainbow")}</option>
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            {t("equity.tex.pairing")}
            <Select className="w-40" value={filter.pairing ?? ""} onChange={(e) => sel("pairing", e.target.value)}>
              <option value="">{t("equity.tex.any")}</option>
              <option value="unpaired">{t("equity.tex.unpaired")}</option>
              <option value="paired">{t("equity.tex.paired")}</option>
              <option value="trips">{t("equity.tex.trips")}</option>
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.batch.max")}
            <Input type="number" min={1} max={1755} className="w-24" value={max} onChange={(e) => setMax(Number(e.target.value))} data-testid="batch-max" />
          </label>
          <span className="text-muted" data-testid="batch-count">
            {t("solver.batch.count", { n: chosen.length, of: flops.length })}
          </span>
          {p?.running ? (
            <Button variant="danger" onClick={() => void getBackend().then((b) => b.solverBatchCancel())} data-testid="batch-cancel">
              {t("solver.cancel")}
            </Button>
          ) : (
            <Button onClick={start} disabled={chosen.length === 0} data-testid="batch-start">
              {t("solver.batch.start")}
            </Button>
          )}
        </div>
        <p className="text-xs text-muted">{t("solver.batch.uses", { board: ui.spot.board })}</p>
        {error && <p className="text-sm text-danger">{error}</p>}
        {p && (
          <p className="text-sm" data-testid="batch-progress">
            {t("solver.batch.progress", { done: p.done, total: p.total })}
            {p.current ? ` · ${p.current}` : ""}
          </p>
        )}
        {p?.errors.map((e) => (
          <p key={e} className="text-xs text-danger">
            {e}
          </p>
        ))}
      </Card>
      {p && p.rows.length > 0 && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="batch-report">
          <h2 className="font-semibold">{t("solver.batch.by_texture")}</h2>
          <table className="max-w-2xl text-sm">
            <tbody>
              {groups.map(([g, rows]) => {
                const n = rows[0].totals.length;
                const avg = Array.from({ length: n }, (_, a) => rows.reduce((s, r) => s + (r.totals[a] ?? 0), 0) / rows.length);
                return (
                  <tr key={g}>
                    <td className="py-0.5 pr-d3">{t(`solver.batch.tex.${g}`)}</td>
                    <td className="w-80">
                      <span className="flex h-3 overflow-hidden rounded">
                        {avg.map((f, a) => (
                          <span key={a} style={{ width: `${f * 100}%`, background: actionColour(a) }} title={`${actionLabel(rows[0].actions[a], t)} ${(f * 100).toFixed(0)} %`} />
                        ))}
                      </span>
                    </td>
                    <td className="pl-d2 text-xs text-muted">{t("solver.batch.flops_n", { n: rows.length })}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted">
              <tr>
                <th className="py-1">{t("equity.tex.flop")}</th>
                <th>{t("solver.batch.first_action")}</th>
                <th className="text-right">{t("solver.batch.ev")}</th>
                <th className="text-right">{t("solver.viewer.equity")}</th>
                <th className="text-right">{t("solver.library.expl")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {p.rows.map((r) => (
                <tr key={r.flop} className="border-t border-border" data-testid="batch-row">
                  <td className="py-1">
                    <span className="flex gap-0.5">
                      {[0, 2, 4].map((i) => (
                        <PlayingCard key={i} card={r.flop.slice(i, i + 2)} size="sm" />
                      ))}
                    </span>
                  </td>
                  <td>
                    <span className="flex flex-wrap gap-x-d2 text-xs">
                      {r.actions.map((a, i) => (
                        <span key={i} className="flex items-center gap-1">
                          <span className="inline-block h-2 w-2 rounded-sm" style={{ background: actionColour(i) }} aria-hidden />
                          {actionLabel(a, t)} {((r.totals[i] ?? 0) * 100).toFixed(0)} %
                        </span>
                      ))}
                    </span>
                  </td>
                  <td className="text-right tabular-nums">{r.evOop.toFixed(2)}</td>
                  <td className="text-right tabular-nums">{(r.equityOop * 100).toFixed(1)} %</td>
                  <td className="text-right tabular-nums">{r.exploitabilityPct.toFixed(2)} %</td>
                  <td className="text-right">
                    <button
                      className="text-primary underline"
                      onClick={() => void getBackend().then((b) => b.solverCacheLoad(r.hash, 0)).then(() => (window.location.hash = "#/m/solver/viewer"))}
                    >
                      {t("solver.library.open")}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
