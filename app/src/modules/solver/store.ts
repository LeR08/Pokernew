import { create } from "zustand";
import { DEFAULT_PARAMS, DEFAULT_TREE, type Lock, type SolveParams, type SpotConfig } from "@/lib/solver";

interface SolverUi {
  spot: SpotConfig;
  params: SolveParams;
  name: string;
  /** Locks to apply on the next solve (from the viewer). */
  locks: Lock[];
  set: (p: Partial<Omit<SolverUi, "set">>) => void;
}

export const useSolverUi = create<SolverUi>((set) => ({
  spot: {
    oopRange: "66+,A8s+,A5s-A4s,AJo+,K9s+,KQo,QTs+,JTs,96s+,85s+,75s+,65s,54s",
    ipRange: "QQ-22,AQs-A2s,ATo+,K5s+,KJo+,Q8s+,J8s+,T7s+,96s+,86s+,75s+,64s+,53s+",
    board: "QsJh2h",
    pot: 5.5,
    stack: 97.5,
    rake: 0,
    rakeCap: 0,
    tree: DEFAULT_TREE,
  },
  params: DEFAULT_PARAMS,
  name: "",
  locks: [],
  set: (p) => set(p),
}));
