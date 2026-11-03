import { useCallback, useEffect, useState } from "react";
import { getBackend } from "@/lib/backend";
import type { Payouts } from "@/lib/icm";

/** Saved payout structures (payouts/*.toml), reloaded when the files change. */
export function usePayouts(): { list: { id: string; p: Payouts }[]; reload: () => void } {
  const [list, setList] = useState<{ id: string; p: Payouts }[]>([]);
  const reload = useCallback(() => {
    void getBackend().then(async (b) => {
      const ids = await b.configList("payouts").catch(() => [] as string[]);
      const out: { id: string; p: Payouts }[] = [];
      for (const id of ids) {
        const v = await b.configGet("payouts", id).catch(() => null);
        if (v) out.push({ id, p: v as unknown as Payouts });
      }
      setList(out);
    });
  }, []);
  useEffect(() => {
    reload();
    let off: (() => void) | undefined;
    void getBackend().then((b) => {
      off = b.onConfigEvent((e) => {
        if (e.family === "payouts") reload();
      });
    });
    return () => off?.();
  }, [reload]);
  return { list, reload };
}

/** File id from a name: "Space KO 1164" → "space-ko-1164". */
export const slug = (s: string) =>
  s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "") || "structure";

/** "1;150\n2;100" or "150, 100" → [150, 100] (the place column is optional). */
export function parsePlaces(text: string): number[] | null {
  const rows = text
    .split(/\n/)
    .map((l) => l.trim())
    .filter(Boolean);
  if (rows.length === 1 && !/[;\t]/.test(rows[0])) {
    const xs = rows[0].split(/[,\s]+/).filter(Boolean).map((x) => Number(x.replace(",", ".")));
    return xs.length && xs.every((x) => Number.isFinite(x) && x >= 0) ? xs : null;
  }
  const out: number[] = [];
  for (const r of rows) {
    const cells = r.split(/[;\t]/).map((c) => c.trim());
    const v = Number((cells.length > 1 ? cells[1] : cells[0]).replace(/[€%\s]/g, "").replace(",", "."));
    if (!Number.isFinite(v) || v < 0) {
      if (out.length === 0) continue; // header line
      return null;
    }
    out.push(v);
  }
  return out.length ? out : null;
}
