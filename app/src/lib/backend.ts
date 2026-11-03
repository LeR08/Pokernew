/**
 * Single entry point to the Rust core. In the Tauri app it calls the
 * `#[tauri::command]`s of src-tauri; in a plain browser (Vite dev server,
 * Vitest, Playwright) it falls back to an in-memory mock built from
 * config/defaults, so the UI can be developed and tested without Tauri.
 */
import type { JSONSchema } from "./schema";
import type { Breakdown, CalcRequest, EquityResult, FlopFilter, TextureReport } from "./equity";
import type { BatchRow, IcmResult, Review, ReviewRequest, Solution, SolveOptions, Spot } from "./icm";
import type {
  HudStatus,
  OverlayData,
  OpponentsResponse,
  Bankroll,
  GridCell,
  LeaksResponse,
  PlayerNotes,
  Session,
  SessionReview,
  Tx,
  Filter,
  Graph,
  Group,
  Hand,
  HandsResponse,
  Preview,
  ReportResponse,
  StatDef,
  TournamentsResponse,
} from "./tracker";

export type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
export type JsonObject = { [k: string]: JsonValue };

export type Family = "settings" | "hotkeys" | "themes" | "profiles" | "rooms" | "stats" | "leaks" | "notes" | "hud" | "payouts";

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
  /** Writes all errors to a text file; returns its path. */
  importErrorsExport(): Promise<string>;
  /** Shows a file in the system file manager. */
  revealPath(path: string): Promise<void>;
  detectRooms(): Promise<FoundAccount[]>;
  backupList(): Promise<BackupInfo[]>;
  backupNow(): Promise<string>;
  backupRestore(name: string): Promise<void>;
  appRestart(): Promise<void>;
  /** Native file/folder picker; null when cancelled. */
  pickPaths(opts: { directory: boolean; multiple: boolean; title: string }): Promise<string[] | null>;
  onImportEvent(cb: (e: ImportEvent) => void): () => void;

  // ---- Tracker & replayer (phase 2) ----
  trackerReport(filter: Filter, group: Group): Promise<ReportResponse>;
  trackerGraph(filter: Filter): Promise<Graph>;
  trackerTournaments(filter: Filter): Promise<TournamentsResponse>;
  trackerHands(filter: Filter, limit: number, offset: number): Promise<HandsResponse>;
  trackerAccounts(): Promise<string[]>;
  handGet(handId: number): Promise<Hand>;
  statPreview(def: StatDef, filter: Filter): Promise<Preview>;
  trackerSessions(filter: Filter): Promise<Session[]>;
  sessionReview(filter: Filter, start: number, end: number): Promise<SessionReview>;
  trackerLeaks(filter: Filter): Promise<LeaksResponse>;
  trackerGrid(filter: Filter): Promise<GridCell[]>;
  notesGet(room: string, name: string): Promise<PlayerNotes>;
  noteAdd(room: string, name: string, text: string): Promise<number>;
  noteDelete(id: number): Promise<void>;
  playerColour(room: string, name: string, colour: string | null): Promise<void>;
  bankrollGet(): Promise<Bankroll>;
  bankrollTxList(): Promise<Tx[]>;
  bankrollTxAdd(tx: Tx): Promise<number>;
  bankrollTxDelete(id: number): Promise<void>;
  trackerOpponents(filter: Filter, minHands: number, limit: number): Promise<OpponentsResponse>;
  autonotesRun(): Promise<number>;

  // ---- HUD (phase 3) ----
  hudStatus(): Promise<HudStatus>;
  playModeGet(): Promise<boolean>;
  hudSetVisible(visible: boolean): Promise<void>;
  hudSetEdit(edit: boolean): Promise<void>;
  /** Edit one table's HUD on the table itself (null = stop editing). */
  hudSetEditTable(windowId: number | null): Promise<void>;
  hudSetTableHidden(windowId: number, hidden: boolean): Promise<void>;
  hudOverlay(windowId: number): Promise<OverlayData>;
  hudSaveSeat(tableSize: number, slot: number, x: number, y: number): Promise<void>;
  /** "playmode" (bool), "hud-refresh", "hud-edit" (bool), "hud-status". */
  onHudEvent(cb: (kind: "playmode" | "hud-refresh" | "hud-edit" | "hud-status", payload: unknown) => void): () => void;
  /** Writes a CSV file in <data>/exports; returns its path. */
  exportCsv(name: string, rows: string[][]): Promise<string>;

  // ---- Equity Lab (phase 4; refused in play mode) ----
  equityCalc(req: CalcRequest): Promise<EquityResult>;
  equityCancel(): Promise<void>;
  rangeBreakdown(range: string, board: string, dead: string): Promise<Breakdown>;
  /** The combos having one of the classes/draws, as 1 326 weights. */
  rangeFilter(range: string, board: string, dead: string, made: string[], draws: string[]): Promise<number[]>;
  flopTexture(range: string, filter: FlopFilter): Promise<TextureReport>;
  /** Saved ranges, as "folder/sub/name". */
  rangesList(): Promise<string[]>;
  rangeLoad(name: string): Promise<string>;
  rangeSave(name: string, text: string): Promise<void>;
  rangeDelete(name: string): Promise<void>;

  // ---- ICM & push/fold (phase 5; refused in play mode) ----
  icmCalc(stacks: number[], payouts: number[]): Promise<IcmResult>;
  pushfoldSolve(spot: Spot, options: SolveOptions): Promise<Solution>;
  icmReviewHand(handId: number, req: ReviewRequest): Promise<Review>;
  icmReviewTournament(tournamentId: number, req: ReviewRequest): Promise<BatchRow[]>;
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
