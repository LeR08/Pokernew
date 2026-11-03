import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/input";
import { colourClass, formatStat, int } from "@/lib/format";
import { cn } from "@/lib/utils";
import { NotesPanel } from "../replayer/NotesPanel";
import { useFilter, useOpponents } from "./api";

const COLS = ["vpip", "pfr", "three_bet", "fold_to_3bet", "steal", "wtsd", "af"];

/** Opponents with their stats and the gap to the population ("pool"). */
export function Opponents() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const qc = useQueryClient();
  const [minHands, setMinHands] = useState(50);
  const [sel, setSel] = useState<string | null>(null);
  const [ran, setRan] = useState<number | null>(null);
  const data = useOpponents(useFilter(), minHands).data;
  const stats = (data?.stats ?? []).filter((s) => COLS.includes(s.id));
  const poolValue = (id: string) => data?.pool?.stats.find((x) => x.id === id)?.value ?? null;
  return (
    <div className="grid grid-cols-1 gap-d4 2xl:grid-cols-[1fr_22rem]">
      <div className="flex flex-col gap-d3">
        <div className="flex flex-wrap items-center gap-d3 text-sm">
          <label className="flex items-center gap-d2">
            {t("opponents.min_hands")}
            <Select className="w-24" value={minHands} onChange={(e) => setMinHands(Number(e.target.value))}>
              {[10, 30, 50, 100, 250].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </Select>
          </label>
          <Button
            variant="outline"
            size="sm"
            data-testid="run-autonotes"
            onClick={() =>
              void getBackend()
                .then((b) => b.autonotesRun())
                .then((n) => {
                  setRan(n);
                  return qc.invalidateQueries({ queryKey: ["db", "notes"] });
                })
            }
          >
            <RefreshCw className="h-4 w-4" />
            {t("opponents.run_autonotes")}
          </Button>
          {ran !== null && <span className="text-success">{t("opponents.autonotes_done", { count: ran })}</span>}
        </div>
        <p className="text-xs text-muted">{t("opponents.help")}</p>
        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full text-sm tabular-nums" data-testid="opponents-table">
            <thead className="bg-surface-alt text-left text-xs text-muted">
              <tr>
                <th className="whitespace-nowrap px-d3 py-d2 font-medium">{t("opponents.player")}</th>
                <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.hands")}</th>
                {stats.map((s) => (
                  <th key={s.id} className="whitespace-nowrap px-d3 py-d2 text-right font-medium">
                    {s.name}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {data?.pool && (
                <tr className="border-t border-border bg-surface-alt/50 italic">
                  <td className="whitespace-nowrap px-d3 py-d2">{t("opponents.pool")}</td>
                  <td className="whitespace-nowrap px-d3 py-d2 text-right">{int(data.pool.hands, lang)}</td>
                  {stats.map((s) => (
                    <td key={s.id} className="whitespace-nowrap px-d3 py-d2 text-right">
                      {formatStat(poolValue(s.id), s.format, lang)}
                    </td>
                  ))}
                </tr>
              )}
              {data && data.rows.length === 0 && (
                <tr>
                  <td colSpan={2 + stats.length} className="px-d3 py-d4 text-center text-muted" data-testid="opponents-empty">
                    {t("opponents.empty", { n: minHands })}
                  </td>
                </tr>
              )}
              {data?.rows.map((r) => (
                <tr
                  key={r.key}
                  className={cn("cursor-pointer border-t border-border hover:bg-surface-alt", sel === r.key && "bg-surface-alt")}
                  onClick={() => setSel(r.key)}
                  data-testid="opponent-row"
                >
                  <td className="whitespace-nowrap px-d3 py-d2 font-medium">{r.key}</td>
                  <td className="whitespace-nowrap px-d3 py-d2 text-right">{int(r.hands, lang)}</td>
                  {stats.map((s) => {
                    const v = r.stats.find((x) => x.id === s.id);
                    const p = poolValue(s.id);
                    const diff = v?.value != null && p != null && s.format.startsWith("pct") ? (v.value - p) * 100 : null;
                    return (
                      <td key={s.id} className={`whitespace-nowrap px-d3 py-d2 text-right ${colourClass(v?.value, s.colourRanges)}`} title={t("tracker.sample", { n: v?.sample ?? 0 })}>
                        {formatStat(v?.value, s.format, lang)}
                        {diff !== null && Math.abs(diff) >= 5 && (
                          <span className="ml-1 text-xs text-muted">
                            ({diff > 0 ? "+" : ""}
                            {int(diff, lang)})
                          </span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
      <div>{sel ? <NotesPanel room="winamax" name={sel} /> : <p className="text-sm text-muted">{t("opponents.pick")}</p>}</div>
    </div>
  );
}
