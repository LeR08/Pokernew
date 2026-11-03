import { useTranslation } from "react-i18next";
import { NavLink, useNavigate, useParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { getBackend } from "@/lib/backend";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { BatchTab } from "./BatchTab";
import { Library } from "./Library";
import { NewSolve } from "./NewSolve";
import { useSolverUi } from "./store";
import { Viewer } from "./Viewer";

const TABS = ["new", "viewer", "library", "compare", "batch"] as const;

export function SolverPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const ui = useSolverUi();
  const { tab = "new" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "new";
  const resolve = () =>
    void getBackend()
      .then((b) => b.solverLoadedSpot(0))
      .then((x) => {
        if (x) ui.set({ spot: x[1] });
        navigate("/m/solver/new");
      });
  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/solver/${x}`}
            data-testid={`solver-tab-${x}`}
            className={cn("-mb-px border-b-2 px-d3 py-d2 text-sm", x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg")}
          >
            {t(`solver.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "new" && <NewSolve />}
      {current === "viewer" && (
        <Card className="flex flex-col gap-d3 p-d4">
          {ui.locks.length > 0 && (
            <div className="flex flex-wrap items-center gap-d2 text-sm">
              <span className="text-warning">{t("solver.with_locks", { n: ui.locks.length })}</span>
              <Button size="sm" onClick={resolve} data-testid="solver-resolve">
                {t("solver.resolve")}
              </Button>
              <Button size="sm" variant="ghost" onClick={() => ui.set({ locks: [] })}>
                {t("solver.clear_locks")}
              </Button>
            </div>
          )}
          <Viewer slot={0} />
        </Card>
      )}
      {current === "library" && <Library />}
      {current === "batch" && <BatchTab />}
      {current === "compare" && (
        <div className="grid grid-cols-1 gap-d4 xl:grid-cols-2">
          {([0, 1] as const).map((s) => (
            <Card key={s} className="flex flex-col gap-d2 p-d3">
              <h2 className="text-sm font-semibold">{t(s === 0 ? "solver.compare.a" : "solver.compare.b")}</h2>
              <Viewer slot={s} compact />
            </Card>
          ))}
          <p className="text-xs text-muted xl:col-span-2">{t("solver.compare.help")}</p>
        </div>
      )}
    </div>
  );
}
