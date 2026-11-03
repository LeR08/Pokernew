import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/card";
import { bb, euros, formatStat, int, resultClass } from "@/lib/format";
import { useTracker } from "@/stores/tracker";
import { cn } from "@/lib/utils";
import { useFilter, useReport, useTournaments } from "./api";
import { StatCard } from "./StatCard";

function Hero({ label, value, cls, testId, sub }: { label: string; value: string; cls?: string; testId?: string; sub?: string }) {
  return (
    <Card className="flex flex-col gap-d1 p-d4">
      <span className="text-xs text-muted">{label}</span>
      <span className={cn("text-3xl font-semibold tabular-nums", cls)} data-testid={testId}>
        {value}
      </span>
      {sub && <span className="text-xs text-muted">{sub}</span>}
    </Card>
  );
}

export function Overview() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const filter = useFilter();
  const euro = useTracker((s) => s.euros);
  const report = useReport(filter, "none").data;
  const tourneys = useTournaments(filter).data;
  const all = report?.rows[0];
  if (!report) return null;
  if (!all || all.hands === 0) return <p className="text-muted">{t("tracker.no_hands")}</p>;
  const tot = tourneys?.totals;

  return (
    <div className="flex flex-col gap-d4">
      <div className="grid grid-cols-2 gap-d3 lg:grid-cols-4" data-testid="overview-hero">
        <Hero label={t("tracker.hands")} value={int(all.hands, lang)} />
        {euro ? (
          <>
            <Hero
              label={t("tracker.cash_result")}
              value={euros(all.netCashCents, lang)}
              cls={resultClass(all.netCashCents)}
              testId="hero-result"
              sub={t("tracker.cash_result_sub")}
            />
            <Hero
              label={t("tracker.tourney_result")}
              value={tot ? euros(tot.profitCents, lang) : "—"}
              cls={tot ? resultClass(tot.profitCents) : ""}
              sub={tot ? t("tracker.tourney_result_sub", { n: tot.withResult }) : ""}
            />
            <Hero
              label={t("tracker.roi")}
              value={tot?.roi != null ? formatStat(tot.roi, "pct1", lang) : "—"}
              cls={tot?.roi != null ? resultClass(tot.roi) : ""}
              sub={tot?.itm != null ? t("tracker.itm", { v: formatStat(tot.itm, "pct0", lang) }) : ""}
            />
          </>
        ) : (
          <>
            <Hero label={t("tracker.net_bb")} value={bb(all.netBb, lang)} cls={resultClass(all.netBb)} testId="hero-result" />
            <Hero
              label="bb/100"
              value={formatStat(all.bbPer100, "bb100", lang)}
              cls={resultClass(all.bbPer100 ?? 0)}
              sub={t("tracker.bb100_sub")}
            />
            <Hero
              label={t("tracker.verdict")}
              value={all.netBb >= 0 ? t("tracker.winning") : t("tracker.losing")}
              cls={resultClass(all.netBb)}
            />
          </>
        )}
      </div>
      <div className="grid grid-cols-1 gap-d3 sm:grid-cols-2 xl:grid-cols-3" data-testid="stat-cards">
        {report.stats
          .filter((m) => m.id !== "bb100")
          .map((m) => (
            <StatCard key={m.id} meta={m} value={all.stats.find((s) => s.id === m.id)} />
          ))}
      </div>
      {report.problems.length > 0 && (
        <ul className="text-sm text-danger">
          {report.problems.map((p) => (
            <li key={p.file + p.id}>
              stats/{p.file}.toml {p.id}: {p.message}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
