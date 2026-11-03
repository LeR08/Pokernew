/** Spot set-up, tree, estimate, solve with live progress. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Pause, Play, Square } from "lucide-react";
import { CardPicker } from "@/components/range/CardPicker";
import { RangeEditor } from "@/components/range/RangeEditor";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { parseCards, tryParseRange } from "@/lib/range";
import type { Estimate, Progress, StreetSizes, Tree } from "@/lib/solver";
import { Modal } from "../equity/Modal";
import { useSolverUi } from "./store";

const STREETS = ["flop", "turn", "river"] as const;

function useTreePresets() {
  const [list, setList] = useState<{ id: string; name: string; pot?: number; stack?: number; tree: Tree }[]>([]);
  useEffect(() => {
    void getBackend().then(async (b) => {
      const ids = await b.configList("solver_trees").catch(() => [] as string[]);
      const out = [];
      for (const id of ids) {
        const v = (await b.configGet("solver_trees", id).catch(() => null)) as unknown as { name: string; pot?: number; stack?: number; tree: Partial<Tree> } | null;
        if (v) out.push({ id, name: v.name, pot: v.pot, stack: v.stack, tree: { ...useSolverUi.getState().spot.tree, ...v.tree } as Tree });
      }
      setList(out);
    });
  }, []);
  return list;
}

function ProgressChart({ p }: { p: Progress }) {
  const { t } = useTranslation();
  const pts = p.history.filter(([, e]) => e > 0);
  if (pts.length < 2) return null;
  const W = 600;
  const H = 160;
  const maxIt = Math.max(...pts.map(([i]) => i), 1);
  const ly = (e: number) => Math.log10(Math.max(e, 0.01));
  const top = ly(Math.max(...pts.map(([, e]) => e)));
  const bottom = Math.min(ly(0.05), ...pts.map(([, e]) => ly(e)));
  const x = (i: number) => 40 + (i / maxIt) * (W - 50);
  const y = (e: number) => 10 + ((top - ly(e)) / Math.max(0.01, top - bottom)) * (H - 30);
  return (
    <svg viewBox={`0 0 ${W} ${H}`} className="w-full max-w-2xl" role="img" aria-label={t("solver.chart")} data-testid="solver-chart">
      {[100, 10, 1, 0.1].filter((v) => ly(v) <= top + 0.01 && ly(v) >= bottom - 0.01).map((v) => (
        <g key={v}>
          <line x1={40} x2={W - 10} y1={y(v)} y2={y(v)} className="stroke-border" />
          <text x={34} y={y(v) + 3} textAnchor="end" className="fill-muted text-[10px]">
            {v} %
          </text>
        </g>
      ))}
      <path d={pts.map(([i, e], k) => `${k ? "L" : "M"}${x(i).toFixed(1)},${y(e).toFixed(1)}`).join("")} fill="none" stroke="#2a78d6" strokeWidth={2} />
    </svg>
  );
}

function SizesRow({ label, v, onChange }: { label: string; v: StreetSizes; onChange: (v: StreetSizes) => void }) {
  const { t } = useTranslation();
  return (
    <tr>
      <td className="py-0.5 pr-d2 font-medium">{label}</td>
      {(["oop", "ip"] as const).map((who) => (
        <td key={who} className="pr-d2">
          <span className="flex gap-1">
            <Input className="w-28 font-mono text-xs" value={v[who].bet} onChange={(e) => onChange({ ...v, [who]: { ...v[who], bet: e.target.value } })} aria-label={t("solver.tree.bet", { who })} />
            <Input className="w-20 font-mono text-xs" value={v[who].raise} onChange={(e) => onChange({ ...v, [who]: { ...v[who], raise: e.target.value } })} aria-label={t("solver.tree.raise", { who })} />
          </span>
        </td>
      ))}
      <td>
        <Input className="w-24 font-mono text-xs" value={v.donk} onChange={(e) => onChange({ ...v, donk: e.target.value })} placeholder="—" aria-label={t("solver.tree.donk")} />
      </td>
    </tr>
  );
}

export function NewSolve() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const ui = useSolverUi();
  const { spot, params } = ui;
  const presets = useTreePresets();
  const [est, setEst] = useState<Estimate | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [edit, setEdit] = useState<"oop" | "ip" | null>(null);
  const setSpot = (p: Partial<typeof spot>) => ui.set({ spot: { ...spot, ...p } });
  const setTree = (p: Partial<Tree>) => setSpot({ tree: { ...spot.tree, ...p } });
  const board = parseCards(spot.board);
  const valid = board !== null && board.length >= 3 && board.length <= 5 && !!tryParseRange(spot.oopRange) && !!tryParseRange(spot.ipRange) && spot.pot > 0 && spot.stack > 0;
  const running = progress && ["building", "solving", "paused"].includes(progress.state);

  useEffect(() => {
    setEst(null);
    if (!valid) return;
    const h = setTimeout(() => {
      void getBackend()
        .then((b) => b.solverEstimate(spot))
        .then((x) => {
          setEst(x);
          setError(null);
        })
        .catch((e: Error) => setError(e.message));
    }, 400);
    return () => clearTimeout(h);
  }, [spot, valid]);

  useEffect(() => {
    let alive = true;
    const tick = () =>
      void getBackend()
        .then((b) => b.solverProgress())
        .then(async (p) => {
          if (!alive) return;
          setProgress(p);
          if (p?.state === "done") {
            const b = await getBackend();
            await b.solverFinish();
            ui.set({ locks: [] });
            navigate("/m/solver/viewer");
          }
        })
        .catch(() => undefined);
    tick();
    const h = setInterval(tick, 500);
    return () => {
      alive = false;
      clearInterval(h);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const start = () => {
    setError(null);
    const name = ui.name.trim() || `${spot.board} · ${spot.pot}/${spot.stack} bb`;
    void getBackend()
      .then((b) => b.solverStart(spot, params, ui.locks, name))
      .catch((e: Error) => setError(e.message));
  };
  const memNeed = est ? (params.compression ? est.memoryCompressed : est.memory) / 1024 / 1024 : 0;

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4">
        <div className="grid grid-cols-1 gap-d3 md:grid-cols-2">
          {(["oop", "ip"] as const).map((who) => {
            const key = who === "oop" ? "oopRange" : "ipRange";
            return (
              <label key={who} className="flex flex-col gap-1 text-sm">
                {t(`solver.${who}_range`)}
                <span className="flex gap-d2">
                  <Input className="font-mono text-xs" value={spot[key]} onChange={(e) => setSpot({ [key]: e.target.value })} aria-invalid={!tryParseRange(spot[key])} data-testid={`solver-${who}`} />
                  <Button size="sm" variant="outline" onClick={() => setEdit(who)}>
                    {t("equity.edit_range")}
                  </Button>
                </span>
              </label>
            );
          })}
        </div>
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <CardPicker label={t("solver.board")} value={spot.board} onChange={(v) => setSpot({ board: v })} max={5} testId="solver-board" />
          {(
            [
              ["pot", 0.5],
              ["stack", 0.5],
            ] as const
          ).map(([k, step]) => (
            <label key={k} className="flex flex-col gap-1">
              {t(`solver.${k}`)}
              <Input type="number" min={0} step={step} className="w-24" value={spot[k]} onChange={(e) => setSpot({ [k]: Number(e.target.value) })} data-testid={`solver-${k}`} />
            </label>
          ))}
          <label className="flex flex-col gap-1">
            {t("solver.rake")}
            <Input type="number" min={0} max={20} step={0.5} className="w-20" value={spot.rake * 100} onChange={(e) => setSpot({ rake: Number(e.target.value) / 100 })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.rake_cap")}
            <Input type="number" min={0} step={0.5} className="w-20" value={spot.rakeCap} onChange={(e) => setSpot({ rakeCap: Number(e.target.value) })} />
          </label>
        </div>
      </Card>

      <Card className="flex flex-col gap-d3 overflow-x-auto p-d4">
        <div className="flex flex-wrap items-center gap-d2 text-sm">
          <span className="font-semibold">{t("solver.tree.title")}</span>
          <Select
            className="w-64"
            value=""
            onChange={(e) => {
              const p = presets.find((x) => x.id === e.target.value);
              if (p) setSpot({ tree: p.tree, ...(p.pot ? { pot: p.pot } : {}), ...(p.stack ? { stack: p.stack } : {}) });
            }}
            data-testid="solver-preset"
            aria-label={t("solver.tree.preset")}
          >
            <option value="">{t("solver.tree.preset")}</option>
            {presets.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </Select>
        </div>
        <table className="text-sm">
          <thead className="text-left text-xs text-muted">
            <tr>
              <th />
              <th>{t("solver.tree.oop")}</th>
              <th>{t("solver.tree.ip")}</th>
              <th>{t("solver.tree.donk")}</th>
            </tr>
          </thead>
          <tbody>
            {STREETS.map((s) => (
              <SizesRow key={s} label={t(`replayer.street.${s}`)} v={spot.tree[s]} onChange={(v) => setTree({ [s]: v })} />
            ))}
          </tbody>
        </table>
        <div className="flex flex-wrap gap-d3 text-sm">
          <label className="flex flex-col gap-1">
            {t("solver.tree.max_raises")}
            <Input type="number" min={0} max={6} className="w-20" value={spot.tree.max_raises} onChange={(e) => setTree({ max_raises: Number(e.target.value) })} data-testid="solver-max-raises" />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.tree.add_allin")}
            <Input type="number" min={0} step={0.1} className="w-20" value={spot.tree.add_allin_threshold} onChange={(e) => setTree({ add_allin_threshold: Number(e.target.value) })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.tree.force_allin")}
            <Input type="number" min={0} step={0.05} className="w-20" value={spot.tree.force_allin_threshold} onChange={(e) => setTree({ force_allin_threshold: Number(e.target.value) })} />
          </label>
        </div>
        <p className="text-xs text-muted">{t("solver.tree.help")}</p>
      </Card>

      <Card className="flex flex-col gap-d3 p-d4">
        <div className="flex flex-wrap items-end gap-d3 text-sm">
          <label className="flex flex-col gap-1">
            {t("solver.target")}
            <Input type="number" min={0.05} step={0.05} className="w-24" value={params.targetPct} onChange={(e) => ui.set({ params: { ...params, targetPct: Number(e.target.value) } })} data-testid="solver-target" />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.max_iterations")}
            <Input type="number" min={10} step={50} className="w-24" value={params.maxIterations} onChange={(e) => ui.set({ params: { ...params, maxIterations: Number(e.target.value) } })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.threads")}
            <Input type="number" min={0} max={64} className="w-20" value={params.threads} onChange={(e) => ui.set({ params: { ...params, threads: Number(e.target.value) } })} />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.memory_cap")}
            <Input type="number" min={256} step={256} className="w-24" value={params.memoryCapMb} onChange={(e) => ui.set({ params: { ...params, memoryCapMb: Number(e.target.value) } })} />
          </label>
          <label className="flex items-center gap-1">
            <input type="checkbox" checked={params.compression} onChange={(e) => ui.set({ params: { ...params, compression: e.target.checked } })} />
            {t("solver.compression")}
          </label>
          <label className="flex min-w-48 flex-1 flex-col gap-1">
            {t("solver.name")}
            <Input value={ui.name} onChange={(e) => ui.set({ name: e.target.value })} placeholder={`${spot.board} · ${spot.pot}/${spot.stack} bb`} />
          </label>
        </div>
        {est && (
          <p className={`text-sm ${memNeed > params.memoryCapMb ? "text-danger" : "text-muted"}`} data-testid="solver-estimate">
            {t("solver.estimate", { nodes: est.treeNodes, oop: est.oopHands, ip: est.ipHands, mb: Math.round(memNeed) })}
            {memNeed > params.memoryCapMb && ` ${t("solver.over_cap")}`}
          </p>
        )}
        {ui.locks.length > 0 && <p className="text-sm text-warning">{t("solver.with_locks", { n: ui.locks.length })}</p>}
        <div className="flex flex-wrap items-center gap-d2">
          {!running ? (
            <Button onClick={start} disabled={!valid || !est || memNeed > params.memoryCapMb} data-testid="solver-start">
              <Play className="h-4 w-4" />
              {t("solver.start")}
            </Button>
          ) : (
            <>
              {progress!.state === "paused" ? (
                <Button variant="outline" onClick={() => void getBackend().then((b) => b.solverControl("resume"))} data-testid="solver-resume">
                  <Play className="h-4 w-4" />
                  {t("solver.resume")}
                </Button>
              ) : (
                <Button variant="outline" onClick={() => void getBackend().then((b) => b.solverControl("pause"))} data-testid="solver-pause">
                  <Pause className="h-4 w-4" />
                  {t("solver.pause")}
                </Button>
              )}
              <Button variant="danger" onClick={() => void getBackend().then((b) => b.solverControl("cancel"))} data-testid="solver-cancel">
                <Square className="h-4 w-4" />
                {t("solver.cancel")}
              </Button>
            </>
          )}
        </div>
        {progress && progress.state !== "done" && (
          <div className="flex flex-col gap-d2" data-testid="solver-progress">
            <p className="text-sm">
              {t(`solver.state.${progress.state}`)} ·{" "}
              {t("solver.progress", {
                it: progress.iteration,
                max: progress.maxIterations,
                e: Number.isFinite(progress.exploitabilityPct) ? progress.exploitabilityPct.toFixed(3) : "—",
                s: (progress.elapsedMs / 1000).toFixed(0),
              })}
            </p>
            {progress.error && <p className="text-sm text-danger">{progress.error}</p>}
            <ProgressChart p={progress} />
          </div>
        )}
        {error && (
          <p className="text-sm text-danger" data-testid="solver-error">
            {error}
          </p>
        )}
      </Card>
      {edit && (
        <Modal title={t(`solver.${edit}_range`)} onClose={() => setEdit(null)}>
          <RangeEditor value={edit === "oop" ? spot.oopRange : spot.ipRange} onChange={(v) => setSpot(edit === "oop" ? { oopRange: v } : { ipRange: v })} dead={board ?? []} />
        </Modal>
      )}
    </div>
  );
}
