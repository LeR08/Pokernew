import { create } from "zustand";
import type { EquityMode } from "@/lib/equity";

/** Equity Lab inputs, kept while switching tabs. */
interface EquityState {
  players: string[];
  board: string;
  dead: string;
  mode: EquityMode;
  precision: number;
  aRange: string;
  aBoard: string;
  aDead: string;
  aVillain: string;
  tRange: string;
  /** Library folder of "my ranges" (a format, e.g. "mtt-25bb"). */
  chartFolder: string;
  /** Who each player is (tracked player name), when the range was estimated from the tracker. */
  who: (string | null)[];
  set: (p: Partial<Omit<EquityState, "set">>) => void;
}

export const useEquity = create<EquityState>((set) => ({
  players: ["AsKs", "QQ+,AK"],
  board: "",
  dead: "",
  mode: "auto",
  precision: 0.001,
  aRange: "22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+",
  aBoard: "Ah7c2d",
  aDead: "",
  aVillain: "",
  tRange: "22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+",
  chartFolder: "",
  who: [],
  set: (p) => set(p),
}));
