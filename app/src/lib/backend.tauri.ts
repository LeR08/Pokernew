import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  Backend,
  BackupInfo,
  ConfigEvent,
  DbStats,
  FoundAccount,
  ImportErrorRow,
  ImportEvent,
  ImportStatus,
  Issue,
  JsonObject,
} from "./backend";
import { BackendError } from "./backend";
import type { JSONSchema } from "./schema";
import type { Breakdown, EquityResult, TextureReport } from "./equity";
import type { BatchRow, IcmResult, Review, Solution } from "./icm";
import type {
  HudStatus,
  OverlayData,
  OpponentsResponse,
  Bankroll,
  Graph,
  GridCell,
  Hand,
  HandsResponse,
  LeaksResponse,
  PlayerNotes,
  Preview,
  ReportResponse,
  Session,
  SessionReview,
  TournamentsResponse,
  Tx,
} from "./tracker";

/** Shape of errors returned by src-tauri commands. */
interface CommandError {
  message: string;
  issues: Issue[];
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const err = e as Partial<CommandError> | string;
    if (typeof err === "string") throw new BackendError(err);
    throw new BackendError(err.message ?? String(e), err.issues ?? []);
  }
}

function subscribe<T>(channel: string, cb: (payload: T) => void): () => void {
  let unlisten: (() => void) | null = null;
  let cancelled = false;
  void listen<T>(channel, (e) => cb(e.payload)).then((u) => {
    if (cancelled) u();
    else unlisten = u;
  });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

export function createTauriBackend(): Backend {
  return {
    appInfo: () => call("app_info"),
    configGet: (family, id) => call<JsonObject>("config_get", { family, id }),
    configList: (family) => call<string[]>("config_list", { family }),
    configRaw: (family, id) => call<string>("config_raw", { family, id }),
    configValidate: (family, id, text) => call<Issue[]>("config_validate", { family, id, text }),
    configSaveText: (family, id, text) => call("config_save_text", { family, id, text }),
    configSaveValue: (family, id, value) => call("config_save_value", { family, id, value }),
    configDelete: (family, id) => call("config_delete", { family, id }),
    configIssues: () => call<Issue[]>("config_issues"),
    schemaGet: (family) => call<JSONSchema>("schema_get", { family }),
    openModuleWindow: (moduleId) => call("open_module_window", { moduleId }),
    onConfigEvent: (cb) => subscribe<ConfigEvent>("config://event", cb),
    onImportEvent: (cb) => subscribe<ImportEvent>("import://event", cb),
    dbStats: () => call<DbStats>("db_stats"),
    importStatus: () => call<ImportStatus>("import_status"),
    importPaths: (paths) => call("import_paths", { paths }),
    importErrors: (limit) => call<ImportErrorRow[]>("import_errors", { limit }),
    importRetryErrors: () => call<number>("import_retry_errors"),
    importErrorsExport: () => call<string>("import_errors_export"),
    async revealPath(path) {
      const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
      await revealItemInDir(path);
    },
    detectRooms: () => call<FoundAccount[]>("detect_rooms"),
    backupList: () => call<BackupInfo[]>("backup_list"),
    backupNow: () => call<string>("backup_now"),
    backupRestore: (name) => call("backup_restore", { name }),
    appRestart: () => call("app_restart"),
    async pickPaths({ directory, multiple, title }) {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const r = await open({ directory, multiple, title });
      if (r === null) return null;
      return Array.isArray(r) ? r : [r];
    },
    trackerReport: (filter, group) => call<ReportResponse>("tracker_report", { filter, group }),
    trackerGraph: (filter) => call<Graph>("tracker_graph", { filter }),
    trackerTournaments: (filter) => call<TournamentsResponse>("tracker_tournaments", { filter }),
    trackerHands: (filter, limit, offset) => call<HandsResponse>("tracker_hands", { filter, limit, offset }),
    trackerAccounts: () => call<string[]>("tracker_accounts"),
    handGet: (handId) => call<Hand>("hand_get", { handId }),
    statPreview: (def, filter) => call<Preview>("stat_preview", { def, filter }),
    trackerSessions: (filter) => call<Session[]>("tracker_sessions", { filter }),
    sessionReview: (filter, start, end) => call<SessionReview>("session_review", { filter, start, end }),
    trackerLeaks: (filter) => call<LeaksResponse>("tracker_leaks", { filter }),
    trackerGrid: (filter) => call<GridCell[]>("tracker_grid", { filter }),
    notesGet: (room, name) => call<PlayerNotes>("notes_get", { room, name }),
    noteAdd: (room, name, text) => call<number>("note_add", { room, name, text }),
    noteDelete: (id) => call("note_delete", { id }),
    playerColour: (room, name, colour) => call("player_colour", { room, name, colour }),
    bankrollGet: () => call<Bankroll>("bankroll_get"),
    bankrollTxList: () => call<Tx[]>("bankroll_tx_list"),
    bankrollTxAdd: (tx) => call<number>("bankroll_tx_add", { tx: { ...tx, id: tx.id ?? 0 } }),
    bankrollTxDelete: (id) => call("bankroll_tx_delete", { id }),
    exportCsv: (name, rows) => call<string>("export_csv", { name, rows }),
    trackerOpponents: (filter, minHands, limit) => call<OpponentsResponse>("tracker_opponents", { filter, minHands, limit }),
    autonotesRun: () => call<number>("autonotes_run"),
    hudStatus: () => call<HudStatus>("hud_status"),
    playModeGet: () => call<boolean>("play_mode_get"),
    hudSetVisible: (visible) => call("hud_set_visible", { visible }),
    hudSetEdit: (edit) => call("hud_set_edit", { edit }),
    hudSetEditTable: (windowId) => call("hud_set_edit_table", { windowId }),
    hudSetTableHidden: (windowId, hidden) => call("hud_set_table_hidden", { windowId, hidden }),
    hudOverlay: (windowId) => call<OverlayData>("hud_overlay", { windowId }),
    hudSaveSeat: (tableSize, slot, x, y) => call("hud_save_seat", { tableSize, slot, x, y }),
    onHudEvent(cb) {
      const offs = [
        subscribe<boolean>("playmode://changed", (p) => cb("playmode", p)),
        subscribe<null>("hud://refresh", (p) => cb("hud-refresh", p)),
        subscribe<boolean>("hud://edit", (p) => cb("hud-edit", p)),
        subscribe<null>("hud://status", (p) => cb("hud-status", p)),
      ];
      return () => offs.forEach((o) => o());
    },
    equityCalc: (req) => call<EquityResult>("equity_calc", { req }),
    equityCancel: () => call("equity_cancel"),
    rangeBreakdown: (rangeText, board, dead) => call<Breakdown>("range_breakdown", { rangeText, board, dead }),
    rangeFilter: (rangeText, board, dead, made, draws) => call<number[]>("range_filter", { rangeText, board, dead, made, draws }),
    flopTexture: (rangeText, filter) => call<TextureReport>("flop_texture", { rangeText, filter }),
    rangesList: () => call<string[]>("ranges_list"),
    rangeLoad: (name) => call<string>("range_load", { name }),
    rangeSave: (name, text) => call("range_save", { name, text }),
    rangeDelete: (name) => call("range_delete", { name }),
    icmCalc: (stacks, payouts) => call<IcmResult>("icm_calc", { stacks, payouts }),
    pushfoldSolve: (spot, options) =>
      call<Solution>("pushfold_solve", { spot, options: { maxIterations: 0, target: 0, locks: [], ...options } }),
    icmReviewHand: (handId, req) => call<Review>("icm_review_hand", { handId, req }),
    icmReviewTournament: (tournamentId, req) => call<BatchRow[]>("icm_review_tournament", { tournamentId, req }),
    markReady() {
      void invoke("frontend_ready").catch(() => undefined);
    },
  };
}
