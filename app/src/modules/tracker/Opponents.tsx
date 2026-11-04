import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router-dom";
import { useQueryClient } from "@tanstack/react-query";
import { RefreshCw } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { Button } from "@/components/ui/button";
import { Input, Select } from "@/components/ui/input";
import { colourClass, formatStat, int } from "@/lib/format";
import { cn } from "@/lib/utils";
import { PlayerSheet } from "./PlayerSheet";
import { useFilter, useOpponents } from "./api";

const COLS = ["vpip", "pfr", "three_bet", "fold_to_3bet", "steal", "wtsd", "af"];

/**
 * Opponents with their stats and the gap to the population. The table scrolls
 * under a fixed header; ↑ / ↓ go from one player to the next; the player sheet
 * (stats, estimated ranges, notes) stays in view on the right.
 */
export function Opponents() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const qc = useQueryClient();
  const [params] = useSearchParams();
  const [minHands, setMinHands] = useState(50);
  const [search, setSearch] = useState("");
  const [sel, setSel] = useState<string | null>(params.get("player"));
  const [ran, setRan] = useState<number | null>(null);
  const data = useOpponents(useFilter(), minHands).data;
  const stats = (data?.stats ?? []).filter((s) => COLS.includes(s.id));
  const poolValue = (id: string) => data?.pool?.stats.find((x) => x.id === id)?.value ?? null;
  const rows = (data?.rows ?? []).filter((r) => r.key.toLowerCase().includes(search.toLowerCase()));
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const p = params.get("player");
    if (p) {
      setSel(p);
      setSearch("");
    }
  }, [params]);
  useEffect(() => {
    if (!sel) return;
    box.current?.querySelector(`[data-key="${CSS.escape(sel)}"]`)?.scrollIntoView({ block: "nearest" });
  }, [sel, data]);

  const move = (d: number) => {
    if (!rows.length) return;
    const i = rows.findIndex((r) => r.key === sel);
    const n = Math.min(rows.length - 1, Math.max(0, (i < 0 ? -1 : i) + d));
    setSel(rows[n].key);
  };

  return (
    <div className="grid grid-cols-1 gap-d4 xl:grid-cols-[1fr_24rem]">
      <div className="flex min-w-0 flex-col gap-d3">
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
          <Input className="w-56" value={search} onChange={(e) => setSearch(e.target.value)} placeholder={t("opponents.search")} data-testid="opponents-search" />
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
        <div
          ref={box}
          className="max-h-[calc(100vh-17rem)] min-h-64 overflow-auto rounded-lg border border-border focus:outline-none focus-visible:ring-2 focus-visible:ring-primary"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              e.preventDefault();
              move(e.key === "ArrowDown" ? 1 : -1);
            }
          }}
          data-testid="opponents-scroll"
        >
          <table className="w-full text-sm tabular-nums" data-testid="opponents-table">
            <thead className="sticky top-0 z-10 bg-surface-alt text-left text-xs text-muted shadow-sm">
              <tr>
                <th className="whitespace-nowrap px-d3 py-d2 font-medium">{t("opponents.player")}</th>
                <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.hands")}</th>
                {stats.map((s) => (
                  <th key={s.id} className="whitespace-nowrap px-d3 py-d2 text-right font-medium">
                    {s.name}
                  </th>
                ))}
              </tr>
              {data?.pool && (
                <tr className="bg-surface-alt italic text-fg">
                  <td className="whitespace-nowrap px-d3 py-d2">{t("opponents.pool")}</td>
                  <td className="whitespace-nowrap px-d3 py-d2 text-right">{int(data.pool.hands, lang)}</td>
                  {stats.map((s) => (
                    <td key={s.id} className="whitespace-nowrap px-d3 py-d2 text-right">
                      {formatStat(poolValue(s.id), s.format, lang)}
                    </td>
                  ))}
                </tr>
              )}
            </thead>
            <tbody>
              {data && rows.length === 0 && (
                <tr>
                  <td colSpan={2 + stats.length} className="px-d3 py-d4 text-center text-muted" data-testid="opponents-empty">
                    {t("opponents.empty", { n: minHands })}
                  </td>
                </tr>
              )}
              {rows.map((r) => (
                <tr
                  key={r.key}
                  data-key={r.key}
                  className={cn("cursor-pointer border-t border-border hover:bg-surface-alt", sel === r.key && "outline outline-2 -outline-offset-2 outline-primary")}
                  onClick={() => setSel(r.key)}
                  data-testid="opponent-row"
                  aria-selected={sel === r.key}
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
        <p className="text-xs text-muted">{t("opponents.keys")}</p>
      </div>
      <div className="xl:sticky xl:top-d4 xl:max-h-[calc(100vh-2rem)] xl:self-start xl:overflow-y-auto">
        {sel ? (
          <PlayerSheet name={sel} row={data?.rows.find((r) => r.key === sel) ?? null} pool={data?.pool ?? null} stats={stats} />
        ) : (
          <p className="text-sm text-muted">{t("opponents.pick")}</p>
        )}
      </div>
    </div>
  );
}
