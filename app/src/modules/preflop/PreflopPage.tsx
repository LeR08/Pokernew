import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { Charts } from "./Charts";
import { PreflopSolver } from "./PreflopSolver";

const TABS = ["charts", "solver"] as const;

export function PreflopPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { tab = "charts" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "charts";
  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/preflop/${x}`}
            data-testid={`preflop-tab-${x}`}
            className={cn("-mb-px border-b-2 px-d3 py-d2 text-sm", x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg")}
          >
            {t(`preflop.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "charts" ? <Charts /> : <PreflopSolver />}
    </div>
  );
}
