import { useQuery } from "@tanstack/react-query";
import { getBackend } from "@/lib/backend";
import type { Filter, Group } from "@/lib/tracker";
import { toFilter, useTracker } from "@/stores/tracker";

/** The filter for the current filter-bar state. */
export function useFilter(): Filter {
  const period = useTracker((s) => s.period);
  const gameType = useTracker((s) => s.gameType);
  const account = useTracker((s) => s.account);
  const adv = useTracker((s) => s.adv);
  return toFilter({ period, gameType, account, adv });
}

// Keys start with "db" so import events refresh them (see lib/importApi.ts).
export const useReport = (filter: Filter, group: Group) =>
  useQuery({ queryKey: ["db", "report", filter, group], queryFn: async () => (await getBackend()).trackerReport(filter, group) });
export const useGraph = (filter: Filter) =>
  useQuery({ queryKey: ["db", "graph", filter], queryFn: async () => (await getBackend()).trackerGraph(filter) });
export const useTournaments = (filter: Filter) =>
  useQuery({ queryKey: ["db", "tournaments", filter], queryFn: async () => (await getBackend()).trackerTournaments(filter) });
export const useHands = (filter: Filter, limit: number, offset: number) =>
  useQuery({
    queryKey: ["db", "hands", filter, limit, offset],
    queryFn: async () => (await getBackend()).trackerHands(filter, limit, offset),
  });
export const useAccounts = () =>
  useQuery({ queryKey: ["db", "accounts"], queryFn: async () => (await getBackend()).trackerAccounts() });

export const useSessions = (filter: Filter) =>
  useQuery({ queryKey: ["db", "sessions", filter], queryFn: async () => (await getBackend()).trackerSessions(filter) });
export const useSessionReview = (filter: Filter, start: number | null, end: number | null) =>
  useQuery({
    queryKey: ["db", "review", filter, start, end],
    queryFn: async () => (await getBackend()).sessionReview(filter, start!, end!),
    enabled: start !== null && end !== null,
  });
export const useLeaks = (filter: Filter) =>
  useQuery({ queryKey: ["db", "leaks", filter], queryFn: async () => (await getBackend()).trackerLeaks(filter) });
export const useGrid = (filter: Filter) =>
  useQuery({ queryKey: ["db", "grid", filter], queryFn: async () => (await getBackend()).trackerGrid(filter) });

/** Writes rows to a CSV file and shows it in the file manager. */
export async function exportCsv(name: string, rows: string[][]): Promise<string> {
  const b = await getBackend();
  const path = await b.exportCsv(name, rows);
  await b.revealPath(path).catch(() => undefined);
  return path;
}

export const useOpponents = (filter: Filter, minHands: number) =>
  useQuery({
    queryKey: ["db", "opponents", filter, minHands],
    queryFn: async () => (await getBackend()).trackerOpponents(filter, minHands, 500),
  });
