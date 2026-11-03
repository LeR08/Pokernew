/**
 * In-browser stand-in for the Rust backend: same API, files kept in memory,
 * seeded from config/defaults and validated with the same JSON Schemas (Ajv).
 * Used by the Vite dev server, Vitest and Playwright. Never used in the app.
 */
import Ajv2020 from "ajv/dist/2020";
import { parse as parseToml, stringify as stringifyToml } from "smol-toml";
import type {
  Backend,
  BackupInfo,
  ConfigEvent,
  DbStats,
  Family,
  ImportErrorRow,
  ImportEvent,
  ImportStatus,
  Issue,
  JsonObject,
} from "./backend";
import { BackendError } from "./backend";
import type { JSONSchema } from "./schema";
import { mockTracker } from "./backend.mock.tracker";
import type { StatMeta } from "./tracker";

const rawDefaults = import.meta.glob("../../../config/defaults/**/*.toml", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const rawSchemas = import.meta.glob("../../../config/defaults/schemas/*.schema.json", {
  import: "default",
  eager: true,
}) as Record<string, JSONSchema>;

const SCHEMA_OF: Record<Family, string> = {
  settings: "settings",
  hotkeys: "hotkeys",
  themes: "theme",
  profiles: "profile",
  rooms: "room",
  stats: "stats",
};

/** "../../../config/defaults/themes/dark.toml" -> ["themes", "dark"] */
function keyOf(path: string): [Family, string] | null {
  const rel = path.split("config/defaults/")[1];
  if (!rel) return null;
  const parts = rel.replace(/\.toml$/, "").split("/");
  if (parts.length === 1 && (parts[0] === "settings" || parts[0] === "hotkeys")) {
    return [parts[0], parts[0]];
  }
  if (parts.length === 2 && (parts[0] === "themes" || parts[0] === "profiles" || parts[0] === "rooms" || parts[0] === "stats")) {
    return [parts[0], parts[1]];
  }
  return null;
}

export interface MockOptions {
  /** Start as a fresh install (first-run wizard shown). Default: false. */
  firstRun?: boolean;
}

/** In the browser, `?firstrun` in the URL starts the mock as a fresh install. */
function defaultOptions(): MockOptions {
  if (typeof window === "undefined") return {};
  return { firstRun: new URLSearchParams(window.location.search).has("firstrun") };
}

export function createMockBackend(opts: MockOptions = defaultOptions()): Backend {
  const files = new Map<string, string>();
  for (const [path, text] of Object.entries(rawDefaults)) {
    const k = keyOf(path);
    if (k) files.set(`${k[0]}/${k[1]}`, text);
  }
  if (!opts.firstRun) {
    files.set("settings/settings", files.get("settings/settings")!.replace("first_run_done = false", "first_run_done = true"));
  }
  // Simulated database.
  const stats: DbStats = { hands: 0, heroHands: 0, players: 0, tournaments: 0, files: 0, errors: 0, firstHandAt: null, lastHandAt: null };
  const history: ImportStatus["history"] = [];
  const errors: ImportErrorRow[] = [];
  const backups: BackupInfo[] = [];
  const importListeners = new Set<(e: ImportEvent) => void>();
  const fakeImport = (trigger: ImportEvent["trigger"], hands: number) => {
    const report = {
      files_seen: 3,
      files_imported: 3,
      hands_parsed: hands,
      hands_inserted: hands,
      duplicates: 0,
      errors: 0,
      summaries: 2,
      elapsed_ms: 90,
    };
    stats.hands += hands;
    stats.heroHands += hands;
    stats.players += hands ? 41 : 0;
    stats.tournaments += hands ? 3 : 0;
    stats.files += 5;
    stats.firstHandAt ??= 1793474056;
    stats.lastHandAt = 1793648730;
    history.unshift({ at: Math.floor(Date.now() / 1000), trigger, report });
    const e = { report, trigger };
    setTimeout(() => importListeners.forEach((l) => l(e)), 50);
  };
  const ajv = new Ajv2020({ strict: false, allErrors: true });
  const schemas = new Map<string, JSONSchema>();
  for (const [path, schema] of Object.entries(rawSchemas)) {
    const name = path.split("/").pop()!.replace(".schema.json", "");
    schemas.set(name, schema);
  }
  const validators = new Map(
    [...schemas].map(([name, s]) => [name, ajv.compile(s as object)] as const),
  );
  const listeners = new Set<(e: ConfigEvent) => void>();
  const emit = (e: ConfigEvent) => listeners.forEach((l) => l(e));

  function validate(family: Family, id: string, text: string): { value?: JsonObject; issues: Issue[] } {
    let value: JsonObject;
    try {
      value = parseToml(text) as JsonObject;
    } catch (e) {
      return { issues: [{ family, id, pointer: "", message: (e as Error).message }] };
    }
    const v = validators.get(SCHEMA_OF[family])!;
    if (v(value)) return { value, issues: [] };
    const issues = (v.errors ?? []).map((err) => ({
      family,
      id,
      pointer: err.instancePath,
      message: err.message ?? "invalid",
    }));
    return { issues };
  }

  const get = (family: Family, id: string): string => {
    const text = files.get(`${family}/${id}`);
    if (text === undefined) throw new BackendError(`${family}/${id} not found`);
    return text;
  };

  const statsMeta = (): StatMeta[] => {
    const out: StatMeta[] = [];
    for (const [k, text] of files) {
      if (!k.startsWith("stats/")) continue;
      const parsed = parseToml(text) as { stat?: Record<string, Record<string, unknown>> };
      for (const [id, d] of Object.entries(parsed.stat ?? {})) {
        out.push({
          id,
          name: String(d.name),
          format: String(d.format ?? "pct1"),
          description: (d.description as StatMeta["description"]) ?? null,
          colourRanges: (d.colour_ranges as StatMeta["colourRanges"]) ?? [],
          minSample: (d.min_sample as number) ?? null,
          file: k.slice(6),
        });
      }
    }
    return out;
  };

  const backend: Backend = {
    ...mockTracker(statsMeta),
    async appInfo() {
      return {
        version: "0.1.0-dev",
        dataDir: "(browser mock)",
        dataDirSource: "default",
        backendReadyMs: 0,
        mock: true,
      };
    },
    async configGet(family, id) {
      const r = validate(family, id, get(family, id));
      if (!r.value) throw new BackendError("invalid", r.issues);
      return r.value;
    },
    async configList(family) {
      return [...files.keys()]
        .filter((k) => k.startsWith(`${family}/`))
        .map((k) => k.slice(family.length + 1))
        .sort();
    },
    async configRaw(family, id) {
      return get(family, id);
    },
    async configValidate(family, id, text) {
      return validate(family, id, text).issues;
    },
    async configSaveText(family, id, text) {
      const r = validate(family, id, text);
      if (r.issues.length) throw new BackendError("invalid", r.issues);
      files.set(`${family}/${id}`, text);
      emit({ kind: "changed", family, id });
    },
    async configSaveValue(family, id, value) {
      // The mock does not preserve comments (the Rust backend does).
      const text = stringifyToml(value);
      await backend.configSaveText(family, id, text);
    },
    async configDelete(family, id) {
      files.delete(`${family}/${id}`);
      emit({ kind: "removed", family, id });
    },
    async configIssues() {
      return [];
    },
    async schemaGet(family) {
      return schemas.get(SCHEMA_OF[family])!;
    },
    async openModuleWindow(moduleId) {
      window.open(`#/window/${moduleId}`, "_blank");
    },
    onConfigEvent(cb) {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    markReady() {},
    async dbStats() {
      return { ...stats };
    },
    async importStatus() {
      const room = parseToml(files.get("rooms/winamax") ?? "") as { folders?: string[] };
      return {
        folders: (room.folders ?? []).map((path) => ({ room: "winamax", path, exists: true })),
        watching: true,
        history: [...history],
        dbPath: "(browser mock)/database.sqlite",
      };
    },
    async importPaths(paths) {
      for (const p of paths) {
        if (p.includes("bad")) {
          errors.unshift({
            id: errors.length + 1,
            file: p,
            byte_offset: 0,
            line_no: 17,
            line: "Tantaculejen dances",
            message: "unknown action",
            raw: "Winamax Poker - Tournament \"SPACE KO\" …\nTantaculejen dances",
            created_at: Math.floor(Date.now() / 1000),
          });
          stats.errors = errors.length;
        }
      }
      fakeImport("manual", 288);
    },
    async importErrors() {
      return [...errors];
    },
    async importErrorsExport() {
      return "(browser mock)/logs/import-errors.txt";
    },
    async revealPath() {},
    async importRetryErrors() {
      const n = errors.length;
      errors.length = 0;
      stats.errors = 0;
      return n;
    },
    async detectRooms() {
      return [
        {
          room: "winamax",
          account: "Tantaculejen",
          folder: "C:\\Users\\DELL 5590\\AppData\\Roaming\\winamax\\documents\\accounts\\Tantaculejen\\history",
          files: 5,
        },
      ];
    },
    async backupList() {
      return [...backups];
    },
    async backupNow() {
      const name = `database-${new Date().toISOString().replace(/\D/g, "").slice(0, 8)}-${String(backups.length).padStart(6, "0")}.sqlite`;
      backups.unshift({ name, size: 1_234_567, created_at: Math.floor(Date.now() / 1000) });
      return name;
    },
    async backupRestore() {},
    async appRestart() {
      window.location.reload();
    },
    async pickPaths({ title }) {
      const p = window.prompt(title);
      return p ? [p] : null;
    },
    onImportEvent(cb) {
      importListeners.add(cb);
      return () => importListeners.delete(cb);
    },
  };
  // Configuring a room folder triggers the start-up scan, as in the app.
  backend.onConfigEvent((e) => {
    if (e.kind === "changed" && e.family === "rooms") {
      const room = parseToml(files.get(`rooms/${e.id}`) ?? "") as { folders?: string[] };
      if ((room.folders ?? []).length && stats.hands === 0) fakeImport("scan", 288);
    }
  });
  return backend;
}
