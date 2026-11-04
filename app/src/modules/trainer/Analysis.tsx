/** Hand analysis against the solver: queue, progress, report by hand and by kind of spot. */
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useSearchParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input, Select } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { useConfig, type Settings } from "@/lib/config";
import { DEFAULT_PARAMS, DEFAULT_TREE, type Tree } from "@/lib/solver";
import type { AnalysisProgress } from "@/lib/trainer";
import { useFilter } from "../tracker/api";

export function Analysis() {
  const { t } = useTranslation();
  const [params] = useSearchParams();
  const fromTracker = useMemo(() => (params.get("hands") ?? "").split(",").filter(Boolean).map(Number), [params]);
  const filter = useFilter();
  const settings = useConfig<Settings>("settings", "settings").data as Settings | undefined;
  const [names, setNames] = useState<string[]>([]);
  const [format, setFormat] = useState("cash 6max 100bb");
  const [trees, setTrees] = useState<{ id: string; name: string; tree: Tree }[]>([]);
  const [treeId, setTreeId] = useState("srp-100bb-simple");
  const [target, setTarget] = useState(1);
  const [maxIt, setMaxIt] = useState(300);
  const [count, setCount] = useState(50);
  const [p, setP] = useState<AnalysisProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void getBackend().then(async (b) => {
      setNames(await b.rangesList());
      const ids = await b.configList("solver_trees").catch(() => [] as string[]);
      const out = [];
      for (const id of ids) {
        const v = (await b.configGet("solver_trees", id).catch(() => null)) as unknown as { name: string; tree: Partial<Tree> } | null;
        if (v) out.push({ id, name: v.name, tree: { ...DEFAULT_TREE, ...v.tree } as Tree });
      }
      setTrees(out);
    });
    const h = setInterval(() => void getBackend().then((b) => b.analysisProgress()).then(setP), 800);
    return () => clearInterval(h);
  }, []);
  const folders = useMemo(() => [...new Set(names.filter((x) => x.includes("/")).map((x) => x.split("/")[0]))].sort(), [names]);

  const start = async () => {
    setError(null);
    const b = await getBackend();
    const ids = fromTracker.length ? fromTracker : (await b.trackerHands(filter, count, 0)).rows.map((h) => h.handId);
    const tree = trees.find((x) => x.id === treeId)?.tree ?? DEFAULT_TREE;
    const memoryCapMb = settings?.performance?.ram_cap_mb ?? DEFAULT_PARAMS.memoryCapMb;
    await b.analysisStart(ids, format, tree, { ...DEFAULT_PARAMS, targetPct: target, maxIterations: maxIt, memoryCapMb }).catch((e: Error) => setError(e.message));
  };

  const ok = (p?.rows ?? []).filter((r) => r.analysis);
  const skipped = (p?.rows ?? []).filter((r) => !r.analysis);
  const sorted = [...ok].sort((a, b) => b.analysis!.totalLossBb - a.analysis!.totalLossBb);
  const leaks = useMemo(() => {
    const m = new Map<string, { n: number; loss: number }>();
    for (const r of ok) for (const d of r.analysis!.decisions) {
      const x = m.get(d.nodeKey) ?? { n: 0, loss: 0 };
      x.n += 1;
      x.loss += d.lossBb;
      m.set(d.nodeKey, x);
    }
    return [...m.entries()].map(([k, v]) => ({ k, n: v.n, avg: v.loss / v.n })).sort((a, b) => b.avg - a.avg);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p]);
  const reasons = useMemo(() => {
    const m = new Map<string, number>();
    for (const r of skipped) m.set(r.error ?? "?", (m.get(r.error ?? "?") ?? 0) + 1);
    return [...m.entries()];
  }, [skipped]);

  return (
    <div className="flex flex-col gap-d4">
      <Card className="flex flex-col gap-d3 p-d4 text-sm">
        <p className="text-muted">{fromTracker.length ? t("trainer.ana.from_tracker", { n: fromTracker.length }) : t("trainer.ana.help")}</p>
        <div className="flex flex-wrap items-end gap-d3">
          <label className="flex min-w-56 flex-col gap-1">
            {t("trainer.ana.format")}
            <Select value={format} onChange={(e) => setFormat(e.target.value)} data-testid="ana-format">
              {folders.map((f) => (
                <option key={f} value={f}>
                  {f}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex min-w-48 flex-col gap-1">
            {t("trainer.ana.tree")}
            <Select value={treeId} onChange={(e) => setTreeId(e.target.value)}>
              {trees.map((x) => (
                <option key={x.id} value={x.id}>
                  {x.name}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.target")}
            <Input type="number" min={0.1} step={0.1} className="w-20" value={target} onChange={(e) => setTarget(Number(e.target.value))} />
          </label>
          <label className="flex flex-col gap-1">
            {t("solver.max_iterations")}
            <Input type="number" min={20} step={50} className="w-24" value={maxIt} onChange={(e) => setMaxIt(Number(e.target.value))} />
          </label>
          {!fromTracker.length && (
            <label className="flex flex-col gap-1">
              {t("trainer.ana.count")}
              <Input type="number" min={1} max={1000} className="w-24" value={count} onChange={(e) => setCount(Number(e.target.value))} />
            </label>
          )}
          {p?.running ? (
            <Button variant="danger" onClick={() => void getBackend().then((b) => b.analysisCancel())} data-testid="ana-cancel">
              {t("solver.cancel")}
            </Button>
          ) : (
            <Button onClick={() => void start()} data-testid="ana-start">
              {t("trainer.ana.start")}
            </Button>
          )}
        </div>
        <p className="text-xs text-muted">{t("trainer.ana.fairplay")}</p>
        {error && <p className="text-danger">{error}</p>}
        {p && (
          <p data-testid="ana-progress">
            {t("trainer.ana.progress", { done: p.done, total: p.total, ok: ok.length })}
            {p.waitingForTables && ` · ${t("trainer.ana.waiting")}`}
          </p>
        )}
      </Card>
      {leaks.length > 0 && (
        <Card className="flex flex-col gap-d2 p-d4" data-testid="ana-leaks">
          <h2 className="font-semibold">{t("trainer.ana.leaks")}</h2>
          <table className="max-w-2xl text-sm">
            <tbody>
              {leaks.map((l) => (
                <tr key={l.k}>
                  <td className="py-0.5 pr-d3">{l.k}</td>
                  <td className="pr-d3 text-right tabular-nums">−{l.avg.toFixed(2)} bb</td>
                  <td className="text-xs text-muted">{t("trainer.ana.decisions", { n: l.n })}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
      {sorted.length > 0 && (
        <Card className="flex flex-col gap-d2 p-d4" data-testid="ana-report">
          <h2 className="font-semibold">{t("trainer.ana.by_hand")}</h2>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-muted">
              <tr>
                <th className="py-1">{t("trainer.ana.hand")}</th>
                <th>{t("solver.board")}</th>
                <th>{t("trainer.ana.line")}</th>
                <th className="text-right">{t("trainer.ana.loss")}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {sorted.map((r) => (
                <tr key={r.handId} className="border-t border-border align-top" data-testid="ana-row">
                  <td className="py-1 font-mono">{r.analysis!.heroCards}</td>
                  <td className="font-mono">{r.analysis!.spot.board}</td>
                  <td className="text-xs">
                    {r.analysis!.decisions.map((d, i) => (
                      <div key={i} className={d.lossBb > 0.01 ? "text-warning" : "text-muted"}>
                        {d.street} : {d.played}
                        {d.played !== d.best ? ` → ${d.best}` : ""} {d.lossBb > 0.005 ? `(−${d.lossBb.toFixed(2)} bb)` : ""}
                        {d.approximate ? ` ${t("trainer.ana.approx")}` : ""}
                      </div>
                    ))}
                  </td>
                  <td className="text-right tabular-nums">−{r.analysis!.totalLossBb.toFixed(2)} bb</td>
                  <td className="text-right">
                    <Link className="text-primary underline" to={`/m/replayer/${r.handId}`}>
                      {t("trainer.ana.replay")}
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>
      )}
      {reasons.length > 0 && (
        <Card className="p-d4 text-xs text-muted" data-testid="ana-skipped">
          <p className="mb-1 font-medium">{t("trainer.ana.skipped", { n: skipped.length })}</p>
          {reasons.map(([r, n]) => (
            <p key={r}>
              {n} × {r}
            </p>
          ))}
        </Card>
      )}
    </div>
  );
}
