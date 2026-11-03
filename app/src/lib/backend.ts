/**
 * Single entry point to the Rust core. In the Tauri app it calls the
 * `#[tauri::command]`s of src-tauri; in a plain browser (Vite dev server,
 * Vitest, Playwright) it falls back to an in-memory mock built from
 * config/defaults, so the UI can be developed and tested without Tauri.
 */
import type { JSONSchema } from "./schema";

export type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
export type JsonObject = { [k: string]: JsonValue };

export type Family = "settings" | "hotkeys" | "themes" | "profiles";

export interface Issue {
  family: string;
  id: string;
  /** JSON pointer to the offending value; "" = whole file (e.g. syntax error). */
  pointer: string;
  message: string;
}

export type ConfigEvent =
  | { kind: "changed"; family: string; id: string }
  | { kind: "removed"; family: string; id: string }
  | { kind: "invalid"; family: string; id: string; issues: Issue[] };

export interface AppInfo {
  version: string;
  dataDir: string;
  dataDirSource: "command_line" | "environment" | "portable" | "default";
  /** Milliseconds from process start to backend ready. */
  backendReadyMs: number;
  mock: boolean;
}

export class BackendError extends Error {
  issues: Issue[];
  constructor(message: string, issues: Issue[] = []) {
    super(message);
    this.issues = issues;
  }
}

export interface Backend {
  appInfo(): Promise<AppInfo>;
  configGet(family: Family, id: string): Promise<JsonObject>;
  configList(family: Family): Promise<string[]>;
  configRaw(family: Family, id: string): Promise<string>;
  configValidate(family: Family, id: string, text: string): Promise<Issue[]>;
  configSaveText(family: Family, id: string, text: string): Promise<void>;
  configSaveValue(family: Family, id: string, value: JsonObject): Promise<void>;
  configDelete(family: Family, id: string): Promise<void>;
  configIssues(): Promise<Issue[]>;
  schemaGet(family: Family): Promise<JSONSchema>;
  openModuleWindow(moduleId: string): Promise<void>;
  /** Subscribes to config events; returns an unsubscribe function. */
  onConfigEvent(cb: (e: ConfigEvent) => void): () => void;
  /** Tells the backend the first screen is rendered (start-up timing). */
  markReady(): void;
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let instance: Backend | null = null;

export async function getBackend(): Promise<Backend> {
  if (instance) return instance;
  if (isTauri()) {
    const { createTauriBackend } = await import("./backend.tauri");
    instance = createTauriBackend();
  } else {
    const { createMockBackend } = await import("./backend.mock");
    instance = createMockBackend();
  }
  return instance;
}

/** For tests: inject a backend. */
export function setBackend(b: Backend | null): void {
  instance = b;
}
