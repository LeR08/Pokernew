/**
 * Solution viewer: breadcrumb of actions, 13×13 grid coloured by action mix,
 * per-combo panel, both ranges' EV/equity/EQR, range breakdown, node locking,
 * export. `slot` 0 or 1 (side by side comparison).
 */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Lock as LockIcon } from "lucide-react";
import { PlayingCard } from "@/components/PlayingCard";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { getBackend } from "@/lib/backend";
import type { Breakdown } from "@/lib/equity";
import { cellName } from "@/lib/range";
import { actionLabel, type NodeView } from "@/lib/solver";
import { cn } from "@/lib/utils";
import { BreakdownTable } from "../equity/Analysis";
import { Modal } from "../equity/Modal";
import { useSolverUi } from "./store";

/** Action colours: categorical slots of the dataviz reference palette, in fixed order by action index. */
const ACTION_COLOURS = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#6250d6", "#e34948"];
export const actionColour = (i: number) => ACTION_COLOURS[i % ACTION_COLOURS.length];

const pct = (x: number) => `${(x * 100).toFixed(1)} %`;

interface Step {
  label: string;
  index: number;
}

export function Viewer({ slot, compact = false }: { slot: 0 | 1; compact?: boolean }) {
  const { t } = useTranslation();
  const ui = useSolverUi();
  const [path, setPath] = useState<number[]>([]);
  const [steps, setSteps] = useState<Step[]>([]);
  const [node, setNode] = useState<NodeView | null>(null);
  const [cell, setCell] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [bd, setBd] = useState<Breakdown | null>(null);
  const [locking, setLocking] = useState<number[] | null>(null);
  const [msg, setMsg] = useState<string | null>(null);

  useEffect(() => {
    setBd(null);
    void getBackend()
      .then((b) => b.solverView(slot, path))
      .then((v) => {
        setNode(v);
        setError(null);
      })
      .catch((e: Error) => setError(e.message));
  }, [slot, path]);

  const go = (index: number, label: string) => {
    setPath([...path, index]);
    setSteps([...steps, { index, label }]);
    setCell(null);
  };
  const back = (k: number) => {
    setPath(path.slice(0, k));
    setSteps(steps.slice(0, k));
    setCell(null);
  };

  if (error) return <p className="text-sm text-muted" data-testid="viewer-empty">{t("solver.viewer.none")}</p>;
  if (!node) return null;
  const actor = node.player;
  const me = actor !== null ? node.players[actor] : null;
  const hands = me && cell !== null ? me.hands.filter((h) => cellName(cellIndex(h.cards)) === cellName(cell)) : [];

  return (
    <div className="flex flex-col gap-d3" data-testid={`viewer-${slot}`}>
      <nav className="flex flex-wrap items-center gap-1 text-sm" data-testid="viewer-path">
        <button className="rounded px-1.5 py-0.5 hover:bg-surface-alt" onClick={() => back(0)}>
          {t("solver.viewer.root")}
        </button>
        {steps.map((s, k) => (
          <span key={k} className="flex items-center gap-1">
            <span className="text-muted">›</span>
            <button className="rounded px-1.5 py-0.5 hover:bg-surface-alt" onClick={() => back(k + 1)}>
              {s.label}
            </button>
          </span>
        ))}
      </nav>
      <div className="flex flex-wrap items-center gap-d3 text-sm">
        <span className="flex gap-0.5">
          {node.board.map((c) => (
            <PlayingCard key={c} card={c} size="sm" />
          ))}
        </span>
        <span>{t("solver.viewer.pot", { pot: node.pot.toFixed(2) })}</span>
        {actor !== null && <span className="font-medium">{t("solver.viewer.to_act", { who: actor === 0 ? "OOP" : "IP" })}</span>}
        {node.locked && (
          <span className="flex items-center gap-1 text-warning">
            <LockIcon className="h-3.5 w-3.5" />
            {t("solver.viewer.locked")}
          </span>
        )}
      </div>

      {node.kind === "terminal" && <p className="text-sm text-muted">{t("solver.viewer.terminal")}</p>}

      {node.kind === "chance" && (
        <Card className="flex flex-col gap-d2 p-d3">
          <p className="text-sm">{t("solver.viewer.pick_card")}</p>
          <div className="flex flex-wrap gap-1" data-testid="viewer-cards">
            {node.cards.map((c) => (
              <button key={c} onClick={() => go("23456789TJQKA".indexOf(c[0]) * 4 + "cdhs".indexOf(c[1]), c)} data-card={c}>
                <PlayingCard card={c} size="sm" />
              </button>
            ))}
          </div>
        </Card>
      )}

      {node.kind === "action" && me && (
        <>
          <div className="flex flex-wrap gap-d2" data-testid="viewer-actions">
            {node.actions.map((a, i) => (
              <Button key={i} size="sm" variant="outline" onClick={() => go(i, `${actor === 0 ? "OOP" : "IP"} ${actionLabel(a, t)}`)} data-testid="viewer-action">
                <span className="inline-block h-2.5 w-2.5 rounded-sm" style={{ background: actionColour(i) }} aria-hidden />
                {actionLabel(a, t)} · {pct(node.totals[i] ?? 0)}
              </Button>
            ))}
          </div>
          <div className={cn("grid gap-d4", compact ? "grid-cols-1" : "grid-cols-1 xl:grid-cols-[minmax(0,34rem)_1fr]")}>
            <div className="grid w-full gap-px rounded-md bg-border p-px" style={{ gridTemplateColumns: "repeat(13, minmax(0, 1fr))" }} data-testid="viewer-grid">
              {me.grid.map((g) => (
                <button
                  key={g.cell}
                  data-name={cellName(g.cell)}
                  onClick={() => setCell(g.cell)}
                  title={g.weight > 0 ? `${cellName(g.cell)} · ${node.actions.map((a, i) => `${actionLabel(a, t)} ${pct(g.freq[i] ?? 0)}`).join(" · ")}` : cellName(g.cell)}
                  className={cn("relative flex aspect-square overflow-hidden bg-surface text-[10px] sm:text-xs", cell === g.cell && "ring-2 ring-inset ring-fg")}
                >
                  {g.weight > 0 && (
                    <span className="absolute inset-0 flex" aria-hidden>
                      {g.freq.map((f, i) => (
                        <span key={i} style={{ width: `${f * 100}%`, background: actionColour(i) }} />
                      ))}
                    </span>
                  )}
                  <span className={cn("relative m-auto", g.weight > 0 ? "font-medium text-white [text-shadow:0_0_2px_#000]" : "text-muted")}>{cellName(g.cell)}</span>
                </button>
              ))}
            </div>
            <div className="flex flex-col gap-d3 text-sm">
              <table className="max-w-md" data-testid="viewer-ranges">
                <thead className="text-left text-xs text-muted">
                  <tr>
                    <th />
                    <th className="text-right">{t("solver.viewer.ev_bb")}</th>
                    <th className="text-right">{t("solver.viewer.equity")}</th>
                    <th className="text-right">{t("solver.viewer.eqr")}</th>
                  </tr>
                </thead>
                <tbody>
                  {node.players.map((p, i) => (
                    <tr key={i} className={i === actor ? "font-semibold" : ""}>
                      <td className="py-0.5">{i === 0 ? "OOP" : "IP"}</td>
                      <td className="text-right tabular-nums">{p.ev.toFixed(2)}</td>
                      <td className="text-right tabular-nums">{pct(p.equity)}</td>
                      <td className="text-right tabular-nums">{pct(p.eqr)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
              {cell !== null && (
                <div data-testid="viewer-combos">
                  <p className="mb-1 font-medium">{cellName(cell)}</p>
                  <table className="w-full text-xs">
                    <thead className="text-left text-muted">
                      <tr>
                        <th>{t("solver.viewer.combo")}</th>
                        {node.actions.map((a, i) => (
                          <th key={i} className="text-right">
                            {actionLabel(a, t)}
                          </th>
                        ))}
                        <th className="text-right">{t("solver.viewer.ev")}</th>
                        <th className="text-right">{t("solver.viewer.equity")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {hands.map((h) => (
                        <tr key={h.combo}>
                          <td className="font-mono">{h.cards}</td>
                          {h.strategy.map((f, i) => (
                            <td key={i} className="text-right tabular-nums">
                              {pct(f)}
                            </td>
                          ))}
                          <td className="text-right tabular-nums">{h.ev.toFixed(2)}</td>
                          <td className="text-right tabular-nums">{pct(h.equity)}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {hands.length === 0 && <p className="text-xs text-muted">{t("solver.viewer.not_in_range")}</p>}
                </div>
              )}
              {!compact && (
                <div className="flex flex-wrap gap-d2">
                  <Button size="sm" variant="outline" onClick={() => void getBackend().then((b) => b.solverBreakdown(slot, path, actor!)).then(setBd)} data-testid="viewer-breakdown">
                    {t("solver.viewer.breakdown")}
                  </Button>
                  <Button size="sm" variant="outline" onClick={() => setLocking(node.actions.map(() => 1 / node.actions.length))} data-testid="viewer-lock">
                    <LockIcon className="h-4 w-4" />
                    {t("solver.viewer.lock")}
                  </Button>
                  {(["csv", "json"] as const).map((f) => (
                    <Button key={f} size="sm" variant="ghost" onClick={() => void getBackend().then((b) => b.solverExport(slot, path, f)).then((p) => setMsg(t("solver.viewer.exported", { path: p })))}>
                      {t("solver.viewer.export", { f: f.toUpperCase() })}
                    </Button>
                  ))}
                </div>
              )}
              {msg && <p className="text-xs text-muted">{msg}</p>}
            </div>
          </div>
          {bd && (
            <div className="grid grid-cols-1 gap-d4 lg:grid-cols-2" data-testid="viewer-bd">
              <BreakdownTable rows={bd.made} kind="made" />
              <BreakdownTable rows={bd.draws} kind="draws" />
            </div>
          )}
        </>
      )}

      {locking && node.kind === "action" && me && (
        <Modal
          title={t("solver.viewer.lock_title")}
          onClose={() => {
            const sum = locking.reduce((a, b) => a + b, 0) || 1;
            const n = me.hands.length;
            const strategy: number[] = [];
            locking.forEach((f) => {
              for (let k = 0; k < n; k++) strategy.push(Math.max(1e-6, f / sum));
            });
            ui.set({ locks: [...ui.locks.filter((l) => l.path.join() !== path.join()), { path, strategy }] });
            setLocking(null);
            setMsg(t("solver.viewer.lock_added"));
          }}
        >
          <p className="mb-d3 text-sm text-muted">{t("solver.viewer.lock_help")}</p>
          <div className="flex flex-col gap-d2">
            {node.actions.map((a, i) => (
              <label key={i} className="flex items-center gap-d3 text-sm">
                <span className="w-32">{actionLabel(a, t)}</span>
                <input
                  type="range"
                  min={0}
                  max={100}
                  value={Math.round(locking[i] * 100)}
                  onChange={(e) => setLocking(locking.map((x, j) => (j === i ? Number(e.target.value) / 100 : x)))}
                  data-testid="lock-slider"
                />
                <span className="w-12 tabular-nums">{Math.round(locking[i] * 100)} %</span>
              </label>
            ))}
          </div>
        </Modal>
      )}
    </div>
  );
}

/** Grid cell of a combo string like "AhKd". */
function cellIndex(cards: string): number {
  const r = (c: string) => 12 - "23456789TJQKA".indexOf(c);
  const a = r(cards[0]);
  const b = r(cards[2]);
  if (a === b) return a * 13 + a;
  const suited = cards[1] === cards[3];
  return suited ? Math.min(a, b) * 13 + Math.max(a, b) : Math.max(a, b) * 13 + Math.min(a, b);
}
