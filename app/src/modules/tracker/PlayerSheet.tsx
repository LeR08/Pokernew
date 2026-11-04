/**
 * Player sheet: key stats against the population, ranges estimated from the
 * stats (the top X % of hands by preflop strength), notes. The estimate is a
 * starting point: a player's real ranges are not exactly the strongest hands.
 */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { getBackend } from "@/lib/backend";
import { CELL_COMBOS, cellName, formatRange, NUM_COMBOS } from "@/lib/range";
import { estimatedRange, type RangeKind } from "@/lib/playerRange";
import type { ReportRow, StatMeta } from "@/lib/tracker";
import { formatStat, int } from "@/lib/format";
import { cn } from "@/lib/utils";
import { usePlayMode } from "@/stores/playMode";
import { NotesPanel } from "../replayer/NotesPanel";

type Kind = RangeKind;
export { estimatedRange };

function MiniGrid({ w }: { w: Float32Array }) {
  return (
    <div className="grid w-full max-w-[15rem] gap-px rounded bg-border p-px" style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }} data-testid="sheet-grid">
      {CELL_COMBOS.map((cs, c) => {
        const f = cs.reduce((a, i) => a + w[i], 0) / cs.length;
        return <span key={c} title={cellName(c)} className={cn("aspect-square", f > 0.5 ? "bg-primary" : "bg-surface")} />;
      })}
    </div>
  );
}

export function PlayerSheet({ row, pool, stats, name }: { row: ReportRow | null; pool: ReportRow | null; stats: StatMeta[]; name: string }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const navigate = useNavigate();
  const playMode = usePlayMode((x) => x.on);
  const [kind, setKind] = useState<Kind>("pfr");
  const w = useMemo(() => (row ? estimatedRange(row, kind) : null), [row, kind]);
  const text = w ? formatRange(w) : "";
  const share = w ? (w.reduce((a, x) => a + x, 0) / NUM_COMBOS) * 100 : 0;
  const [msg, setMsg] = useState<string | null>(null);
  return (
    <div className="flex flex-col gap-d3" data-testid="player-sheet">
      <Card className="flex flex-col gap-d2 p-d3 text-sm">
        <div className="flex items-baseline justify-between gap-d2">
          <span className="text-base font-semibold">{name}</span>
          {row && <span className="text-xs text-muted">{t("sheet.hands", { n: int(row.hands, lang) })}</span>}
        </div>
        {row ? (
          <table className="text-xs tabular-nums">
            <tbody>
              {stats.map((s) => {
                const v = row.stats.find((x) => x.id === s.id)?.value;
                const p = pool?.stats.find((x) => x.id === s.id)?.value;
                return (
                  <tr key={s.id}>
                    <td className="py-0.5 text-muted">{s.name}</td>
                    <td className="text-right font-medium">{formatStat(v, s.format, lang)}</td>
                    <td className="pl-d2 text-right text-muted">{t("sheet.pool", { v: formatStat(p, s.format, lang) })}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <p className="text-xs text-muted">{t("sheet.few_hands")}</p>
        )}
      </Card>
      {row && !playMode && (
        <Card className="flex flex-col gap-d2 p-d3 text-sm" data-testid="sheet-range">
          <div className="flex flex-wrap items-center gap-1">
            <span className="mr-1 font-semibold">{t("sheet.range")}</span>
            {(["pfr", "call", "three_bet", "vpip"] as Kind[]).map((k) => (
              <Button key={k} size="sm" variant={kind === k ? "default" : "outline"} onClick={() => setKind(k)} data-testid={`sheet-kind-${k}`}>
                {t(`sheet.kind.${k}`)}
              </Button>
            ))}
          </div>
          {w ? (
            <>
              <MiniGrid w={w} />
              <p className="text-xs text-muted">{t("sheet.estimate", { p: share.toFixed(1) })}</p>
              <code className="break-all text-xs" data-testid="sheet-range-text">
                {text || "—"}
              </code>
              <div className="flex flex-wrap gap-d2">
                <Button size="sm" variant="outline" disabled={!text} onClick={() => navigate(`/m/equity/calc?players=${encodeURIComponent(`random|${text}`)}`)} data-testid="sheet-to-equity">
                  {t("sheet.to_equity")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={!text}
                  onClick={() =>
                    void getBackend()
                      .then((b) => b.rangeSave(`joueurs/${name.replace(/[^\p{L}\p{N} _\-+.()%]/gu, "_")}/${kind}`, text))
                      .then(() => setMsg(t("sheet.saved")))
                      .catch((e: Error) => setMsg(e.message))
                  }
                  data-testid="sheet-save"
                >
                  {t("sheet.save")}
                </Button>
              </div>
              {msg && <p className="text-xs text-muted">{msg}</p>}
            </>
          ) : (
            <p className="text-xs text-muted">{t("sheet.no_stat")}</p>
          )}
        </Card>
      )}
      <NotesPanel room="winamax" name={name} />
    </div>
  );
}
