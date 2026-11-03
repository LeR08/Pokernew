/** Push/fold equilibrium for 2 to 10 players, chip EV or ICM, with "what if" sliders and locks. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Lock, Trash2 } from "lucide-react";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { decisionKey, positionNames, type Decision, type Solution, type Spot } from "@/lib/icm";
import { CELL_COMBOS, tryParseRange } from "@/lib/range";
import { Modal } from "../equity/Modal";
import { ClassGrid, classFreq } from "./ClassGrid";
import { parsePlaces } from "./payouts";
import { PayoutPicker } from "./PayoutPicker";
import { useIcm } from "./store";

export function buildSpot(s: ReturnType<typeof useIcm.getState>): Spot | null {
  const icm = s.pfModel === "icm";
  const payouts = icm ? parsePlaces(s.pfPayouts) : [];
  const others = icm && s.pfOthers.trim() ? parsePlaces(s.pfOthers) : [];
  if (payouts === null || others === null) return null;
  return {
    stacks: s.pfStacks,
    sb: 0.5,
    bb: 1,
    ante: s.pfAnte,
    bbAnte: s.pfBbAnte,
    others,
    payouts,
    bounties: icm && s.pfBounties.some((b) => b > 0) ? s.pfStacks.map((_, i) => s.pfBounties[i] ?? 0) : [],
    fgsDepth: icm ? s.pfFgs : 0,
  };
}

export function PushFoldTab() {
  const { t } = useTranslation();
  const s = useIcm();
  const [sol, setSol] = useState<Solution | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sel, setSel] = useState<string>("push-0");
  const [locking, setLocking] = useState<Decision | null>(null);
  const [lockText, setLockText] = useState("");
  const n = s.pfStacks.length;
  const pos = positionNames(n);
  const spot = useMemo(() => buildSpot(s), [s]);

  useEffect(() => {
    if (!spot) return;
    setBusy(true);
    const h = setTimeout(() => {
      void getBackend()
        .then((b) => b.pushfoldSolve(spot, { locks: s.pfLocks.map(([d, f]) => [d, f]) }))
        .then((r) => {
          setSol(r);
          setError(null);
        })
        .catch((e: Error) => setError(e.message))
        .finally(() => setBusy(false));
    }, 350);
    return () => clearTimeout(h);
  }, [spot, s.pfLocks]);

  const label = (d: Decision) =>
    d.kind === "push" ? t("icm.pf.push_node", { pos: pos[d.p] }) : t("icm.pf.call_node", { pos: pos[d.q], vs: pos[d.p] });
  const node = sol?.nodes.find((x) => decisionKey(x.decision) === sel) ?? sol?.nodes[0];
  const fmt = (x: number) => (sol?.unit === "money" ? `${x.toFixed(3)} €` : `${x.toFixed(2)} bb`);
  const share = node ? node.freq.reduce((a, f, c) => a + f * CELL_COMBOS[c].length, 0) / 1326 : 0;
  const setStack = (i: number, v: number) => s.set({ pfStacks: s.pfStacks.map((x, j) => (j === i ? v : x)) });
  const resize = (m: number) => {
    const st = Array.from({ length: m }, (_, i) => s.pfStacks[i] ?? 10);
    s.set({ pfStacks: st, pfLocks: [], pfBounties: [] });
    setSel("push-0");
  };

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex flex-col gap-1">
            {t("icm.pf.players")}
            <Select className="w-24" value={n} onChange={(e) => resize(Number(e.target.value))} data-testid="pf-players">
              {[2, 3, 4, 5, 6, 7, 8, 9, 10].map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            {t("icm.pf.ante")}
            <Input type="number" min={0} step={0.05} className="w-24" value={s.pfAnte} onChange={(e) => s.set({ pfAnte: Math.max(0, Number(e.target.value)) })} data-testid="pf-ante" />
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={s.pfBbAnte} onChange={(e) => s.set({ pfBbAnte: e.target.checked })} />
            {t("icm.pf.bb_ante")}
          </label>
          <label className="flex flex-col gap-1">
            {t("icm.pf.model")}
            <Select className="w-40" value={s.pfModel} onChange={(e) => s.set({ pfModel: e.target.value as "chips" | "icm" })} data-testid="pf-model">
              <option value="chips">{t("icm.pf.chips")}</option>
              <option value="icm">{t("icm.pf.icm")}</option>
            </Select>
          </label>
          {s.pfModel === "icm" && (
            <label className="flex flex-col gap-1">
              {t("icm.pf.fgs")}
              <Select className="w-44" value={s.pfFgs} onChange={(e) => s.set({ pfFgs: Number(e.target.value) })} data-testid="pf-fgs">
                <option value={0}>{t("icm.pf.fgs_off")}</option>
                <option value={1}>{t("icm.pf.fgs_n", { n: 1 })}</option>
                <option value={2}>{t("icm.pf.fgs_n", { n: 2 })}</option>
                <option value={3}>{t("icm.pf.fgs_n", { n: 3 })}</option>
              </Select>
            </label>
          )}
        </div>
        {s.pfModel === "icm" && (
          <div className="flex flex-col gap-d2">
            <PayoutPicker value={s.pfPayouts} onChange={(v) => s.set({ pfPayouts: v })} testId="pf-payouts" />
            <label className="flex flex-col gap-1 text-sm">
              {t("icm.pf.others")}
              <Input value={s.pfOthers} onChange={(e) => s.set({ pfOthers: e.target.value })} placeholder="12, 30, 8" />
            </label>
          </div>
        )}
        <table className="w-full max-w-3xl text-sm" data-testid="pf-stacks">
          <thead className="text-left text-xs text-muted">
            <tr>
              <th className="py-1">{t("icm.pf.position")}</th>
              <th>{t("icm.pf.stack_bb")}</th>
              <th className="w-1/2">{t("icm.pf.what_if")}</th>
              {s.pfModel === "icm" && <th>{t("icm.pf.bounty")}</th>}
            </tr>
          </thead>
          <tbody>
            {s.pfStacks.map((x, i) => (
              <tr key={i}>
                <td className="py-0.5 font-medium">{pos[i]}</td>
                <td>
                  <Input type="number" min={1} max={100} step={0.5} className="w-24" value={x} onChange={(e) => setStack(i, Number(e.target.value))} data-testid="pf-stack" aria-label={pos[i]} />
                </td>
                <td>
                  <input type="range" min={1} max={30} step={0.5} value={x} onChange={(e) => setStack(i, Number(e.target.value))} className="w-full" aria-label={t("icm.pf.what_if")} />
                </td>
                {s.pfModel === "icm" && (
                  <td>
                    <Input
                      type="number"
                      min={0}
                      step={0.01}
                      className="w-24"
                      value={s.pfBounties[i] ?? 0}
                      onChange={(e) => s.set({ pfBounties: s.pfStacks.map((_, j) => (j === i ? Number(e.target.value) : (s.pfBounties[j] ?? 0))) })}
                      aria-label={t("icm.pf.bounty")}
                    />
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
        <p className="text-xs text-muted">{t("icm.pf.model_help")}</p>
        {!spot && <p className="text-sm text-danger">{t("icm.bad_prizes")}</p>}
        {error && (
          <p className="text-sm text-danger" data-testid="pf-error">
            {error}
          </p>
        )}
      </Card>

      {sol && node && (
        <Card className={`flex flex-col gap-d3 p-d4 ${busy ? "opacity-60" : ""}`} data-testid="pf-solution">
          <div className="flex flex-wrap items-center gap-d2 text-sm">
            <Select className="w-64" value={decisionKey(node.decision)} onChange={(e) => setSel(e.target.value)} data-testid="pf-node">
              {sol.nodes.map((x) => (
                <option key={decisionKey(x.decision)} value={decisionKey(x.decision)}>
                  {label(x.decision)}
                </option>
              ))}
            </Select>
            <span data-testid="pf-share">{t("icm.pf.share", { p: (share * 100).toFixed(1) })}</span>
            <span className="text-xs text-muted">{t("icm.pf.status", { it: sol.iterations, e: fmt(sol.exploitability) })}</span>
            <Button
              size="sm"
              variant="outline"
              onClick={() => {
                setLocking(node.decision);
                setLockText("");
              }}
              data-testid="pf-lock"
            >
              <Lock className="h-4 w-4" />
              {t("icm.pf.lock")}
            </Button>
          </div>
          <ClassGrid freq={node.freq} evAct={node.evAct} evFold={node.evFold} fmt={fmt} act={node.decision.kind === "push" ? t("icm.pf.push") : t("icm.pf.call")} />
          {s.pfLocks.length > 0 && (
            <ul className="flex flex-col gap-1 text-sm" data-testid="pf-locks">
              {s.pfLocks.map(([d, , text]) => (
                <li key={decisionKey(d)} className="flex items-center gap-d2">
                  <Lock className="h-3.5 w-3.5" />
                  {label(d)} : <code className="truncate text-xs">{text}</code>
                  <Button size="icon" variant="ghost" aria-label={t("icm.pf.unlock")} onClick={() => s.set({ pfLocks: s.pfLocks.filter(([x]) => decisionKey(x) !== decisionKey(d)) })}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <table className="max-w-md text-sm">
            <tbody>
              {sol.values.map((v, i) => (
                <tr key={i}>
                  <td className="pr-d3 text-muted">{pos[i]}</td>
                  <td className="tabular-nums">{fmt(v)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}

      {locking && (
        <Modal
          title={t("icm.pf.lock_title", { node: label(locking) })}
          onClose={() => {
            const w = tryParseRange(lockText);
            if (w && lockText.trim()) {
              const f = classFreq(w, CELL_COMBOS);
              s.set({ pfLocks: [...s.pfLocks.filter(([x]) => decisionKey(x) !== decisionKey(locking)), [locking, f, lockText]] });
            }
            setLocking(null);
          }}
        >
          <p className="mb-d2 text-sm text-muted">{t("icm.pf.lock_help")}</p>
          <RangeEditor value={lockText} onChange={setLockText} testId="lock-editor" />
        </Modal>
      )}
    </div>
  );
}
