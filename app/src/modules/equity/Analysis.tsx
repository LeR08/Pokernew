/** Range breakdown on a board (made hands, draws), next-street filter, equity buckets. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CardPicker } from "@/components/range/CardPicker";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import type { Breakdown, BreakdownRow } from "@/lib/equity";
import { formatRange, parseCards, tryParseRange } from "@/lib/range";
import { EquityHistogram } from "./charts";
import { combos, pct } from "./format";
import { Modal } from "./Modal";
import { useEquity } from "./store";

export function BreakdownTable({
  rows,
  kind,
  selected,
  onToggle,
}: {
  rows: BreakdownRow[];
  kind: "made" | "draws";
  selected?: Set<string>;
  onToggle?: (id: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <table className="w-full text-sm" data-testid={`breakdown-${kind}`}>
      <thead className="text-left text-xs text-muted">
        <tr>
          {onToggle && <th className="w-6" />}
          <th className="py-1">{t(kind === "made" ? "equity.made" : "equity.draws")}</th>
          <th className="text-right">{t("equity.combos")}</th>
          <th className="w-40 text-right">%</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r) => (
          <tr key={r.id} className={r.combos === 0 ? "text-muted" : ""} data-testid="breakdown-row" data-id={r.id}>
            {onToggle && (
              <td>
                <input
                  type="checkbox"
                  checked={selected?.has(r.id) ?? false}
                  onChange={() => onToggle(r.id)}
                  disabled={r.combos === 0}
                  aria-label={t(`equity.class.${r.id}`)}
                  data-testid="breakdown-check"
                />
              </td>
            )}
            <td className="py-0.5">{t(`equity.class.${r.id}`)}</td>
            <td className="text-right tabular-nums">{combos(Math.round(r.combos * 10) / 10)}</td>
            <td className="text-right">
              <span className="flex items-center justify-end gap-d2">
                <span className="h-1.5 rounded-full bg-primary" style={{ width: `${Math.max(1, r.share * 100)}px` }} aria-hidden />
                <span className="w-14 tabular-nums">{pct(r.share, 1)}</span>
              </span>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function Analysis() {
  const { t } = useTranslation();
  const s = useEquity();
  const [data, setData] = useState<Breakdown | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [buckets, setBuckets] = useState<number[] | null>(null);
  const [vsEquity, setVsEquity] = useState<number | null>(null);
  const [editVillain, setEditVillain] = useState(false);
  const [busy, setBusy] = useState(false);

  const board = parseCards(s.aBoard);
  const dead = parseCards(s.aDead) ?? [];
  const okBoard = board !== null && board.length >= 3 && board.length <= 5;
  const okRange = tryParseRange(s.aRange) !== null;

  useEffect(() => {
    setBuckets(null);
    setVsEquity(null);
    if (!okBoard || !okRange) {
      setData(null);
      return;
    }
    const h = setTimeout(() => {
      void getBackend()
        .then((b) => b.rangeBreakdown(s.aRange, s.aBoard, s.aDead))
        .then((d) => {
          setData(d);
          setError(null);
        })
        .catch((e: Error) => setError(e.message));
    }, 150);
    return () => clearTimeout(h);
  }, [s.aRange, s.aBoard, s.aDead, okBoard, okRange]);

  const toggle = (id: string) => {
    const n = new Set(selected);
    if (n.has(id)) n.delete(id);
    else n.add(id);
    setSelected(n);
  };

  const keep = () => {
    const made = [...selected].filter((id) => data?.made.some((r) => r.id === id));
    const draws = [...selected].filter((id) => data?.draws.some((r) => r.id === id));
    void getBackend()
      .then((b) => b.rangeFilter(s.aRange, s.aBoard, s.aDead, made, draws))
      .then((w) => {
        s.set({ aRange: formatRange(Float32Array.from(w)) });
        setSelected(new Set());
      })
      .catch((e: Error) => setError(e.message));
  };

  const runBuckets = () => {
    setBusy(true);
    void getBackend()
      .then((b) => b.equityCalc({ players: [s.aRange, s.aVillain], board: s.aBoard, dead: s.aDead, mode: "auto", precision: 0.002 }))
      .then((r) => {
        const out = new Array(10).fill(0);
        for (const c of r.players[0].perCombo) out[Math.min(9, Math.floor(c.equity * 10))] += c.weight;
        setBuckets(out);
        setVsEquity(r.players[0].equity);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setBusy(false));
  };

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <div className="flex flex-wrap items-end gap-d4">
          <CardPicker label={t("equity.board_3_5")} value={s.aBoard} onChange={(v) => s.set({ aBoard: v })} max={5} blocked={dead} testId="a-board" />
          <CardPicker label={t("equity.dead")} value={s.aDead} onChange={(v) => s.set({ aDead: v })} max={10} blocked={board ?? []} testId="a-dead" />
        </div>
        <RangeEditor value={s.aRange} onChange={(v) => s.set({ aRange: v })} dead={[...(board ?? []), ...dead]} testId="a-range" />
      </Card>

      {error && <p className="text-sm text-danger">{error}</p>}
      {!okBoard && <p className="text-sm text-muted">{t("equity.need_board")}</p>}

      {data && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="breakdown">
          <p className="text-sm">
            {t("equity.breakdown_total", { n: combos(Math.round(data.total * 10) / 10) })}
          </p>
          <div className="grid grid-cols-1 gap-d4 lg:grid-cols-2">
            <BreakdownTable rows={data.made} kind="made" selected={selected} onToggle={toggle} />
            <BreakdownTable rows={data.draws} kind="draws" selected={selected} onToggle={toggle} />
          </div>
          <div className="flex flex-wrap items-center gap-d3">
            <Button disabled={selected.size === 0} onClick={keep} data-testid="keep-selection">
              {t("equity.keep")}
            </Button>
            <span className="text-xs text-muted">{t("equity.keep_help")}</span>
          </div>
        </Card>
      )}

      {data && (
        <Card className="flex flex-col gap-d3 p-d4">
          <h2 className="font-semibold">{t("equity.vs_title")}</h2>
          <div className="flex flex-wrap items-center gap-d2">
            <Input
              className="min-w-48 flex-1 font-mono"
              value={s.aVillain}
              onChange={(e) => s.set({ aVillain: e.target.value })}
              placeholder={t("equity.villain_placeholder")}
              aria-invalid={s.aVillain !== "" && tryParseRange(s.aVillain) === null}
              data-testid="villain-input"
              aria-label={t("equity.villain")}
            />
            <Button size="sm" variant="outline" onClick={() => setEditVillain(true)}>
              {t("equity.edit_range")}
            </Button>
            <Button size="sm" disabled={busy || !s.aVillain.trim() || tryParseRange(s.aVillain) === null} onClick={runBuckets} data-testid="run-buckets">
              {t("equity.run")}
            </Button>
          </div>
          {vsEquity !== null && (
            <p className="text-sm" data-testid="vs-equity">
              {t("equity.vs_equity", { e: pct(vsEquity) })}
            </p>
          )}
          {buckets && <EquityHistogram buckets={buckets} />}
        </Card>
      )}

      {editVillain && (
        <Modal title={t("equity.villain")} onClose={() => setEditVillain(false)}>
          <RangeEditor value={s.aVillain} onChange={(v) => s.set({ aVillain: v })} dead={[...(board ?? []), ...dead]} testId="villain-editor" />
        </Modal>
      )}
    </div>
  );
}
