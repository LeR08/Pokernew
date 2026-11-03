import { useTranslation } from "react-i18next";
import { Select } from "@/components/ui/input";
import { useTracker, type Period } from "@/stores/tracker";
import { useAccounts } from "./api";

/** One row of filters above every tracker view. */
export function FilterBar() {
  const { t } = useTranslation();
  const s = useTracker();
  const accounts = useAccounts().data ?? [];
  return (
    <div className="flex flex-wrap items-center gap-d3 rounded-lg border border-border bg-surface p-d3 text-sm" data-testid="filter-bar">
      <label className="flex items-center gap-d2">
        {t("tracker.filter.period")}
        <Select className="w-40" value={s.period} onChange={(e) => s.setPeriod(e.target.value as Period)} data-testid="filter-period">
          {(["all", "7d", "30d", "90d", "year"] as const).map((p) => (
            <option key={p} value={p}>
              {t(`tracker.period.${p}`)}
            </option>
          ))}
        </Select>
      </label>
      <label className="flex items-center gap-d2">
        {t("tracker.filter.format")}
        <Select
          className="w-36"
          value={s.gameType}
          onChange={(e) => s.setGameType(e.target.value as typeof s.gameType)}
          data-testid="filter-format"
        >
          <option value="">{t("tracker.format.all")}</option>
          <option value="cash">{t("tracker.format.cash")}</option>
          <option value="mtt">{t("tracker.format.mtt")}</option>
          <option value="spin">{t("tracker.format.spin")}</option>
        </Select>
      </label>
      {accounts.length > 1 && (
        <label className="flex items-center gap-d2">
          {t("tracker.filter.account")}
          <Select className="w-40" value={s.account} onChange={(e) => s.setAccount(e.target.value)}>
            <option value="">{t("tracker.all_accounts")}</option>
            {accounts.map((a) => (
              <option key={a} value={a}>
                {a}
              </option>
            ))}
          </Select>
        </label>
      )}
      <label className="ml-auto flex cursor-pointer items-center gap-d2" title={t("tracker.euros_help")}>
        <input type="checkbox" checked={s.euros} onChange={(e) => s.setEuros(e.target.checked)} data-testid="toggle-euros" />
        {t("tracker.show_euros")}
      </label>
    </div>
  );
}
