import { useMemo } from "react";
import { getBackend } from "@/lib/backend";
import { useConfig } from "@/lib/config";
import type { Settings } from "@/lib/config";
import { DEFAULT_THRESHOLDS, type Grade, type HistoryEntry, type Thresholds } from "@/lib/trainer";

/** Grade thresholds from Settings > Modules > Trainer (settings.toml [module_settings.trainer]). */
export function useThresholds(): Thresholds {
  const s = useConfig<Settings>("settings", "settings").data as Settings | undefined;
  return useMemo(() => {
    const m = (s?.module_settings?.trainer ?? {}) as Partial<Thresholds>;
    return { ...DEFAULT_THRESHOLDS, ...m };
  }, [s]);
}

export function record(e: Omit<HistoryEntry, "at">) {
  void getBackend().then((b) => b.trainerRecord({ ...e, at: Math.floor(Date.now() / 1000) }));
}

export const GRADE_CLASS: Record<Grade, string> = {
  best: "text-success",
  correct: "text-success",
  inaccuracy: "text-warning",
  mistake: "text-danger",
  blunder: "text-danger",
};
