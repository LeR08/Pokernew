import { create } from "zustand";
import type { Decision } from "@/lib/icm";

interface IcmState {
  calcStacks: number[];
  calcPayouts: string;
  pfStacks: number[];
  pfAnte: number;
  pfBbAnte: boolean;
  pfModel: "chips" | "icm";
  pfPayouts: string;
  pfOthers: string;
  pfFgs: number;
  pfBounties: number[];
  pfLocks: [Decision, number[], string][];
  set: (p: Partial<Omit<IcmState, "set">>) => void;
}

export const useIcm = create<IcmState>((set) => ({
  calcStacks: [5000, 3000, 2000],
  calcPayouts: "50, 30, 20",
  pfStacks: [10, 10],
  pfAnte: 0,
  pfBbAnte: false,
  pfModel: "chips",
  pfPayouts: "50, 30, 20",
  pfOthers: "",
  pfFgs: 0,
  pfBounties: [],
  pfLocks: [],
  set: (p) => set(p),
}));
