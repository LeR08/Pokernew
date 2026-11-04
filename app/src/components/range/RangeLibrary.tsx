/** Saved ranges (<data>/ranges/**.txt): load, save as, delete. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { formatRange } from "@/lib/range";
import type { ReportRow } from "@/lib/tracker";
import { useFilter } from "@/modules/tracker/api";
import { estimatedRange } from "@/modules/tracker/PlayerSheet";

export function RangeLibrary({ value, onLoad }: { value: string; onLoad: (text: string) => void }) {
  const { t } = useTranslation();
  const [names, setNames] = useState<string[]>([]);
  const [pick, setPick] = useState("");
  const [name, setName] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [folder, setFolder] = useState("");
  const [players, setPlayers] = useState<ReportRow[] | null>(null);
  const [player, setPlayer] = useState("");
  const filter = useFilter();
  const folders = [...new Set(names.filter((n) => n.includes("/")).map((n) => n.split("/")[0]))].sort();
  const inFolder = names.filter((n) => folder && n.startsWith(`${folder}/`));
  const loadPlayers = () =>
    players === null &&
    void getBackend()
      .then((b) => b.trackerOpponents(filter, 30, 500))
      .then((r) => setPlayers(r.rows))
      .catch(() => setPlayers([]));
  const fromPlayer = (kind: "pfr" | "call" | "three_bet" | "vpip") => {
    const row = players?.find((r) => r.key === player);
    const w = row ? estimatedRange(row, kind) : null;
    if (w) {
      onLoad(formatRange(w));
      setMsg(t("range.lib_from_player", { name: player }));
    }
  };

  const refresh = () =>
    void getBackend()
      .then((b) => b.rangesList())
      .then(setNames)
      .catch(() => setNames([]));
  useEffect(refresh, []);

  const run = (f: () => (Promise<unknown>), ok: string) =>
    void f()
      .then(() => {
        setMsg(ok);
        refresh();
      })
      .catch((e: Error) => setMsg(e.message));

  return (
    <details className="rounded-md border border-border p-d2 text-sm" data-testid="range-library" open>
      <summary className="cursor-pointer font-medium">{t("range.library")}</summary>
      <div className="mt-d2 flex flex-col gap-d2">
        <div className="flex flex-wrap gap-d2" data-testid="range-presets">
          <Select className="w-48" value={folder} onChange={(e) => setFolder(e.target.value)} aria-label={t("range.lib_format")} data-testid="range-preset-folder">
            <option value="">{t("range.lib_format")}</option>
            {folders.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </Select>
          <Select
            className="w-56"
            value=""
            disabled={!folder}
            onChange={(e) => e.target.value && run(async () => onLoad(await (await getBackend()).rangeLoad(e.target.value)), t("range.lib_loaded", { name: e.target.value }))}
            aria-label={t("range.lib_situation")}
            data-testid="range-preset-situation"
          >
            <option value="">{t("range.lib_situation")}</option>
            {inFolder.map((n) => (
              <option key={n} value={n}>
                {n.slice(folder.length + 1)}
              </option>
            ))}
          </Select>
        </div>
        <div className="flex flex-wrap items-center gap-d2" data-testid="range-from-player">
          <Select className="w-48" value={player} onFocus={loadPlayers} onChange={(e) => setPlayer(e.target.value)} aria-label={t("range.lib_player")} data-testid="range-player">
            <option value="">{t("range.lib_player")}</option>
            {(players ?? []).map((r) => (
              <option key={r.key} value={r.key}>
                {r.key} ({r.hands})
              </option>
            ))}
          </Select>
          {(["pfr", "call", "three_bet", "vpip"] as const).map((k) => (
            <Button key={k} size="sm" variant="outline" disabled={!player} onClick={() => fromPlayer(k)} data-testid={`range-player-${k}`}>
              {t(`sheet.kind.${k}`)}
            </Button>
          ))}
        </div>
        <div className="flex gap-d2">
          <Select value={pick} onChange={(e) => setPick(e.target.value)} data-testid="range-lib-pick" aria-label={t("range.library")}>
            <option value="">{names.length ? t("range.lib_choose") : t("range.lib_empty")}</option>
            {names.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </Select>
          <Button
            size="sm"
            variant="outline"
            disabled={!pick}
            data-testid="range-lib-load"
            onClick={() => run(async () => onLoad(await (await getBackend()).rangeLoad(pick)), t("range.lib_loaded", { name: pick }))}
          >
            {t("range.lib_load")}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            disabled={!pick}
            onClick={() => run(async () => (await getBackend()).rangeDelete(pick), t("range.lib_deleted", { name: pick }))}
          >
            {t("range.lib_delete")}
          </Button>
        </div>
        <div className="flex gap-d2">
          <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="6max/100bb/CO_open" data-testid="range-lib-name" />
          <Button
            size="sm"
            disabled={!name.trim() || !value.trim()}
            data-testid="range-lib-save"
            onClick={() => run(async () => (await getBackend()).rangeSave(name.trim(), value), t("range.lib_saved", { name: name.trim() }))}
          >
            {t("range.lib_save")}
          </Button>
        </div>
        {msg && <p className="text-xs text-muted" data-testid="range-lib-msg">{msg}</p>}
      </div>
    </details>
  );
}
