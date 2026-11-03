import { useTranslation } from "react-i18next";
import { useConfigList, useSaveValue, useSettings } from "@/lib/config";
import type { JsonObject } from "@/lib/backend";
import { Select } from "@/components/ui/input";

/** One-click profile switch (writes general.profile in settings.toml). */
export function ProfileSwitcher() {
  const { t } = useTranslation();
  const settings = useSettings().data;
  const profiles = useConfigList("profiles").data ?? [];
  const save = useSaveValue("settings", "settings");
  if (!settings) return null;
  return (
    <label className="flex flex-col gap-d1 px-d3 pb-d2 text-xs text-muted">
      {t("profile.label")}
      <Select
        aria-label={t("profile.switch")}
        data-testid="profile-switcher"
        value={settings.general.profile}
        onChange={(e) =>
          save.mutate({ ...settings, general: { ...settings.general, profile: e.target.value } } as unknown as JsonObject)
        }
      >
        {profiles.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </Select>
    </label>
  );
}
