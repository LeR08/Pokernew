import type { Theme } from "@/lib/config";

/** Writes theme tokens, density and font scale onto <html> as CSS variables. */
export function applyAppearance(
  root: HTMLElement,
  theme: Theme | undefined,
  density: "compact" | "comfortable",
  fontScale: number,
): void {
  if (theme) {
    for (const [k, v] of Object.entries(theme.colors)) root.style.setProperty(`--c-${k}`, v);
    root.dataset.theme = theme.base;
    root.style.colorScheme = theme.base;
  }
  root.dataset.density = density;
  root.style.setProperty("--density", density === "compact" ? "0.75" : "1");
  const scale = Number.isFinite(fontScale) ? Math.min(1.5, Math.max(0.8, fontScale)) : 1;
  root.style.fontSize = `${16 * scale}px`;
}

/** The active profile may force a theme; otherwise settings.appearance.theme. */
export function effectiveThemeId(settingsTheme: string, profileTheme?: string): string {
  return profileTheme || settingsTheme;
}
