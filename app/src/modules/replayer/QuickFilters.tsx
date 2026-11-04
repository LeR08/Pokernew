import { useTranslation } from "react-i18next";
import { useTracker } from "@/stores/tracker";

/** Quick filters of the Replayer list: player at the table, won/lost, minimum result in bb. */
export function QuickFilters() {
  const { t } = useTranslation();
  const adv = useTracker((s) => s.adv);
  const setAdv = useTracker((s) => s.setAdv);
  const field = "h-8 rounded-md border border-border bg-surface px-d2 text-sm";
  return (
    <div className="flex flex-wrap items-end gap-d3 text-sm" data-testid="replayer-quick">
      <label className="flex flex-col gap-1">
        <span className="text-xs text-muted">{t("replayer.quick_player")}</span>
        <input
          className={field}
          value={adv.withPlayer}
          onChange={(e) => setAdv({ withPlayer: e.target.value })}
          data-testid="quick-player"
        />
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-muted">{t("replayer.quick_result")}</span>
        <select
          className={field}
          value={adv.result}
          onChange={(e) => setAdv({ result: e.target.value as "" | "won" | "lost" })}
          data-testid="quick-result"
        >
          <option value="">{t("replayer.quick_any")}</option>
          <option value="won">{t("replayer.quick_won")}</option>
          <option value="lost">{t("replayer.quick_lost")}</option>
        </select>
      </label>
      <label className="flex flex-col gap-1">
        <span className="text-xs text-muted">{t("replayer.quick_min_bb")}</span>
        <input
          type="number"
          min={0}
          step={1}
          className={`${field} w-28`}
          value={adv.minBb ?? ""}
          onChange={(e) => setAdv({ minBb: e.target.value === "" ? null : Math.max(0, Number(e.target.value)) })}
          data-testid="quick-min-bb"
        />
      </label>
    </div>
  );
}

const LIST_KEY = "lps.replayer.list";

/** Remembers the hand ids of the list the user picked from (for previous / next hand). */
export function rememberList(ids: number[]) {
  try {
    sessionStorage.setItem(LIST_KEY, JSON.stringify(ids));
  } catch {
    /* ignore */
  }
}

export function recallList(): number[] {
  try {
    const v = JSON.parse(sessionStorage.getItem(LIST_KEY) ?? "[]") as unknown;
    return Array.isArray(v) ? v.filter((x): x is number => typeof x === "number") : [];
  } catch {
    return [];
  }
}
