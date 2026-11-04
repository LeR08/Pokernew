/** One-click ranges for a player of the calculator: my charts, or the range estimated for a tracked player. */
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { estimatedRange, RANGE_KINDS, type RangeKind } from "@/lib/playerRange";
import { formatRange } from "@/lib/range";
import type { ReportRow } from "@/lib/tracker";
import { useEquity } from "./store";

/** Tracked players with at least 30 hands (shared by every row). */
export function useTrackedPlayers(enabled = true) {
  return useQuery({
    queryKey: ["equity", "tracked-players"],
    queryFn: async () => (await (await getBackend()).trackerOpponents({}, 30, 1000)).rows,
    enabled,
    staleTime: 60_000,
  });
}

export function useRangeNames() {
  return useQuery({
    queryKey: ["equity", "range-names"],
    queryFn: async () => (await getBackend()).rangesList(),
    staleTime: 60_000,
  });
}

/** The estimated range of `name` for `kind`, as range text (null if unknown). */
export function rangeOf(rows: ReportRow[] | undefined, name: string, kind: RangeKind): string | null {
  const row = rows?.find((r) => r.key === name);
  const w = row ? estimatedRange(row, kind) : null;
  return w ? formatRange(w) : null;
}

/** "Format of my ranges" picker, above the players. */
export function ChartFolderPicker() {
  const { t } = useTranslation();
  const s = useEquity();
  const names = useRangeNames().data ?? [];
  const folders = [...new Set(names.filter((n) => n.includes("/")).map((n) => n.split("/")[0]))].sort();
  return (
    <label className="flex items-center gap-d2 text-sm">
      <span className="text-muted">{t("equity.my_ranges_format")}</span>
      <Select className="w-52" value={s.chartFolder} onChange={(e) => s.set({ chartFolder: e.target.value })} data-testid="chart-folder">
        <option value="">—</option>
        {folders.map((f) => (
          <option key={f} value={f}>
            {f}
          </option>
        ))}
      </Select>
    </label>
  );
}

export function QuickRanges({ index }: { index: number }) {
  const { t } = useTranslation();
  const s = useEquity();
  const names = useRangeNames().data ?? [];
  const players = useTrackedPlayers().data;
  const who = s.who[index] ?? "";
  const apply = (text: string, name: string | null) => {
    const w = [...s.who];
    while (w.length < s.players.length) w.push(null);
    w[index] = name;
    s.set({ players: s.players.map((p, j) => (j === index ? text : p)), who: w });
  };
  const charts = names.filter((n) => s.chartFolder && n.startsWith(`${s.chartFolder}/`));
  return (
    <div className="flex flex-wrap items-center gap-d2 pl-[6.25rem] text-xs" data-testid="quick-ranges">
      <Select
        className="h-8 w-56 text-xs"
        value=""
        disabled={!charts.length}
        aria-label={t("equity.my_ranges")}
        data-testid="quick-chart"
        onChange={(e) => {
          const n = e.target.value;
          if (n) void getBackend().then((b) => b.rangeLoad(n)).then((text) => apply(text.trim(), null));
        }}
      >
        <option value="">{charts.length ? t("equity.my_ranges") : t("equity.my_ranges_pick_format")}</option>
        {charts.map((n) => (
          <option key={n} value={n}>
            {n.slice(s.chartFolder.length + 1)}
          </option>
        ))}
      </Select>
      <Select
        className="h-8 w-48 text-xs"
        value={who}
        aria-label={t("equity.tracked_player")}
        data-testid="quick-player"
        onChange={(e) => {
          const name = e.target.value;
          const text = name ? rangeOf(players, name, "pfr") : null;
          if (text) apply(text, name);
        }}
      >
        <option value="">{t("equity.tracked_player")}</option>
        {(players ?? []).map((r) => (
          <option key={r.key} value={r.key}>
            {r.key} ({r.hands})
          </option>
        ))}
      </Select>
      {who && (
        <span className="flex gap-1" data-testid="quick-kinds">
          {RANGE_KINDS.map((k) => (
            <button
              key={k}
              type="button"
              className="rounded border border-border px-1.5 py-0.5 hover:border-primary"
              data-testid={`quick-kind-${k}`}
              onClick={() => {
                const text = rangeOf(players, who, k);
                if (text) apply(text, who);
              }}
            >
              {t(`sheet.kind.${k}`)}
            </button>
          ))}
        </span>
      )}
    </div>
  );
}
