import { useQuery } from "@tanstack/react-query";
import { getBackend } from "@/lib/backend";
import type { Filter, Group } from "@/lib/tracker";
import { toFilter, useTracker } from "@/stores/tracker";

/** The filter for the current filter-bar state. */
export function useFilter(): Filter {
  const period = useTracker((s) => s.period);
  const gameType = useTracker((s) => s.gameType);
  const account = useTracker((s) => s.account);
  return toFilter({ period, gameType, account });
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
