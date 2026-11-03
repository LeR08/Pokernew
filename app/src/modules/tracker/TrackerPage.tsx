import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { FilterBar } from "./FilterBar";
import { GraphView } from "./GraphView";
import { HandsList } from "./HandsList";
import { Overview } from "./Overview";
import { Positions } from "./Positions";
import { Grid } from "./Grid";
import { Leaks } from "./Leaks";
import { Opponents } from "./Opponents";
import { Sessions } from "./Sessions";
import { StatsEditor } from "./StatsEditor";
import { Tournaments } from "./Tournaments";

const TABS = ["overview", "sessions", "leaks", "report", "grid", "graph", "tournaments", "hands", "opponents", "stats"] as const;

export function TrackerPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { tab = "overview" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "overview";
  return (
    <div className="mx-auto flex max-w-7xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/tracker/${x}`}
            className={cn(
              "-mb-px border-b-2 px-d3 py-d2 text-sm",
              x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg",
            )}
          >
            {t(`tracker.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current !== "stats" && <FilterBar />}
      {current === "overview" && <Overview />}
      {current === "sessions" && <Sessions />}
      {current === "leaks" && <Leaks />}
      {current === "grid" && <Grid />}
      {current === "report" && <Positions />}
      {current === "graph" && <GraphView />}
      {current === "tournaments" && <Tournaments />}
      {current === "hands" && <HandsList />}
      {current === "opponents" && <Opponents />}
      {current === "stats" && <StatsEditor />}
    </div>
  );
}
