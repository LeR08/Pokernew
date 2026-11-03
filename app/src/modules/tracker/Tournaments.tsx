import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";
import { Card } from "@/components/ui/card";
import { formatDate } from "@/lib/importApi";
import { bb, euros, formatStat, int, resultClass } from "@/lib/format";
import { useFilter, useTournaments } from "./api";

export function Tournaments() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
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
