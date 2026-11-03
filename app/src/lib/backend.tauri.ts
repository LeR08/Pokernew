import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Backend, ConfigEvent, Issue, JsonObject } from "./backend";
import { BackendError } from "./backend";
import type { JSONSchema } from "./schema";

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
    onConfigEvent(cb) {
      let unlisten: (() => void) | null = null;
      let cancelled = false;
      void listen<ConfigEvent>("config://event", (e) => cb(e.payload)).then((u) => {
        if (cancelled) u();
        else unlisten = u;
      });
      return () => {
        cancelled = true;
        unlisten?.();
      };
    },
    markReady() {
      void invoke("frontend_ready").catch(() => undefined);
    },
  };
}
