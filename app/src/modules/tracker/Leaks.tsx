import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/card";
import { formatStat, text } from "@/lib/format";
import { cn } from "@/lib/utils";
import { useFilter, useLeaks } from "./api";

const BADGE: Record<string, string> = {
  leak: "border-danger text-danger",
  possible: "border-warning text-warning",
  ok: "border-success text-success",
  too_few: "border-border text-muted",
};

/** Hero stats against target ranges (leaks/targets.toml). */
export function Leaks() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const data = useLeaks(useFilter()).data;
  if (!data) return null;
  const meta = (id: string) => data.stats.find((s) => s.id === id);
  const shown = data.rows.filter((r) => r.status !== "too_few");
  if (shown.length === 0) return <p className="text-muted">{t("leaks.not_enough")}</p>;
  return (
    <div className="flex flex-col gap-d3">
      <p className="text-sm text-muted">{t("leaks.intro")}</p>
      <ul className="flex flex-col gap-d2" data-testid="leaks">
        {shown.map((r) => {
          const m = meta(r.stat);
          const fmt = m?.format ?? "pct1";
          const advice = r.direction === "high" ? text(r.tooHigh, lang) : r.direction === "low" ? text(r.tooLow, lang) : "";
          const range = [r.target.min != null ? formatStat(r.target.min, fmt, lang) : "", r.target.max != null ? formatStat(r.target.max, fmt, lang) : ""];
          return (
            <Card key={`${r.stat}-${r.gameType}`} className="flex flex-col gap-d2 p-d3 md:flex-row md:items-center" data-testid={`leak-${r.stat}-${r.gameType}`}>
              <span className={cn("w-28 shrink-0 rounded border px-d2 py-0.5 text-center text-xs font-medium", BADGE[r.status])}>
                {t(`leaks.status.${r.status}`)}
              </span>
              <div className="w-44 shrink-0">
                <div className="font-medium">{m?.name ?? r.stat}</div>
                <div className="text-xs text-muted">{t(`tracker.format.${r.gameType}`, { defaultValue: r.gameType })}</div>
              </div>
              <div className="w-56 shrink-0 text-sm tabular-nums">
                <span className="text-lg font-semibold">{formatStat(r.value, fmt, lang)}</span>{" "}
                <span className="text-xs text-muted">
                  {range[0] && range[1]
                    ? t("leaks.target", { min: range[0], max: range[1] })
                    : range[0]
                      ? t("leaks.target_min", { min: range[0] })
                      : t("leaks.target_max", { max: range[1] })}
                </span>
                {r.ciLow != null && r.ciHigh != null && fmt.startsWith("pct") && (
                  <div className="text-xs text-muted">
                    {t("leaks.ci", { low: formatStat(r.ciLow, "pct0", lang), high: formatStat(r.ciHigh, "pct0", lang), n: r.sample })}
                  </div>
                )}
              </div>
              <div className="flex-1 text-sm">
                {advice && <p>{advice}</p>}
                {r.bb100WhenDone != null && r.bb100WhenNot != null && (
                  <p className="text-xs text-muted">
                    {t("leaks.impact", { done: formatStat(r.bb100WhenDone, "bb100", lang), not: formatStat(r.bb100WhenNot, "bb100", lang) })}
                  </p>
                )}
              </div>
            </Card>
          );
        })}
      </ul>
    </div>
  );
}
