import { useQueries } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Check, Palette } from "lucide-react";
import { getBackend, type JsonObject } from "@/lib/backend";
import { qk, useConfigList, useSaveValue, useSettings, type Theme } from "@/lib/config";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const SWATCH = ["background", "surface", "primary", "accent", "success", "danger"];

/** Pick a theme in one click; "Customise" copies the current theme into custom.toml and opens its editor. */
export function ThemeGallery({ onEdit }: { onEdit: (id: string) => void }) {
  const { t, i18n } = useTranslation();
  const ids = useConfigList("themes").data ?? [];
  const settings = useSettings().data;
  const save = useSaveValue("settings", "settings");
  const themes = useQueries({
    queries: ids.map((id) => ({
      queryKey: qk.config("themes", id),
      queryFn: async () => (await (await getBackend()).configGet("themes", id)) as unknown as Theme,
      staleTime: Infinity,
    })),
  });
  const current = settings?.appearance.theme ?? "";
  const pick = (id: string) => {
    if (!settings) return;
    save.mutate({ ...settings, appearance: { ...settings.appearance, theme: id } } as unknown as JsonObject);
  };
  const customise = async () => {
    const b = await getBackend();
    if (!ids.includes("custom")) await b.configSaveText("themes", "custom", await b.configRaw("themes", current || "dark"));
    pick("custom");
    onEdit("custom");
  };
  // "Océan / Ocean" -> the half matching the UI language
  const label = (name: string) => {
    const parts = name.split(" / ");
    return parts.length === 2 ? parts[i18n.language === "en" ? 1 : 0] : name;
  };
  return (
    <div className="flex flex-col gap-d3" data-testid="theme-gallery">
      <div className="grid grid-cols-2 gap-d3 sm:grid-cols-3 lg:grid-cols-5">
        {ids.map((id, i) => {
          const th = themes[i]?.data;
          if (!th) return null;
          return (
            <button
              key={id}
              type="button"
              onClick={() => pick(id)}
              data-testid={`theme-${id}`}
              className={cn(
                "flex flex-col gap-d2 rounded-lg border p-d2 text-left text-sm",
                id === current ? "border-primary ring-2 ring-primary" : "border-border hover:border-primary",
              )}
              style={{ background: th.colors.background, color: th.colors.text }}
            >
              <div className="flex h-8 overflow-hidden rounded">
                {SWATCH.map((k) => (
                  <span key={k} className="flex-1" style={{ background: th.colors[k] }} />
                ))}
              </div>
              <span className="flex items-center justify-between gap-1 font-medium">
                {label(th.name)}
                {id === current && <Check className="h-4 w-4" />}
              </span>
            </button>
          );
        })}
      </div>
      <div>
        <Button variant="outline" size="sm" onClick={() => void customise()} data-testid="theme-customise">
          <Palette className="h-4 w-4" />
          {t("settings.theme_customise")}
        </Button>
        <span className="ml-d3 text-xs text-muted">{t("settings.theme_customise_hint")}</span>
      </div>
    </div>
  );
}
