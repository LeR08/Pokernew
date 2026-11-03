import { useEffect, useState } from "react";
import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Copy, Trash2 } from "lucide-react";
import { getBackend, type Family, type JsonObject, type JsonValue } from "@/lib/backend";
import { useAppInfo, useConfigList, useSaveValue, useSettings } from "@/lib/config";
import { allModules } from "@/modules/registry";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Select } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import { FileEditor, useFormContext } from "./FileEditor";
import { SchemaForm } from "./SchemaForm";
import { isValidId } from "./ids";

const TABS = ["general", "rooms", "hotkeys", "themes", "profiles", "modules", "about"] as const;
type Tab = (typeof TABS)[number];

/** Editor for a folder family (themes, profiles): pick a file, copy, delete. */
function FolderEditor({ family }: { family: Family }) {
  const { t } = useTranslation();
  const ids = useConfigList(family).data ?? [];
  const [id, setId] = useState("");
  useEffect(() => {
    if (ids.length && !ids.includes(id)) setId(ids[0]);
  }, [ids, id]);

  const duplicate = async () => {
    const newId = window.prompt(t("settings.new_file_prompt"), `${id}-copy`);
    if (!newId || !isValidId(newId) || ids.includes(newId)) return;
    const b = await getBackend();
    await b.configSaveText(family, newId, await b.configRaw(family, id));
    setId(newId);
  };
  const remove = async () => {
    if (ids.length <= 1 || !window.confirm(t("settings.delete_confirm", { id }))) return;
    await (await getBackend()).configDelete(family, id);
  };

  return (
    <div className="flex flex-col gap-d4">
      <div className="flex flex-wrap items-center gap-d2">
        <Select aria-label={t("settings.file")} value={id} onChange={(e) => setId(e.target.value)} className="w-56" data-testid="file-select">
          {ids.map((x) => (
            <option key={x} value={x}>
              {x}
            </option>
          ))}
        </Select>
        <Button variant="outline" size="sm" onClick={() => void duplicate()} disabled={!id}>
          <Copy className="h-4 w-4" />
          {t("settings.new_file", { id })}
        </Button>
        <Button variant="outline" size="sm" onClick={() => void remove()} disabled={ids.length <= 1}>
          <Trash2 className="h-4 w-4" />
          {t("settings.delete")}
        </Button>
      </div>
      {id && <FileEditor key={id} family={family} id={id} />}
    </div>
  );
}

/** Settings declared by module manifests, stored in settings.toml [module_settings.<id>]. */
function ModuleSettings() {
  const { t } = useTranslation();
  const settings = useSettings().data;
  const save = useSaveValue("settings", "settings");
  const ctx = useFormContext();
  const withSettings = allModules.filter((m) => m.settingsSchema);
  if (!settings) return null;
  if (withSettings.length === 0) return <p className="text-muted">{t("settings.no_module_settings")}</p>;
  return (
    <div className="flex flex-col gap-d4">
      {withSettings.map((m) => {
        const value = (settings.module_settings?.[m.id] ?? m.settingsDefaults ?? {}) as JsonValue;
        return (
          <Card key={m.id} className="p-d4">
            <h3 className="mb-d3 font-semibold">{t(m.name)}</h3>
            <SchemaForm
              schema={m.settingsSchema!}
              value={value}
              ctx={ctx}
              onChange={(v) =>
                save.mutate({
                  ...settings,
                  module_settings: { ...(settings.module_settings ?? {}), [m.id]: v },
                } as unknown as JsonObject)
              }
            />
          </Card>
        );
      })}
    </div>
  );
}

function About() {
  const { t } = useTranslation();
  const info = useAppInfo().data;
  if (!info) return null;
  return (
    <dl className="grid grid-cols-[12rem_1fr] gap-d2 text-sm" data-testid="about">
      <dt className="text-muted">{t("about.version")}</dt>
      <dd>{info.version}</dd>
      <dt className="text-muted">{t("about.data_dir")}</dt>
      <dd className="font-mono">{info.dataDir}</dd>
      <dt className="text-muted">{t("about.source")}</dt>
      <dd data-testid="data-dir-source">{t(`about.sources.${info.dataDirSource}`)}</dd>
      <dt />
      <dd className="text-muted">{info.mock ? t("about.mock") : t("about.startup", { ms: info.backendReadyMs })}</dd>
      <dt />
      <dd className="text-muted">{t("about.offline")}</dd>
    </dl>
  );
}

export function SettingsCentre() {
  const { t } = useTranslation();
  const { tab = "general" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? (tab as Tab) : "general";
  return (
    <div className="mx-auto flex max-w-5xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t("settings.title")}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t("settings.title")}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/settings/${x}`}
            className={cn(
              "-mb-px border-b-2 px-d3 py-d2 text-sm",
              x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg",
            )}
          >
            {t(`settings.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "general" && <FileEditor family="settings" id="settings" />}
      {current === "rooms" && <FolderEditor family="rooms" />}
      {current === "hotkeys" && <FileEditor family="hotkeys" id="hotkeys" />}
      {current === "themes" && <FolderEditor family="themes" />}
      {current === "profiles" && <FolderEditor family="profiles" />}
      {current === "modules" && <ModuleSettings />}
      {current === "about" && <About />}
    </div>
  );
}
