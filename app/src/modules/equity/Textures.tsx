/** A range's breakdown aggregated over the 1 755 flop classes (or a subset). */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayingCard } from "@/components/PlayingCard";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { DRAW_IDS, MADE_IDS, type FlopFilter, type TextureReport } from "@/lib/equity";
import { RANKS, tryParseRange } from "@/lib/range";
import { BreakdownTable } from "./Analysis";
import { pct } from "./format";
import { Modal } from "./Modal";
import { useEquity } from "./store";

/** Pair or better = every made class above "ace high". */
const PAIR_PLUS = MADE_IDS.slice(0, MADE_IDS.indexOf("ace_high"));

export function Textures() {
  const { t } = useTranslation();
  const s = useEquity();
  const [filter, setFilter] = useState<FlopFilter>({});
  const [rep, setRep] = useState<TextureReport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState(false);
  const [metric, setMetric] = useState("pair_plus");
  const [asc, setAsc] = useState(false);

  const run = () => {
    setBusy(true);
    setError(null);
    void getBackend()
      .then((b) => b.flopTexture(s.tRange, filter))
      .then(setRep)
      .catch((e: Error) => setError(e.message))
      .finally(() => setBusy(false));
  };

  const value = (r: TextureReport["rows"][number]) => {
    if (metric === "pair_plus") return PAIR_PLUS.reduce((a, id) => a + r.made[MADE_IDS.indexOf(id)], 0);
    const m = MADE_IDS.indexOf(metric as (typeof MADE_IDS)[number]);
    return m >= 0 ? r.made[m] : r.draws[DRAW_IDS.indexOf(metric as (typeof DRAW_IDS)[number])];
  };
  const rows = useMemo(() => {
    if (!rep) return [];
    const sorted = [...rep.rows].sort((a, b) => (asc ? value(a) - value(b) : value(b) - value(a)));
    return sorted.slice(0, 200);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rep, metric, asc]);

  const sel = (k: keyof FlopFilter, v: string) =>
    setFilter({ ...filter, [k]: v === "" ? null : k === "highMin" || k === "highMax" ? Number(v) : k === "connected" ? v === "yes" : v });

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <div className="flex flex-wrap items-center gap-d2">
          <span className="text-sm">{t("equity.range")}</span>
          <code className="max-w-xl flex-1 truncate rounded bg-surface-alt px-d2 py-1 text-xs" data-testid="t-range">
            {s.tRange || "—"}
          </code>
          <Button size="sm" variant="outline" onClick={() => setEdit(true)} data-testid="t-edit">
            {t("equity.edit_range")}
          </Button>
        </div>
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex flex-col gap-1">
            {t("equity.tex.suits")}
            <Select className="w-40" value={filter.suits ?? ""} onChange={(e) => sel("suits", e.target.value)} data-testid="t-suits">
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
            {t("equity.tex.high")}
            <span className="flex items-center gap-1">
              <Select className="w-20" value={filter.highMin ?? ""} onChange={(e) => sel("highMin", e.target.value)} aria-label={t("equity.tex.high_min")}>
                <option value="">—</option>
                {[...RANKS].map((r, i) => (
                  <option key={r} value={i}>
                    {r}
                  </option>
                ))}
              </Select>
              –
              <Select className="w-20" value={filter.highMax ?? ""} onChange={(e) => sel("highMax", e.target.value)} aria-label={t("equity.tex.high_max")}>
                <option value="">—</option>
                {[...RANKS].map((r, i) => (
                  <option key={r} value={i}>
                    {r}
                  </option>
                ))}
              </Select>
            </span>
          </label>
          <label className="flex flex-col gap-1">
            {t("equity.tex.connected")}
            <Select
              className="w-32"
              value={filter.connected === undefined || filter.connected === null ? "" : filter.connected ? "yes" : "no"}
              onChange={(e) => sel("connected", e.target.value)}
            >
              <option value="">{t("equity.tex.any")}</option>
              <option value="yes">{t("equity.tex.yes")}</option>
              <option value="no">{t("equity.tex.no")}</option>
            </Select>
          </label>
          <Button disabled={busy || tryParseRange(s.tRange) === null} onClick={run} data-testid="t-run">
            {busy ? t("equity.running") : t("equity.run")}
          </Button>
        </div>
        {error && <p className="text-sm text-danger">{error}</p>}
      </Card>

      {rep && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="t-report">
          <p className="text-sm" data-testid="t-summary">
            {t("equity.tex.summary", { classes: rep.classes, flops: rep.flops })}
          </p>
          <div className="grid grid-cols-1 gap-d4 lg:grid-cols-2">
            <BreakdownTable rows={rep.made} kind="made" />
            <BreakdownTable rows={rep.draws} kind="draws" />
          </div>
          <div className="flex flex-wrap items-center gap-d2 text-sm">
            {t("equity.tex.sort")}
            <Select className="w-56" value={metric} onChange={(e) => setMetric(e.target.value)} data-testid="t-metric">
              <option value="pair_plus">{t("equity.tex.pair_plus")}</option>
              {[...MADE_IDS, ...DRAW_IDS].map((id) => (
                <option key={id} value={id}>
                  {t(`equity.class.${id}`)}
                </option>
              ))}
            </Select>
            <Button size="sm" variant="outline" onClick={() => setAsc(!asc)}>
              {asc ? t("equity.tex.asc") : t("equity.tex.desc")}
            </Button>
            {rep.rows.length > rows.length && <span className="text-xs text-muted">{t("equity.tex.first", { n: rows.length })}</span>}
          </div>
          <table className="w-full text-sm" data-testid="t-rows">
            <thead className="text-left text-xs text-muted">
              <tr>
                <th className="py-1">{t("equity.tex.flop")}</th>
                <th className="text-right">{t("equity.tex.weight")}</th>
                <th className="text-right">{metric === "pair_plus" ? t("equity.tex.pair_plus") : t(`equity.class.${metric}`)}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.cards} className="border-t border-border" data-testid="t-row">
                  <td className="py-1">
                    <span className="flex gap-0.5">
                      {[0, 2, 4].map((i) => (
                        <PlayingCard key={i} card={r.cards.slice(i, i + 2)} size="sm" />
                      ))}
                    </span>
                  </td>
                  <td className="text-right text-muted">{r.weight}</td>
                  <td className="text-right tabular-nums">{pct(value(r), 1)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}

      {edit && (
        <Modal title={t("equity.range")} onClose={() => setEdit(false)}>
          <RangeEditor value={s.tRange} onChange={(v) => s.set({ tRange: v })} testId="t-editor" />
        </Modal>
      )}
    </div>
  );
}
