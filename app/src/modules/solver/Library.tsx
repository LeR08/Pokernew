/** Cached solutions: open, compare, rename, delete. */
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { getBackend } from "@/lib/backend";
import { formatDate } from "@/lib/importApi";
import type { CacheEntry } from "@/lib/solver";

export function Library() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [list, setList] = useState<CacheEntry[] | null>(null);
  const [filter, setFilter] = useState("");
  const [error, setError] = useState<string | null>(null);
  const reload = () => void getBackend().then((b) => b.solverCacheList()).then(setList);
  useEffect(reload, []);

  const open = (hash: string, slot: 0 | 1) =>
    void getBackend()
      .then((b) => b.solverCacheLoad(hash, slot))
      .then(() => navigate(slot === 0 ? "/m/solver/viewer" : "/m/solver/compare"))
      .catch((e: Error) => setError(e.message));

  const shown = (list ?? []).filter((e) => `${e.name} ${e.spot.board}`.toLowerCase().includes(filter.toLowerCase()));
  return (
    <Card className="flex flex-col gap-d3 p-d4">
      <Input className="max-w-sm" value={filter} onChange={(e) => setFilter(e.target.value)} placeholder={t("solver.library.filter")} aria-label={t("solver.library.filter")} />
      {list && list.length === 0 && <p className="text-sm text-muted">{t("solver.library.empty")}</p>}
      {error && <p className="text-sm text-danger">{error}</p>}
      <table className="w-full text-sm" data-testid="library">
        <thead className="text-left text-xs text-muted">
          <tr>
            <th className="py-1">{t("solver.library.name")}</th>
            <th>{t("solver.board")}</th>
            <th>{t("solver.library.spot")}</th>
            <th className="text-right">{t("solver.library.expl")}</th>
            <th className="text-right">{t("solver.library.size")}</th>
            <th>{t("solver.library.date")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {shown.map((e) => (
            <tr key={e.hash} className="border-t border-border" data-testid="library-row">
              <td className="py-1">
                {e.name}
                {e.locks.length > 0 && <span className="ml-1 text-xs text-warning">{t("solver.library.locked", { n: e.locks.length })}</span>}
              </td>
              <td className="font-mono">{e.spot.board}</td>
              <td>
                {e.spot.pot} / {e.spot.stack} bb
              </td>
              <td className="text-right tabular-nums">{e.exploitabilityPct.toFixed(2)} %</td>
              <td className="text-right tabular-nums">{(e.bytes / 1024 / 1024).toFixed(0)} Mo</td>
              <td>{formatDate(e.createdAt, i18n.language)}</td>
              <td className="flex justify-end gap-1">
                <Button size="sm" onClick={() => open(e.hash, 0)} data-testid="library-open">
                  {t("solver.library.open")}
                </Button>
                <Button size="sm" variant="outline" onClick={() => open(e.hash, 1)} data-testid="library-compare">
                  {t("solver.library.compare")}
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => {
                    const name = window.prompt(t("solver.library.rename"), e.name);
                    if (name) void getBackend().then((b) => b.solverCacheRename(e.hash, name)).then(reload);
                  }}
                >
                  {t("solver.library.rename")}
                </Button>
                <Button size="sm" variant="ghost" onClick={() => void getBackend().then((b) => b.solverCacheDelete(e.hash)).then(reload)} data-testid="library-delete">
                  {t("solver.library.delete")}
                </Button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </Card>
  );
}
