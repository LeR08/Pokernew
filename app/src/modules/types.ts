import type { ComponentType } from "react";
import type { LucideIcon } from "lucide-react";
import type { JSONSchema } from "@/lib/schema";

/** A page of a module, mounted under `/m/<module id>/<path>`. */
export interface ModuleRoute {
  /** Relative path; "" is the module's home page. */
  path: string;
  component: ComponentType<{ manifest: ModuleManifest }>;
}

/** A command a module exposes (command palette / hotkeys in later phases). */
export interface ModuleCommand {
  id: string;
  /** i18n key */
  title: string;
  run: () => void | Promise<void>;
}

/**
 * Module contract. Every folder `src/modules/<id>/` exports a `manifest`
 * from `manifest.ts`; the launcher builds tiles, sidebar entries, routes
 * and settings pages from these objects only — adding a module never
 * touches the shell.
 */
export interface ModuleManifest {
  id: string;
  /** i18n key of the display name. */
  name: string;
  /** i18n key of the one-line description. */
  description: string;
  icon: LucideIcon;
  /** Roadmap phase in which the module becomes functional. */
  phase: number;
  /** Paid tools this module replaces (shown on the tile). */
  replaces: string[];
  routes: ModuleRoute[];
  /** Module-specific settings, stored in settings.toml [module_settings.<id>]. */
  settingsSchema?: JSONSchema;
  /** Default values for settingsSchema. */
  settingsDefaults?: Record<string, unknown>;
  commands?: ModuleCommand[];
  /**
   * Study-only module: locked while a poker table is open (fair-play rule,
   * enforced from phase 3 when table detection exists).
   */
  studyOnly?: boolean;
}
