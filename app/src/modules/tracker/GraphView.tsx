import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/card";
import { bb, euros } from "@/lib/format";
import { useTracker } from "@/stores/tracker";
import { useFilter, useGraph, useTournaments } from "./api";
import { LineChart, type Line } from "./LineChart";

export function GraphView() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const filter = useFilter();
  const euro = useTracker((s) => s.euros);
  const g = useGraph(filter).data;
  const tourneys = useTournaments(filter).data;

  const fmtBb = useMemo(() => (v: number) => bb(v, lang), [lang]);
  const fmtEur = useMemo(() => (v: number) => euros(Math.round(v * 100), lang), [lang]);
  const bbLines: Line[] = useMemo(
    () =>
      g
        ? [
            { name: t("tracker.graph.total"), data: g.totalBb.map((v) => +v.toFixed(2)) },
            { name: t("tracker.graph.showdown"), data: g.showdownBb.map((v) => +v.toFixed(2)) },
            { name: t("tracker.graph.non_showdown"), data: g.nonShowdownBb.map((v) => +v.toFixed(2)) },
          ]
        : [],
    [g, t],
  );
  const cashLines: Line[] = useMemo(() => (g ? [{ name: t("tracker.graph.cash_eur"), data: g.cashEur }] : []), [g, t]);
  const tLines: Line[] = useMemo(
    () => (tourneys ? [{ name: t("tracker.graph.tourney_eur"), data: tourneys.totals.curveEur }] : []),
    [tourneys, t],
  );
  const tX = useMemo(() => (tourneys ? tourneys.totals.curveEur.map((_, i) => i + 1) : []), [tourneys]);

  if (!g) return null;
  if (g.hands === 0) return <p className="text-muted">{t("tracker.no_hands")}</p>;

  if (!euro) {
    return (
      <Card className="p-d4">
        <h3 className="mb-d2 font-semibold">{t("tracker.graph.title_bb")}</h3>
        <LineChart x={g.x} lines={bbLines} xLabel={t("tracker.hands")} format={fmtBb} testId="graph-bb" />
        <p className="mt-d2 text-xs text-muted">{t("tracker.graph.help_bb")}</p>
      </Card>
    );
  }
  const hasCash = g.cashEur.some((v) => v !== 0);
  return (
    <div className="flex flex-col gap-d4">
      {hasCash && (
        <Card className="p-d4">
          <h3 className="mb-d2 font-semibold">{t("tracker.graph.title_cash")}</h3>
          <LineChart x={g.x} lines={cashLines} xLabel={t("tracker.hands")} format={fmtEur} testId="graph-cash" />
        </Card>
      )}
      <Card className="p-d4">
        <h3 className="mb-d2 font-semibold">{t("tracker.graph.title_tourney")}</h3>
        {tX.length > 0 ? (
          <LineChart x={tX} lines={tLines} xLabel={t("tracker.tournaments")} format={fmtEur} testId="graph-tourney" />
        ) : (
          <p className="text-sm text-muted">{t("tracker.no_tourney_results")}</p>
        )}
        <p className="mt-d2 text-xs text-muted">{t("tracker.graph.help_eur")}</p>
      </Card>
    </div>
  );
}
