/**
 * In-browser stand-in for the Rust backend: same API, files kept in memory,
 * seeded from config/defaults and validated with the same JSON Schemas (Ajv).
 * Used by the Vite dev server, Vitest and Playwright. Never used in the app.
 */
import Ajv2020 from "ajv/dist/2020";
import { parse as parseToml, stringify as stringifyToml } from "smol-toml";
import type { Backend, ConfigEvent, Family, Issue, JsonObject } from "./backend";
import { BackendError } from "./backend";
import type { JSONSchema } from "./schema";

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
};

/** "../../../config/defaults/themes/dark.toml" -> ["themes", "dark"] */
function keyOf(path: string): [Family, string] | null {
  const rel = path.split("config/defaults/")[1];
  if (!rel) return null;
  const parts = rel.replace(/\.toml$/, "").split("/");
  if (parts.length === 1 && (parts[0] === "settings" || parts[0] === "hotkeys")) {
    return [parts[0], parts[0]];
  }
  if (parts.length === 2 && (parts[0] === "themes" || parts[0] === "profiles")) {
    return [parts[0], parts[1]];
  }
  return null;
}

export function createMockBackend(): Backend {
  const files = new Map<string, string>();
  for (const [path, text] of Object.entries(rawDefaults)) {
    const k = keyOf(path);
    if (k) files.set(`${k[0]}/${k[1]}`, text);
  }
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

  const backend: Backend = {
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
  };
  return backend;
}
