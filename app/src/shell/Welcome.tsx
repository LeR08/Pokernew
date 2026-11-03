/**
 * First-run wizard: language, rooms (detected hand-history folders and
 * screen names), default profile, first import. "Later" skips it; it can be
 * redone from the settings (rooms tab).
 */
import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Check, FolderOpen, FolderSearch } from "lucide-react";
import { getBackend, type FoundAccount, type JsonObject } from "@/lib/backend";
import { qk, useConfig, useConfigList, useSettings, type Settings } from "@/lib/config";
import { useDbStats, useImportEvents } from "@/lib/importApi";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Switch } from "@/components/ui/input";
import { cn } from "@/lib/utils";

const STEPS = ["language", "rooms", "profile", "import"] as const;

interface RoomFile {
  name: string;
  enabled: boolean;
  folders: string[];
  hero_names: string[];
  hud_allowed: boolean;
}

export function Welcome() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const settings = useSettings().data;
  const room = useConfig<RoomFile>("rooms", "winamax").data;
  const profiles = useConfigList("profiles").data ?? [];
  const stats = useDbStats().data;
  useImportEvents();
  const [step, setStep] = useState(0);
  const [found, setFound] = useState<FoundAccount[] | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [extra, setExtra] = useState<string[]>([]);
  const [hudAllowed, setHudAllowed] = useState(true);

  useEffect(() => {
    void getBackend()
      .then((b) => b.detectRooms())
      .then((f) => {
        setFound(f);
        setSelected(f.map((a) => a.folder));
      });
  }, []);
  useEffect(() => {
    if (room) setHudAllowed(room.hud_allowed);
  }, [room]);

  if (!settings) return null;

  const saveSettings = async (patch: (s: Settings) => Settings) => {
    const b = await getBackend();
    await b.configSaveValue("settings", "settings", patch(settings) as unknown as JsonObject);
    await qc.invalidateQueries({ queryKey: qk.config("settings", "settings") });
  };

  const saveRooms = async () => {
    if (!room) return;
    const accounts = (found ?? []).filter((a) => selected.includes(a.folder));
    const folders = [...new Set([...room.folders, ...accounts.map((a) => a.folder), ...extra])];
    const hero_names = [...new Set([...room.hero_names, ...accounts.map((a) => a.account)])];
    const b = await getBackend();
    await b.configSaveValue("rooms", "winamax", { ...room, folders, hero_names, hud_allowed: hudAllowed } as unknown as JsonObject);
  };

  const finish = async () => {
    await saveSettings((s) => ({ ...s, general: { ...s.general, first_run_done: true } }) as Settings);
    navigate("/");
  };

  const next = async () => {
    if (STEPS[step] === "rooms") await saveRooms();
    setStep((s) => Math.min(s + 1, STEPS.length - 1));
  };

  const current = STEPS[step];
  return (
    <div className="flex min-h-screen items-center justify-center bg-bg p-d6 text-fg" data-testid="welcome">
      <Card className="flex w-full max-w-2xl flex-col gap-d4 p-d6">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-semibold">{t("welcome.title")}</h1>
          <Button variant="ghost" size="sm" onClick={() => void finish()} data-testid="welcome-skip">
            {t("welcome.later")}
          </Button>
        </div>
        <ol className="flex gap-d2 text-xs">
          {STEPS.map((s, i) => (
            <li
              key={s}
              className={cn("flex-1 rounded px-d2 py-1 text-center", i === step ? "bg-primary text-primary-fg" : i < step ? "bg-surface-alt" : "text-muted")}
            >
              {i + 1}. {t(`welcome.step.${s}`)}
            </li>
          ))}
        </ol>

        {current === "language" && (
          <div className="flex flex-col gap-d3">
            <p>{t("welcome.language_intro")}</p>
            <div className="flex gap-d2">
              {(["fr", "en"] as const).map((l) => (
                <Button
                  key={l}
                  variant={settings.general.language === l ? "default" : "outline"}
                  onClick={() => void saveSettings((s) => ({ ...s, general: { ...s.general, language: l } }))}
                >
                  {t(`enum.${l}`)}
                </Button>
              ))}
            </div>
          </div>
        )}

        {current === "rooms" && (
          <div className="flex flex-col gap-d3">
            <p>{t("welcome.rooms_intro")}</p>
            {found === null ? (
              <p className="text-muted">{t("welcome.searching")}</p>
            ) : found.length === 0 ? (
              <p className="text-warning">{t("welcome.none_found")}</p>
            ) : (
              <ul className="flex flex-col gap-d2" data-testid="found-accounts">
                {found.map((a) => (
                  <li key={a.folder} className="flex items-center gap-d3 rounded border border-border p-d3">
                    <Switch
                      label={a.account}
                      checked={selected.includes(a.folder)}
                      onChange={(on) => setSelected((s) => (on ? [...s, a.folder] : s.filter((x) => x !== a.folder)))}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="font-medium">
                        Winamax · {a.account}
                      </div>
                      <code className="block truncate text-xs text-muted">{a.folder}</code>
                    </div>
                    <span className="text-xs text-muted">{t("welcome.files", { count: a.files })}</span>
                  </li>
                ))}
              </ul>
            )}
            {extra.map((f) => (
              <code key={f} className="truncate rounded border border-border p-d2 text-xs">
                {f}
              </code>
            ))}
            <Button
              variant="outline"
              size="sm"
              className="w-fit"
              onClick={() =>
                void getBackend()
                  .then((b) => b.pickPaths({ directory: true, multiple: false, title: t("welcome.add_folder") }))
                  .then((p) => p && setExtra((e) => [...e, ...p]))
              }
            >
              <FolderOpen className="h-4 w-4" />
              {t("welcome.add_folder")}
            </Button>
            <label className="flex items-center gap-d3 text-sm">
              <Switch label={t("welcome.hud_allowed")} checked={hudAllowed} onChange={setHudAllowed} />
              {t("welcome.hud_allowed")}
            </label>
            <p className="text-xs text-muted">{t("welcome.hud_allowed_help")}</p>
          </div>
        )}

        {current === "profile" && (
          <div className="flex flex-col gap-d3">
            <p>{t("welcome.profile_intro")}</p>
            <div className="flex flex-wrap gap-d2">
              {profiles.map((p) => (
                <Button
                  key={p}
                  variant={settings.general.profile === p ? "default" : "outline"}
                  onClick={() => void saveSettings((s) => ({ ...s, general: { ...s.general, profile: p } }))}
                >
                  {p}
                </Button>
              ))}
            </div>
          </div>
        )}

        {current === "import" && (
          <div className="flex flex-col gap-d3">
            <p>{t("welcome.import_intro")}</p>
            <div className="flex items-center gap-d3 rounded border border-border p-d3">
              <FolderSearch className="h-6 w-6 text-primary" aria-hidden />
              <span data-testid="welcome-hands">{t("welcome.hands_so_far", { count: stats?.hands ?? 0 })}</span>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="w-fit"
              onClick={() =>
                void getBackend().then(async (b) => {
                  const p = await b.pickPaths({ directory: true, multiple: false, title: t("welcome.import_other") });
                  if (p) await b.importPaths(p);
                })
              }
            >
              <FolderOpen className="h-4 w-4" />
              {t("welcome.import_other")}
            </Button>
          </div>
        )}

        <div className="flex justify-between">
          <Button variant="outline" onClick={() => setStep((s) => Math.max(0, s - 1))} disabled={step === 0}>
            {t("welcome.back")}
          </Button>
          {step < STEPS.length - 1 ? (
            <Button onClick={() => void next()} data-testid="welcome-next">
              {t("welcome.next")}
            </Button>
          ) : (
            <Button onClick={() => void finish()} data-testid="welcome-finish">
              <Check className="h-4 w-4" />
              {t("welcome.finish")}
            </Button>
          )}
        </div>
      </Card>
    </div>
  );
}
