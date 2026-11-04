import { create } from "zustand";
import type { Filter } from "@/lib/tracker";

/** "More filters" panel (not persisted: starts empty on each launch). */
export interface Advanced {
  who: "hero" | "opponents";
  handClasses: string[];
  potTypes: string[];
  stackMin: number | null;
  stackMax: number | null;
  street: "" | "flop" | "turn" | "river";
  texture: number;
  showdown: "" | "yes" | "no";
  allinPreflop: "" | "yes" | "no";
  buyInMin: number | null;
  buyInMax: number | null;
  withPlayer: string;
  result: "" | "won" | "lost";
  minBb: number | null;
}

export const EMPTY_ADVANCED: Advanced = {
  who: "hero",
  handClasses: [],
  potTypes: [],
  stackMin: null,
  stackMax: null,
  street: "",
  texture: 0,
  showdown: "",
  allinPreflop: "",
  buyInMin: null,
  buyInMax: null,
  withPlayer: "",
  result: "",
  minBb: null,
};

/** Number of active advanced filters (for the badge). */
export function activeCount(a: Advanced): number {
  return [
    a.who !== "hero",
    a.handClasses.length > 0,
    a.potTypes.length > 0,
    a.stackMin !== null || a.stackMax !== null,
    a.street !== "",
    a.texture !== 0,
    a.showdown !== "",
    a.allinPreflop !== "",
    a.buyInMin !== null || a.buyInMax !== null,
    a.withPlayer.trim() !== "",
    a.result !== "",
    a.minBb !== null,
  ].filter(Boolean).length;
}

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
  adv: Advanced;
  setAdv: (patch: Partial<Advanced>) => void;
  resetAdv: () => void;
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
    adv: EMPTY_ADVANCED,
    setAdv: (patch) => set((st) => ({ adv: { ...st.adv, ...patch } })),
    resetAdv: () => set({ adv: EMPTY_ADVANCED }),
  };
});

const DAY = 86_400;

/** The backend filter for the current UI state. `now` is injectable for tests. */
export function toFilter(
  s: Pick<TrackerState, "period" | "gameType" | "account"> & { adv?: Advanced },
  now = Date.now() / 1000,
): Filter {
  const f: Filter = {};
  const since: Record<Exclude<Period, "all" | "year">, number> = { "7d": 7, "30d": 30, "90d": 90 };
  if (s.period === "year") f.from = Math.floor(new Date(new Date(now * 1000).getFullYear(), 0, 1).getTime() / 1000);
  else if (s.period !== "all") f.from = Math.floor(now - since[s.period] * DAY);
  if (s.gameType) f.gameTypes = [s.gameType];
  if (s.account) f.accounts = [s.account];
  const a = s.adv;
  if (a) {
    if (a.who === "opponents") f.who = "opponents";
    if (a.handClasses.length) f.handClasses = a.handClasses;
    if (a.potTypes.length) f.potTypes = a.potTypes;
    if (a.stackMin !== null) f.stackMin = a.stackMin;
    if (a.stackMax !== null) f.stackMax = a.stackMax;
    if (a.street) f.street = a.street;
    if (a.texture) f.texture = a.texture;
    if (a.showdown) f.showdown = a.showdown === "yes";
    if (a.allinPreflop) f.allinPreflop = a.allinPreflop === "yes";
    if (a.buyInMin !== null) f.buyInMin = Math.round(a.buyInMin * 100);
    if (a.buyInMax !== null) f.buyInMax = Math.round(a.buyInMax * 100);
    if (a.withPlayer.trim()) f.withPlayer = a.withPlayer.trim();
    if (a.result) f.result = a.result;
    if (a.minBb !== null) f.minBb = a.minBb;
  }
  return f;
}
