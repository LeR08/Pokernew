/** TanStack Query hooks for the Import & Database module. */
import { useEffect, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { getBackend, type ImportEvent } from "./backend";

export const ik = {
  stats: ["db", "stats"] as const,
  status: ["db", "status"] as const,
  errors: ["db", "errors"] as const,
  backups: ["db", "backups"] as const,
};

export const useDbStats = () =>
  useQuery({ queryKey: ik.stats, queryFn: async () => (await getBackend()).dbStats() });
export const useImportStatus = () =>
  useQuery({ queryKey: ik.status, queryFn: async () => (await getBackend()).importStatus() });
export const useImportErrors = () =>
  useQuery({ queryKey: ik.errors, queryFn: async () => (await getBackend()).importErrors(200) });
export const useBackups = () =>
  useQuery({ queryKey: ik.backups, queryFn: async () => (await getBackend()).backupList() });

/** Refreshes import data on every import event; returns the last event. */
export function useImportEvents(): ImportEvent | null {
  const qc = useQueryClient();
  const [last, setLast] = useState<ImportEvent | null>(null);
  useEffect(() => {
    let off: (() => void) | undefined;
    let alive = true;
    void getBackend().then((b) => {
      if (!alive) return;
      off = b.onImportEvent((e) => {
        setLast(e);
        void qc.invalidateQueries({ queryKey: ["db"] });
      });
    });
    return () => {
      alive = false;
      off?.();
    };
  }, [qc]);
  return last;
}

/** Text to paste into a bug report / parser test case. */
export function testCaseText(e: { file: string; line_no: number; line: string; message: string; raw: string }): string {
  return [
    `# Import error — ${e.file}`,
    `# line ${e.line_no}: ${e.message}`,
    `# offending line: ${e.line}`,
    "",
    e.raw,
    "",
  ].join("\n");
}

export function formatDate(unix: number | null | undefined, lang: string): string {
  if (!unix) return "—";
  return new Date(unix * 1000).toLocaleString(lang === "fr" ? "fr-FR" : "en-GB");
}
