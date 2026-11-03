import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useConfig, useConfigList, useSaveValue, useSettings, type Hotkeys } from "@/lib/config";
import type { JsonObject } from "@/lib/backend";
import { useUi } from "@/stores/ui";
import { matches } from "./keys";

/** In-app hotkeys from hotkeys.toml (global hotkeys come with the HUD, phase 3). */
export function useAppHotkeys(): void {
  const navigate = useNavigate();
  const hotkeys = useConfig<Hotkeys>("hotkeys", "hotkeys").data;
  const settings = useSettings().data;
  const profiles = useConfigList("profiles").data ?? [];
  const saveSettings = useSaveValue("settings", "settings");
  const toggleSidebar = useUi((s) => s.toggleSidebar);

  useEffect(() => {
    if (!hotkeys) return;
    const actions: Record<string, () => void> = {
      go_home: () => navigate("/"),
      open_settings: () => navigate("/settings"),
      toggle_sidebar: toggleSidebar,
      next_profile: () => {
        if (!settings || profiles.length === 0) return;
        const i = profiles.indexOf(settings.general.profile);
        const next = profiles[(i + 1) % profiles.length];
        saveSettings.mutate({ ...settings, general: { ...settings.general, profile: next } } as unknown as JsonObject);
      },
    };
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("[data-hotkey-capture], .cm-editor")) return;
      for (const [action, combo] of Object.entries(hotkeys.actions)) {
        if (matches(e, combo) && actions[action]) {
          e.preventDefault();
          actions[action]();
          return;
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [hotkeys, settings, profiles, navigate, toggleSidebar, saveSettings]);
}
