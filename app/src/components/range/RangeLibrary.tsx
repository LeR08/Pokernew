/** Saved ranges (<data>/ranges/**.txt): load, save as, delete. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";

export function RangeLibrary({ value, onLoad }: { value: string; onLoad: (text: string) => void }) {
  const { t } = useTranslation();
  const [names, setNames] = useState<string[]>([]);
  const [pick, setPick] = useState("");
  const [name, setName] = useState("");
  const [msg, setMsg] = useState<string | null>(null);

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
    <details className="rounded-md border border-border p-d2 text-sm" data-testid="range-library">
      <summary className="cursor-pointer font-medium">{t("range.library")}</summary>
      <div className="mt-d2 flex flex-col gap-d2">
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
