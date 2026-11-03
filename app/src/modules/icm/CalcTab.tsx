/** ICM calculator: $EV, chip EV, bubble factors, risk premiums. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import type { IcmResult } from "@/lib/icm";
import { parsePlaces } from "./payouts";
import { PayoutPicker } from "./PayoutPicker";
import { useIcm } from "./store";

const money = (x: number) => `${x.toFixed(2)} €`;

export function CalcTab() {
  const { t } = useTranslation();
  const s = useIcm();
  const [res, setRes] = useState<IcmResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const payouts = parsePlaces(s.calcPayouts);

  useEffect(() => {
    if (!payouts || s.calcStacks.some((x) => !(x >= 0))) return;
    const h = setTimeout(() => {
      void getBackend()
        .then((b) => b.icmCalc(s.calcStacks, payouts))
        .then((r) => {
          setRes(r);
          setError(null);
        })
        .catch((e: Error) => setError(e.message));
    }, 120);
    return () => clearTimeout(h);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [s.calcStacks, s.calcPayouts]);

  const total = s.calcStacks.reduce((a, b) => a + b, 0);
  const setStack = (i: number, v: number) => s.set({ calcStacks: s.calcStacks.map((x, j) => (j === i ? v : x)) });
  const maxBf = res ? Math.max(1, ...res.bubble.flatMap((r) => r.filter((x) => x < 999))) : 1;

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <PayoutPicker value={s.calcPayouts} onChange={(v) => s.set({ calcPayouts: v })} testId="calc-payouts" />
        <table className="w-full text-sm" data-testid="icm-table">
          <thead className="text-left text-xs text-muted">
            <tr>
              <th className="py-1">{t("icm.player")}</th>
              <th>{t("icm.stack")}</th>
              <th className="text-right">{t("icm.chips_pct")}</th>
              <th className="text-right">{t("icm.icm_ev")}</th>
              <th className="text-right">{t("icm.chip_ev")}</th>
              <th className="text-right">{t("icm.diff")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {s.calcStacks.map((x, i) => (
              <tr key={i} className="border-t border-border" data-testid="icm-row">
                <td className="py-1">{t("icm.player_n", { n: i + 1 })}</td>
                <td>
                  <Input type="number" min={0} value={x} onChange={(e) => setStack(i, Number(e.target.value))} className="w-32" data-testid="icm-stack" aria-label={t("icm.stack")} />
                </td>
                <td className="text-right text-muted">{total ? ((x / total) * 100).toFixed(1) : "0"} %</td>
                <td className="text-right font-semibold" data-testid="icm-ev">
                  {res ? money(res.icm[i] ?? 0) : "—"}
                </td>
                <td className="text-right">{res ? money(res.chipEv[i] ?? 0) : "—"}</td>
                <td className="text-right text-muted">{res ? money((res.icm[i] ?? 0) - (res.chipEv[i] ?? 0)) : "—"}</td>
                <td>
                  <Button size="icon" variant="ghost" aria-label={t("icm.remove_player")} disabled={s.calcStacks.length <= 2} onClick={() => s.set({ calcStacks: s.calcStacks.filter((_, j) => j !== i) })}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </td>
              </tr>
            ))}
          </tbody>
          {res && (
            <tfoot>
              <tr className="border-t border-border text-sm">
                <td className="py-1" colSpan={3}>
                  {t("icm.total")}
                </td>
                <td className="text-right font-semibold" data-testid="icm-total">
                  {money(res.icm.reduce((a, b) => a + b, 0))}
                </td>
                <td className="text-right">{money(res.prizePool)}</td>
                <td colSpan={2} />
              </tr>
            </tfoot>
          )}
        </table>
        <div>
          <Button size="sm" variant="outline" disabled={s.calcStacks.length >= 16} onClick={() => s.set({ calcStacks: [...s.calcStacks, 1000] })} data-testid="icm-add">
            <Plus className="h-4 w-4" />
            {t("icm.add_player")}
          </Button>
        </div>
        {!payouts && <p className="text-sm text-danger">{t("icm.bad_prizes")}</p>}
        {error && <p className="text-sm text-danger">{error}</p>}
      </Card>
      {res && (
        <Card className="flex flex-col gap-d2 overflow-x-auto p-d4">
          <h2 className="font-semibold">{t("icm.bubble_title")}</h2>
          <p className="text-xs text-muted">{t("icm.bubble_help")}</p>
          <table className="text-sm" data-testid="bubble-matrix">
            <thead>
              <tr className="text-xs text-muted">
                <th className="px-2 text-left">{t("icm.vs")}</th>
                {s.calcStacks.map((_, j) => (
                  <th key={j} className="px-2">
                    {j + 1}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {res.bubble.map((row, i) => (
                <tr key={i}>
                  <td className="px-2 text-xs text-muted">{t("icm.player_n", { n: i + 1 })}</td>
                  {row.map((b, j) => (
                    <td
                      key={j}
                      className="px-2 py-1 text-center tabular-nums"
                      title={i === j ? "" : t("icm.risk_tip", { p: (res.risk[i][j] * 100).toFixed(1) })}
                      style={i === j ? undefined : { background: `color-mix(in oklab, #2a78d6 ${Math.round(((Math.min(b, maxBf) - 1) / Math.max(0.01, maxBf - 1)) * 60)}%, transparent)` }}
                    >
                      {i === j ? "—" : b >= 999 ? "∞" : b.toFixed(2)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
    </div>
  );
}
