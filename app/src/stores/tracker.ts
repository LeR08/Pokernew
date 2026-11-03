import { create } from "zustand";
import type { Filter } from "@/lib/tracker";

export type Period = "all" | "7d" | "30d" | "90d" | "year";

interface TrackerState {
  period: Period;
  gameType: "" | "cash" | "mtt" | "spin";
  account: string;
  /** false = big blinds / chips, true = euros. */
  euros: boolean;
  setPeriod: (p: Period) => void;
  setGameType: (g: TrackerState["gameType"]) => void;
  setAccount: (a: string) => void;
  setEuros: (e: boolean) => void;
}

const KEY = "lps.tracker.v1";

function load(): Partial<TrackerState> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "{}") as Partial<TrackerState>;
  } catch {
    return {};
  }
}

function save(s: TrackerState) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ period: s.period, gameType: s.gameType, account: s.account, euros: s.euros }));
  } catch {
    /* private mode: keep in memory only */
  }
}

export const useTracker = create<TrackerState>((set, get) => {
  const saved = load();
  const upd = (p: Partial<TrackerState>) => {
    set(p);
    save(get());
  };
  return {
    period: saved.period ?? "all",
    gameType: saved.gameType ?? "",
    account: saved.account ?? "",
    euros: saved.euros ?? false,
    setPeriod: (period) => upd({ period }),
    setGameType: (gameType) => upd({ gameType }),
    setAccount: (account) => upd({ account }),
    setEuros: (euros) => upd({ euros }),
  };
});

const DAY = 86_400;

/** The backend filter for the current UI state. `now` is injectable for tests. */
export function toFilter(s: Pick<TrackerState, "period" | "gameType" | "account">, now = Date.now() / 1000): Filter {
  const f: Filter = {};
  const since: Record<Exclude<Period, "all" | "year">, number> = { "7d": 7, "30d": 30, "90d": 90 };
  if (s.period === "year") f.from = Math.floor(new Date(new Date(now * 1000).getFullYear(), 0, 1).getTime() / 1000);
  else if (s.period !== "all") f.from = Math.floor(now - since[s.period] * DAY);
  if (s.gameType) f.gameTypes = [s.gameType];
  if (s.account) f.accounts = [s.account];
  return f;
}
