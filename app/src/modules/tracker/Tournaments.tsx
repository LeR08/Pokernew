import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Card } from "@/components/ui/card";
import { formatDate } from "@/lib/importApi";
import { bb, euros, formatStat, int, resultClass } from "@/lib/format";
import { useFilter, useTournaments } from "./api";
import { usePlayMode } from "@/stores/playMode";

export function Tournaments() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const playMode = usePlayMode((x) => x.on);
  const data = useTournaments(useFilter()).data;
  if (!data) return null;
  const tot = data.totals;
  return (
    <div className="flex flex-col gap-d4">
      <div className="grid grid-cols-2 gap-d3 md:grid-cols-5" data-testid="tourney-totals">
        {[
          [t("tracker.tournaments"), int(tot.tournaments, lang), ""],
          [t("tracker.cost"), euros(tot.costCents, lang, false), ""],
          [t("tracker.won"), euros(tot.wonCents, lang, false), ""],
          [t("tracker.profit"), euros(tot.profitCents, lang), resultClass(tot.profitCents)],
          [t("tracker.roi"), tot.roi != null ? formatStat(tot.roi, "pct1", lang) : "—", resultClass(tot.roi ?? 0)],
        ].map(([label, value, cls]) => (
          <Card key={label} className="flex flex-col gap-d1 p-d3">
            <span className="text-xs text-muted">{label}</span>
            <span className={`text-xl font-semibold tabular-nums ${cls}`}>{value}</span>
          </Card>
        ))}
      </div>
      <p className="-mt-d2 text-xs text-muted">{t("tracker.tourney_help", { n: tot.withResult, total: tot.tournaments })}</p>
      <Card className="flex flex-col gap-d3 p-d4" data-testid="tourney-recap">
        <h2 className="font-semibold">{t("tracker.perf.title")}</h2>
        <div className="grid grid-cols-2 gap-d3 text-sm sm:grid-cols-3 lg:grid-cols-6">
          {[
            [t("tracker.perf.itm"), tot.itm != null ? formatStat(tot.itm, "pct1", lang) : "—"],
            [t("tracker.perf.entries"), `${int(tot.entries, lang)} (${t("tracker.perf.reentries", { n: tot.reentries })})`],
            [t("tracker.perf.knockouts"), int(tot.knockouts, lang)],
            [t("tracker.perf.bounties"), euros(tot.bountiesCents, lang, false)],
            [t("tracker.perf.avg_finish"), tot.avgFinishPct != null ? t("tracker.perf.top", { p: Math.round(tot.avgFinishPct * 100) }) : "—"],
            [t("tracker.perf.best"), tot.bestFinish ? `${tot.bestFinish[0]} / ${tot.bestFinish[1]}` : "—"],
            [t("tracker.perf.wins"), int(tot.wins, lang)],
            [t("tracker.perf.top10"), int(tot.top10, lang)],
            [t("tracker.perf.avg_buy_in"), tot.avgBuyInCents != null ? euros(tot.avgBuyInCents, lang, false) : "—"],
            [t("tracker.perf.biggest"), euros(tot.biggestCashCents, lang, false)],
            [t("tracker.perf.ko_per"), tot.tournaments ? (tot.knockouts / tot.tournaments).toFixed(2) : "—"],
            [t("tracker.perf.profit_per"), tot.withResult ? euros(Math.round(tot.profitCents / tot.withResult), lang) : "—"],
          ].map(([label, value]) => (
            <div key={label} className="flex flex-col">
              <span className="text-xs text-muted">{label}</span>
              <span className="font-semibold tabular-nums">{value}</span>
            </div>
          ))}
        </div>
        <div className="grid grid-cols-1 gap-d4 lg:grid-cols-2">
          {(
            [
              ["by_kind", tot.byKind],
              ["by_buy_in", tot.byBuyIn],
            ] as const
          ).map(([k, groups]) => (
            <table key={k} className="text-sm tabular-nums" data-testid={`tourney-${k}`}>
              <thead className="text-left text-xs text-muted">
                <tr>
                  <th className="py-1">{t(`tracker.perf.${k}`)}</th>
                  <th className="text-right">{t("tracker.tournaments")}</th>
                  <th className="text-right">{t("tracker.perf.itm")}</th>
                  <th className="text-right">{t("tracker.roi")}</th>
                  <th className="text-right">{t("tracker.profit")}</th>
                </tr>
              </thead>
              <tbody>
                {groups.map((g) => (
                  <tr key={g.key} className="border-t border-border">
                    <td className="py-1">{groupLabel(g.key, t)}</td>
                    <td className="text-right">{g.tournaments}</td>
                    <td className="text-right">{g.itm != null ? formatStat(g.itm, "pct0", lang) : "—"}</td>
                    <td className={`text-right ${resultClass(g.roi ?? 0)}`}>{g.roi != null ? formatStat(g.roi, "pct0", lang) : "—"}</td>
                    <td className={`text-right ${resultClass(g.profitCents)}`}>{euros(g.profitCents, lang)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ))}
        </div>
      </Card>
      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-sm tabular-nums" data-testid="tourney-table">
          <thead className="bg-surface-alt text-left text-xs text-muted">
            <tr>
              <th className="whitespace-nowrap px-d3 py-d2 font-medium">{t("tracker.col.date")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 font-medium">{t("tracker.col.tournament")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.col.entries")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.cost")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.won")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.profit")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.col.finish")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.perf.ko")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.hands")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.col_net_bb")}</th>
            </tr>
          </thead>
          <tbody>
            {data.rows.map((r) => (
              <tr key={r.id} className="border-t border-border">
                <td className="whitespace-nowrap px-d3 py-d2">{formatDate(r.startedAt, lang)}</td>
                <td className="whitespace-nowrap px-d3 py-d2">
                  {r.name}
                  {r.gameType === "spin" && <span className="ml-d2 rounded bg-surface-alt px-1 text-xs">{t("tracker.format.spin")}</span>}
                  {!playMode && (
                    <Link className="ml-d2 text-xs text-primary underline" to={`/m/icm/review?tournament=${r.id}`} data-testid="tournament-icm">
                      {t("tracker.icm_review")}
                    </Link>
                  )}
                </td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">{r.entries}</td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">{euros(r.costCents, lang, false)}</td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">{r.wonCents != null ? euros(r.wonCents, lang, false) : "—"}</td>
                <td className={`whitespace-nowrap px-d3 py-d2 text-right ${resultClass(r.profitCents ?? 0)}`}>
                  {r.profitCents != null ? euros(r.profitCents, lang) : "—"}
                </td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">
                  {r.finishPosition ? `${r.finishPosition}${r.registeredPlayers ? ` / ${r.registeredPlayers}` : ""}` : "—"}
                </td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">{r.knockouts || "—"}</td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">
                  <Link className="text-primary underline" to={`/m/tracker/hands?tournament=${r.id}`}>
                    {r.hands}
                  </Link>
                </td>
                <td className={`whitespace-nowrap px-d3 py-d2 text-right ${resultClass(r.netBb)}`}>{bb(r.netBb, lang)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/** "mtt·knockout" → "MTT · KO", "b:1-5" → "1 à 5 €". */
function groupLabel(key: string, t: (k: string, o?: Record<string, unknown>) => string): string {
  if (key.includes(":")) return t(`tracker.perf.bucket.${key.split(":")[0]}`);
  const [g, k] = key.split("·");
  return `${t(`tracker.format.${g}`, { defaultValue: g.toUpperCase() })} · ${t(`tracker.perf.kind.${k}`, { defaultValue: k })}`;
}
