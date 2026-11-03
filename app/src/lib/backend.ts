/**
 * Single entry point to the Rust core. In the Tauri app it calls the
 * `#[tauri::command]`s of src-tauri; in a plain browser (Vite dev server,
 * Vitest, Playwright) it falls back to an in-memory mock built from
 * config/defaults, so the UI can be developed and tested without Tauri.
 */
import type { JSONSchema } from "./schema";

export type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
export type JsonObject = { [k: string]: JsonValue };

export type Family = "settings" | "hotkeys" | "themes" | "profiles" | "rooms";

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

export interface DbStats {
  hands: number;
  heroHands: number;
  players: number;
  tournaments: number;
  files: number;
  errors: number;
  firstHandAt: number | null;
  lastHandAt: number | null;
}

export interface ImportReport {
  files_seen: number;
  files_imported: number;
  hands_parsed: number;
  hands_inserted: number;
  duplicates: number;
  errors: number;
  summaries: number;
  elapsed_ms: number;
}

export interface ImportEvent {
  report: ImportReport;
  trigger: "scan" | "watch" | "manual";
}

export interface ImportStatus {
  folders: { room: string; path: string; exists: boolean }[];
  watching: boolean;
  history: { at: number; trigger: ImportEvent["trigger"]; report: ImportReport }[];
  dbPath: string;
}

export interface ImportErrorRow {
  id: number;
  file: string;
  byte_offset: number;
  line_no: number;
  line: string;
  message: string;
  raw: string;
  created_at: number;
}

export interface FoundAccount {
  room: string;
  account: string;
  folder: string;
  files: number;
}

export interface BackupInfo {
  name: string;
  size: number;
  created_at: number;
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

  // ---- Import & database (phase 1) ----
  dbStats(): Promise<DbStats>;
  importStatus(): Promise<ImportStatus>;
  importPaths(paths: string[]): Promise<void>;
  importErrors(limit?: number): Promise<ImportErrorRow[]>;
  importRetryErrors(): Promise<number>;
  detectRooms(): Promise<FoundAccount[]>;
  backupList(): Promise<BackupInfo[]>;
  backupNow(): Promise<string>;
  backupRestore(name: string): Promise<void>;
  appRestart(): Promise<void>;
  /** Native file/folder picker; null when cancelled. */
  pickPaths(opts: { directory: boolean; multiple: boolean; title: string }): Promise<string[] | null>;
  onImportEvent(cb: (e: ImportEvent) => void): () => void;
}

export const isTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

let instance: Promise<Backend> | null = null;

/**
 * The backend singleton. The promise itself is cached so that concurrent
 * first calls share one instance (two instances would not see each other's
 * events).
 */
export function getBackend(): Promise<Backend> {
  instance ??= isTauri()
    ? import("./backend.tauri").then((m) => m.createTauriBackend())
    : import("./backend.mock").then((m) => m.createMockBackend());
  return instance;
}

/** For tests: inject a backend. */
export function setBackend(b: Backend | null): void {
  instance = b ? Promise.resolve(b) : null;
}
