import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useConfig, useSettings, type Profile, type Theme } from "@/lib/config";
import { applyAppearance, effectiveThemeId } from "./apply";

/** Keeps theme, density, font scale and language in sync with the config files. */
export function AppearanceSync() {
  const { i18n } = useTranslation();
  const settings = useSettings().data;
  const profile = useConfig<Profile>("profiles", settings?.general.profile ?? "", !!settings).data;
  const themeId = settings ? effectiveThemeId(settings.appearance.theme, profile?.theme) : "";
  const theme = useConfig<Theme>("themes", themeId, !!themeId).data;

  useEffect(() => {
    if (!settings) return;
    applyAppearance(document.documentElement, theme, settings.appearance.density, settings.appearance.font_scale);
  }, [settings, theme]);

  useEffect(() => {
    const lng = settings?.general.language;
    if (lng && i18n.language !== lng) {
      void i18n.changeLanguage(lng);
      document.documentElement.lang = lng;
    }
  }, [settings?.general.language, i18n]);

  return null;
}
