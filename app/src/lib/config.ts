/**
 * TanStack Query hooks over the backend config API. Queries are invalidated
 * by backend config events, so a TOML edited in any text editor shows up in
 * the UI without a restart (hot reload).
 */
import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { getBackend, type ConfigEvent, type Family, type JsonObject } from "./backend";

export const qk = {
  config: (f: Family, id: string) => ["config", f, id] as const,
  raw: (f: Family, id: string) => ["raw", f, id] as const,
  list: (f: Family) => ["list", f] as const,
  schema: (f: Family) => ["schema", f] as const,
  issues: ["issues"] as const,
  appInfo: ["appInfo"] as const,
};

export function useConfig<T = JsonObject>(family: Family, id: string, enabled = true) {
  return useQuery({
    queryKey: qk.config(family, id),
    queryFn: async () => (await (await getBackend()).configGet(family, id)) as T,
    enabled: enabled && !!id,
    staleTime: Infinity,
  });
}

export function useRaw(family: Family, id: string) {
  return useQuery({
    queryKey: qk.raw(family, id),
    queryFn: async () => (await getBackend()).configRaw(family, id),
    enabled: !!id,
    staleTime: Infinity,
  });
}

export function useConfigList(family: Family) {
  return useQuery({
    queryKey: qk.list(family),
    queryFn: async () => (await getBackend()).configList(family),
    staleTime: Infinity,
  });
}

export function useSchema(family: Family) {
  return useQuery({
    queryKey: qk.schema(family),
    queryFn: async () => (await getBackend()).schemaGet(family),
    staleTime: Infinity,
  });
}

export function useIssues() {
  return useQuery({
    queryKey: qk.issues,
    queryFn: async () => (await getBackend()).configIssues(),
    staleTime: Infinity,
  });
}

export function useAppInfo() {
  return useQuery({
    queryKey: qk.appInfo,
    queryFn: async () => (await getBackend()).appInfo(),
    staleTime: Infinity,
  });
}

export function invalidateFor(qc: QueryClient, e: ConfigEvent): void {
  const f = e.family as Family;
  void qc.invalidateQueries({ queryKey: qk.issues });
  if (e.kind === "invalid") return;
  void qc.invalidateQueries({ queryKey: qk.config(f, e.id) });
  void qc.invalidateQueries({ queryKey: qk.raw(f, e.id) });
  void qc.invalidateQueries({ queryKey: qk.list(f) });
}

/** Subscribes the query cache to backend config events (mount once). */
export function useConfigEvents(onEvent?: (e: ConfigEvent) => void): void {
  const qc = useQueryClient();
  useEffect(() => {
    let off: (() => void) | undefined;
    let alive = true;
    void getBackend().then((b) => {
      if (!alive) return;
      off = b.onConfigEvent((e) => {
        invalidateFor(qc, e);
        onEvent?.(e);
      });
    });
    return () => {
      alive = false;
      off?.();
    };
  }, [qc, onEvent]);
}

export function useSaveValue(family: Family, id: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (value: JsonObject) => (await getBackend()).configSaveValue(family, id, value),
    onSuccess: () => invalidateFor(qc, { kind: "changed", family, id }),
  });
}

export function useSaveText(family: Family) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, text }: { id: string; text: string }) =>
      (await getBackend()).configSaveText(family, id, text),
    onSuccess: (_d, v) => invalidateFor(qc, { kind: "changed", family, id: v.id }),
  });
}

/** Typed view of settings.toml. */
export interface Settings {
  general: { language: "fr" | "en"; profile: string; first_run_done?: boolean };
  appearance: { theme: string; density: "compact" | "comfortable"; font_scale: number };
  modules: { order: string[]; disabled: string[] };
  performance: { worker_threads: number; ram_cap_mb: number };
  backup?: { enabled: boolean; every_hours: number; keep: number };
  module_settings?: Record<string, Record<string, unknown>>;
}

export interface Theme {
  name: string;
  base: "light" | "dark";
  colors: Record<string, string>;
}

export interface Profile {
  name: string;
  description?: string;
  theme?: string;
}

export interface Hotkeys {
  actions: Record<string, string>;
}

export const useSettings = () => useConfig<Settings>("settings", "settings");
