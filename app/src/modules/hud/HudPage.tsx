import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { ClipboardCopy, Eye, EyeOff, Lock, Move } from "lucide-react";
import { getBackend } from "@/lib/backend";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { useConfig, useSaveValue, useSettings, type Profile } from "@/lib/config";
import type { JsonObject } from "@/lib/backend";
import type { HudLayout } from "@/lib/tracker";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";

export function HudPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const status = useQuery({ queryKey: ["hud", "status"], queryFn: async () => (await getBackend()).hudStatus(), refetchInterval: 2000 }).data;
  const room = useConfig<{ hud_allowed: boolean }>("rooms", "winamax").data;
  const settings = useSettings().data;
  const profile = useConfig<Profile & { hud?: string }>("profiles", settings?.general.profile ?? "", !!settings).data;
  const layoutId = profile?.hud ?? "standard";
  const layout = useConfig<HudLayout>("hud", layoutId).data;
  const saveLayout = useSaveValue("hud", layoutId);
  // Local copy so the box answers at once while the file is being saved.
  const [heroOn, setHeroOn] = useState<boolean | null>(null);
  useEffect(() => {
    if (layout) setHeroOn(layout.show_hero ?? false);
  }, [layout]);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    let off: (() => void) | undefined;
    void getBackend().then((b) => {
      off = b.onHudEvent(() => void qc.invalidateQueries({ queryKey: ["hud"] }));
    });
    return () => off?.();
  }, [qc]);
  const refresh = () => qc.invalidateQueries({ queryKey: ["hud"] });
  if (!status) return null;

  const inspector = status.windows.map((w) => `${w.process}\t${w.class}\t${w.title}`).join("\n");
  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>

      {!status.supported && (
        <p className="rounded-md border border-warning p-d3 text-sm text-warning" data-testid="hud-unsupported">
          {t("hud.unsupported")}
        </p>
      )}
      {room && !room.hud_allowed && (
        <p className="rounded-md border border-warning p-d3 text-sm text-warning">
          {t("hud.not_allowed")}{" "}
          <Link className="underline" to="/settings/rooms">
            {t("hud.settings_rooms")}
          </Link>
        </p>
      )}

      <div className="grid grid-cols-1 gap-d3 md:grid-cols-3">
        <Card className="flex flex-col gap-d1 p-d4">
          <span className="text-xs text-muted">{t("hud.tables")}</span>
          <span className="text-2xl font-semibold" data-testid="hud-table-count">
            {status.tables.length}
          </span>
        </Card>
        <Card className="flex flex-col gap-d1 p-d4">
          <span className="text-xs text-muted">{t("hud.play_mode")}</span>
          <span className={cn("flex items-center gap-d2 text-lg font-semibold", status.playMode ? "text-warning" : "text-success")} data-testid="hud-playmode">
            {status.playMode && <Lock className="h-4 w-4" aria-hidden />}
            {status.playMode ? (status.unlockIn > 0 ? t("hud.unlock_in", { s: status.unlockIn }) : t("hud.playing")) : t("hud.studying")}
          </span>
        </Card>
        <Card className="flex flex-col gap-d2 p-d4">
          <Button
            variant="outline"
            size="sm"
            data-testid="hud-toggle"
            onClick={() => void getBackend().then((b) => b.hudSetVisible(!status.visible)).then(refresh)}
          >
            {status.visible ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            {status.visible ? t("hud.hide") : t("hud.show")}
          </Button>
          <label className="flex items-center gap-d2 text-sm">
            <input
              type="checkbox"
              checked={heroOn ?? false}
              disabled={!layout || heroOn === null}
              data-testid="hud-show-hero"
              onChange={(e) => {
                if (!layout) return;
                setHeroOn(e.target.checked);
                saveLayout.mutate({ ...layout, show_hero: e.target.checked } as unknown as JsonObject, {
                  onSuccess: () => void getBackend().then((b) => b.hudSetVisible(status.visible)),
                });
              }}
            />
            {t("hud.show_hero")}
          </label>
          <Button
            variant={status.edit ? "default" : "outline"}
            size="sm"
            data-testid="hud-edit"
            onClick={() => void getBackend().then((b) => b.hudSetEdit(!status.edit)).then(refresh)}
          >
            <Move className="h-4 w-4" />
            {status.edit ? t("hud.edit_off") : t("hud.edit_on")}
          </Button>
        </Card>
      </div>
      <p className="text-xs text-muted">{t("hud.hotkeys_help")}</p>

      <Card className="flex flex-col gap-d2 p-d4">
        <h2 className="font-semibold">{t("hud.detected")}</h2>
        {status.tables.length === 0 ? (
          <p className="text-sm text-muted">{t("hud.none")}</p>
        ) : (
          <ul className="flex flex-col gap-1 text-sm" data-testid="hud-tables">
            {status.tables.map((m) => {
              const editing = status.editTable === m.window.id || status.edit;
              const hidden = status.hidden.includes(m.window.id);
              return (
                <li key={m.window.id} className="flex flex-wrap items-center gap-d2 rounded border border-border px-d3 py-d2" data-testid="hud-table">
                  <span className="font-medium">{m.table.tableName}</span>
                  <span className="text-xs text-muted">
                    {t("hud.max", { n: m.table.maxSeats })} · {m.window.client.w}×{m.window.client.h}
                  </span>
                  <span className="ml-auto flex gap-d2">
                    <Button
                      variant={editing ? "default" : "outline"}
                      size="sm"
                      data-testid="hud-table-edit"
                      onClick={() => void getBackend().then((b) => b.hudSetEditTable(editing ? null : m.window.id)).then(refresh)}
                    >
                      <Move className="h-4 w-4" />
                      {editing ? t("hud.table_done") : t("hud.table_edit")}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      data-testid="hud-table-hide"
                      onClick={() => void getBackend().then((b) => b.hudSetTableHidden(m.window.id, !hidden)).then(refresh)}
                    >
                      {hidden ? <Eye className="h-4 w-4" /> : <EyeOff className="h-4 w-4" />}
                      {hidden ? t("hud.table_show") : t("hud.table_hide")}
                    </Button>
                  </span>
                  <code className="w-full truncate text-xs text-muted">{m.window.title}</code>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      <Card className="flex flex-col gap-d2 p-d4">
        <div className="flex items-center justify-between">
          <h2 className="font-semibold">{t("hud.inspector")}</h2>
          <Button
            variant="outline"
            size="sm"
            disabled={!inspector}
            onClick={() =>
              void navigator.clipboard.writeText(inspector).then(() => {
                setCopied(true);
                setTimeout(() => setCopied(false), 2000);
              })
            }
          >
            <ClipboardCopy className="h-4 w-4" />
            {copied ? t("import.copied") : t("hud.copy_windows")}
          </Button>
        </div>
        <p className="text-xs text-muted">{t("hud.inspector_help")}</p>
        <ul className="flex max-h-64 flex-col gap-0.5 overflow-y-auto font-mono text-xs" data-testid="hud-windows">
          {status.windows.map((w) => (
            <li key={w.id} className="truncate">
              <span className="text-muted">{w.process}</span> · <span className="text-muted">{w.class}</span> · {w.title}
            </li>
          ))}
        </ul>
      </Card>
      <p className="text-xs text-muted">
        {t("hud.layout_help")}{" "}
        <Link className="text-primary underline" to="/settings/profiles">
          {t("hud.profiles")}
        </Link>
      </p>
    </div>
  );
}
