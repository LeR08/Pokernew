/** Preflop solver: settings, solve, clickable tree, 13×13 grid per action, save to the library. */
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { positionNames } from "@/lib/icm";
import { DEFAULT_PREFLOP, type Act, type PreflopConfig, type PreflopSolution } from "@/lib/preflop";
import { CELL_COMBOS, cellName, formatRange, NUM_COMBOS } from "@/lib/range";
import { cn } from "@/lib/utils";
import { actionColour } from "../solver/Viewer";

const size = (c: number) => CELL_COMBOS[c].length;

export function PreflopSolver() {
  const { t } = useTranslation();
  const [cfg, setCfg] = useState<PreflopConfig>(DEFAULT_PREFLOP);
  const [target, setTarget] = useState(0.005);
  const [maxIt, setMaxIt] = useState(2000);
  const [sol, setSol] = useState<PreflopSolution | null>(null);
  const [path, setPath] = useState<number[]>([0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [msg, setMsg] = useState<string | null>(null);
  const [cell, setCell] = useState<number | null>(null);
  const n = cfg.stacks.length;
  const pos = positionNames(n);
  const set = (p: Partial<PreflopConfig>) => setCfg({ ...cfg, ...p });

  const label = (a: Act) => (a.kind === "raise" || a.kind === "all_in" || a.kind === "call" ? t(`preflop.act.${a.kind}`, { n: +a.to.toFixed(2) }) : t(`preflop.act.${a.kind}`));

  const run = () => {
    setBusy(true);
    setError(null);
    void getBackend()
      .then((b) => b.preflopSolve(cfg, target, maxIt))
      .then((s) => {
        setSol(s);
        setPath([0]);
        setCell(null);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setBusy(false));
  };

  const node = sol ? sol.nodes[path[path.length - 1]] : null;
  const steps = useMemo(() => {
    if (!sol) return [];
    const out: string[] = [];
    for (let k = 1; k < path.length; k++) {
      const parent = sol.nodes[path[k - 1]];
      const i = parent.children.indexOf(path[k]);
      out.push(`${pos[parent.player ?? 0]} ${label(parent.actions[i])}`);
    }
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sol, path]);

  const totals = node && node.player !== null
    ? node.actions.map((_, k) => {
        let s = 0;
        let w = 0;
        for (let c = 0; c < 169; c++) {
          const x = size(c) * node.reach[c];
          s += x * node.freq[k][c];
          w += x;
        }
        return w > 0 ? s / w : 0;
      })
    : [];

  const saveAll = () => {
    if (!sol) return;
    const folder = `solveur/${name.trim() || `${n}j ${cfg.stacks[0]}bb`}`;
    const lines = ["# Local Poker Suite ranges v1"];
    const walk = (id: number, label0: string) => {
      const nd = sol.nodes[id];
      if (nd.player === null || nd.prob < 0.002) return;
      nd.actions.forEach((a, k) => {
        const lbl = `${label0}${label0 ? " · " : ""}${pos[nd.player!]} ${label(a)}`;
        if (a.kind !== "fold") {
          const w = new Float32Array(NUM_COMBOS);
          for (let c = 0; c < 169; c++) for (const i of CELL_COMBOS[c]) w[i] = Math.round(nd.reach[c] * nd.freq[k][c] * 1000) / 1000;
          lines.push(`[${lbl.replace(/[[\]/]/g, "-")}]`, formatRange(w));
        }
        walk(nd.children[k], lbl);
      });
    };
    walk(0, "");
    void getBackend()
      .then((b) => b.rangesImport(lines.join("\n") + "\n", folder, true))
      .then((k) => setMsg(t("preflop.solver.saved", { n: k, folder })))
      .catch((e: Error) => setMsg(e.message));
  };

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4 text-sm">
        <div className="flex flex-wrap items-end gap-d3">
          <label className="flex flex-col gap-1">
            {t("preflop.solver.players")}
            <Select className="w-20" value={n} onChange={(e) => set({ stacks: Array.from({ length: Number(e.target.value) }, () => cfg.stacks[0]), allowLimp: Number(e.target.value) === 2 && cfg.allowLimp })} data-testid="pf-players">
              {[2, 3, 4, 5, 6].map((k) => (
                <option key={k} value={k}>
                  {k}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.stack")}
            <Input type="number" min={5} max={300} className="w-24" value={cfg.stacks[0]} onChange={(e) => set({ stacks: cfg.stacks.map(() => Number(e.target.value)) })} data-testid="pf-stack" />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.ante")}
            <Input type="number" min={0} step={0.05} className="w-20" value={cfg.ante} onChange={(e) => set({ ante: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.open")}
            <Input type="number" min={1.5} step={0.25} className="w-20" value={cfg.open} onChange={(e) => set({ open: Number(e.target.value) })} data-testid="pf-open" />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.three_bet")}
            <Input type="number" min={1.5} step={0.25} className="w-20" value={cfg.threeBet} onChange={(e) => set({ threeBet: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.four_bet")}
            <Input type="number" min={1.5} step={0.1} className="w-20" value={cfg.fourBet} onChange={(e) => set({ fourBet: Number(e.target.value) })} />
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={cfg.allowLimp} onChange={(e) => set({ allowLimp: e.target.checked })} />
            {t("preflop.solver.limp")}
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={cfg.allowAllin} onChange={(e) => set({ allowAllin: e.target.checked })} />
            {t("preflop.solver.allin")}
          </label>
        </div>
        <div className="flex flex-wrap items-end gap-d3">
          <label className="flex flex-col gap-1">
            {t("preflop.solver.real_ip")}
            <Input type="number" min={0.5} max={1.5} step={0.05} className="w-20" value={cfg.realizationIp} onChange={(e) => set({ realizationIp: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.real_oop")}
            <Input type="number" min={0.5} max={1.5} step={0.05} className="w-20" value={cfg.realizationOop} onChange={(e) => set({ realizationOop: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.curve")}
            <Input type="number" min={0.5} max={4} step={0.1} className="w-20" value={cfg.realizationCurve} onChange={(e) => set({ realizationCurve: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.rake")}
            <Input type="number" min={0} max={20} step={0.5} className="w-20" value={cfg.rake * 100} onChange={(e) => set({ rake: Number(e.target.value) / 100 })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.rake_cap")}
            <Input type="number" min={0} step={0.5} className="w-20" value={cfg.rakeCap} onChange={(e) => set({ rakeCap: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.target")}
            <Input type="number" min={0} step={0.001} className="w-24" value={target} onChange={(e) => setTarget(Number(e.target.value))} />
          </label>
          <label className="flex flex-col gap-1">
            {t("preflop.solver.max_it")}
            <Input type="number" min={10} step={100} className="w-24" value={maxIt} onChange={(e) => setMaxIt(Number(e.target.value))} />
          </label>
          <Button onClick={run} disabled={busy} data-testid="pf-run">
            {busy ? t("preflop.solver.running") : t("preflop.solver.run")}
          </Button>
        </div>
        <p className="text-xs text-muted">{t("preflop.solver.help")}</p>
        {error && <p className="text-sm text-danger">{error}</p>}
      </Card>

      {sol && node && (
        <Card className="flex flex-col gap-d3 p-d4" data-testid="pf-solution">
          <p className="text-sm text-muted" data-testid="pf-status">
            {t("preflop.solver.status", { it: sol.iterations, e: sol.exploitability.toFixed(4), v: sol.values.map((v, i) => `${pos[i]} ${v >= 0 ? "+" : ""}${v.toFixed(3)}`).join(" · ") })}
          </p>
          <nav className="flex flex-wrap items-center gap-1 text-sm" data-testid="pf-path">
            <button className="rounded px-1.5 py-0.5 hover:bg-surface-alt" onClick={() => setPath([0])}>
              {t("solver.viewer.root")}
            </button>
            {steps.map((s, k) => (
              <span key={k} className="flex items-center gap-1">
                <span className="text-muted">›</span>
                <button className="rounded px-1.5 py-0.5 hover:bg-surface-alt" onClick={() => setPath(path.slice(0, k + 2))}>
                  {s}
                </button>
              </span>
            ))}
          </nav>
          {node.player === null ? (
            <p className="text-sm">
              {node.end?.type === "fold"
                ? t("preflop.solver.end_fold", { who: pos[node.end.winner] })
                : node.end?.type === "showdown"
                  ? t("preflop.solver.end_showdown", { pot: node.pot.toFixed(1) })
                  : t("preflop.solver.end_flop", { pot: node.pot.toFixed(1) })}
            </p>
          ) : (
            <>
              <p className="text-sm font-medium">{t("preflop.solver.to_act", { who: pos[node.player], p: (node.prob * 100).toFixed(1) })}</p>
              <div className="flex flex-wrap gap-d2" data-testid="pf-actions">
                {node.actions.map((a, k) => (
                  <Button key={k} size="sm" variant="outline" onClick={() => (setPath([...path, node.children[k]]), setCell(null))} data-testid="pf-action">
                    <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: actionColour(k) }} aria-hidden />
                    {label(a)} · {(totals[k] * 100).toFixed(1)} %
                  </Button>
                ))}
              </div>
              <div className="grid grid-cols-1 gap-d4 xl:grid-cols-[minmax(0,34rem)_1fr]">
                <div className="grid w-full gap-px rounded-md bg-border p-px" style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }} data-testid="pf-grid">
                  {Array.from({ length: 169 }, (_, c) => {
                    const r = node.reach[c];
                    return (
                      <button
                        key={c}
                        data-name={cellName(c)}
                        onClick={() => setCell(c)}
                        className={cn("relative flex aspect-square overflow-hidden bg-surface text-[10px] sm:text-xs", cell === c && "ring-2 ring-inset ring-fg")}
                        title={cellName(c)}
                      >
                        {r > 0.001 && (
                          <span className="absolute inset-x-0 bottom-0 flex" style={{ height: `${Math.min(1, r) * 100}%` }} aria-hidden>
                            {node.actions.map((_, k) => (
                              <span key={k} style={{ width: `${node.freq[k][c] * 100}%`, background: actionColour(k) }} />
                            ))}
                          </span>
                        )}
                        <span className={cn("relative m-auto", r > 0.5 ? "font-medium text-white [text-shadow:0_0_2px_#000]" : "text-muted")}>{cellName(c)}</span>
                      </button>
                    );
                  })}
                </div>
                {cell !== null && (
                  <table className="h-fit text-sm" data-testid="pf-cell">
                    <caption className="mb-1 text-left font-medium">
                      {cellName(cell)} · {t("preflop.solver.reach", { p: (node.reach[cell] * 100).toFixed(0) })}
                    </caption>
                    <tbody>
                      {node.actions.map((a, k) => (
                        <tr key={k}>
                          <td className="pr-d3">{label(a)}</td>
                          <td className="tabular-nums">{(node.freq[k][cell] * 100).toFixed(1)} %</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                )}
              </div>
              <p className="text-xs text-muted">{t("preflop.solver.grid_help")}</p>
            </>
          )}
          <div className="flex flex-wrap items-end gap-d2 border-t border-border pt-d3 text-sm">
            <label className="flex flex-col gap-1">
              {t("preflop.solver.save_name")}
              <Input className="w-64" value={name} onChange={(e) => setName(e.target.value)} placeholder={`${n}j ${cfg.stacks[0]}bb`} />
            </label>
            <Button size="sm" onClick={saveAll} data-testid="pf-save">
              {t("preflop.solver.save")}
            </Button>
            {msg && (
              <span className="text-muted" data-testid="pf-msg">
                {msg}
              </span>
            )}
          </div>
        </Card>
      )}
    </div>
  );
}
