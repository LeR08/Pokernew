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
import type { Graph, Hand, HandsResponse, Preview, ReportResponse, TournamentsResponse } from "./tracker";

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
    markReady() {
      void invoke("frontend_ready").catch(() => undefined);
    },
  };
}
