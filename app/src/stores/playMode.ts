import { useEffect } from "react";
import { create } from "zustand";
import { getBackend } from "@/lib/backend";

/**
 * Play mode (fair-play): true while a poker table is open, and 60 s after the
 * last one closed. Study modules are locked. Driven by the backend; the UI
 * cannot turn it off.
 */
export const usePlayMode = create<{ on: boolean; set: (v: boolean) => void }>((set) => ({
  on: false,
  set: (on) => set({ on }),
}));

export function usePlayModeSync(): void {
  const set = usePlayMode((s) => s.set);
  useEffect(() => {
    let off: (() => void) | undefined;
    let alive = true;
    void getBackend().then(async (b) => {
      const v = await b.playModeGet().catch(() => false);
      if (!alive) return;
      set(v);
      off = b.onHudEvent((k, p) => {
        if (k === "playmode") set(Boolean(p));
      });
    });
    return () => {
      alive = false;
      off?.();
    };
  }, [set]);
}
