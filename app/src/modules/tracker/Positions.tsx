import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Select } from "@/components/ui/input";
import { bb, colourClass, euros, formatStat, int, resultClass } from "@/lib/format";
import type { Group } from "@/lib/tracker";
import { useTracker } from "@/stores/tracker";
import { useFilter, useReport } from "./api";

/** A report: one row per group (position by default), all stats as columns. */
export function Positions() {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const [group, setGroup] = useState<Group>("position");
  const filter = useFilter();
  const euro = useTracker((s) => s.euros);
  const report = useReport(filter, group).data;
  const stats = report?.stats.filter((s) => s.id !== "bb100") ?? [];
  return (
    <div className="flex flex-col gap-d3">
      <label className="flex items-center gap-d2 text-sm">
        {t("tracker.group_by")}
        <Select className="w-44" value={group} onChange={(e) => setGroup(e.target.value as Group)} data-testid="group-by">
          {(["position", "game_type", "month", "stake", "account"] as const).map((g) => (
            <option key={g} value={g}>
              {t(`tracker.group.${g}`)}
            </option>
          ))}
        </Select>
      </label>
      <div className="overflow-x-auto rounded-lg border border-border">
        <table className="w-full text-sm tabular-nums" data-testid="report-table">
          <thead className="bg-surface-alt text-left text-xs text-muted">
            <tr>
              <th className="whitespace-nowrap px-d3 py-d2 font-medium">{t(`tracker.group.${group}`)}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{t("tracker.hands")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">{euro ? t("tracker.col_cash_eur") : t("tracker.col_net_bb")}</th>
              <th className="whitespace-nowrap px-d3 py-d2 text-right font-medium">bb/100</th>
              {stats.map((s) => (
                <th key={s.id} className="whitespace-nowrap px-d3 py-d2 text-right font-medium" title={typeof s.description === "string" ? s.description : s.description?.[lang === "fr" ? "fr" : "en"]}>
                  {s.name}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {report?.rows.map((r) => (
              <tr key={r.key} className="border-t border-border">
                <td className="whitespace-nowrap px-d3 py-d2 font-medium">{group === "game_type" ? t(`tracker.format.${r.key}`, { defaultValue: r.key }) : r.key}</td>
                <td className="whitespace-nowrap px-d3 py-d2 text-right">{int(r.hands, lang)}</td>
                <td className={`whitespace-nowrap px-d3 py-d2 text-right ${resultClass(euro ? r.netCashCents : r.netBb)}`}>
                  {euro ? euros(r.netCashCents, lang) : bb(r.netBb, lang)}
                </td>
                <td className={`whitespace-nowrap px-d3 py-d2 text-right ${resultClass(r.bbPer100 ?? 0)}`}>{formatStat(r.bbPer100, "bb100", lang)}</td>
                {stats.map((s) => {
                  const v = r.stats.find((x) => x.id === s.id);
                  return (
                    <td key={s.id} className={`whitespace-nowrap px-d3 py-d2 text-right ${colourClass(v?.value, s.colourRanges)}`} title={t("tracker.sample", { n: v?.sample ?? 0 })}>
                      {formatStat(v?.value, s.format, lang)}
                    </td>
                  );
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      {euro && <p className="text-xs text-muted">{t("tracker.eur_cash_only")}</p>}
    </div>
  );
}
