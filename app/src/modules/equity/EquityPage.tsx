import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { Analysis } from "./Analysis";
import { Calculator } from "./Calculator";
import { Textures } from "./Textures";

const TABS = ["calc", "analysis", "textures"] as const;

export function EquityPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { tab = "calc" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "calc";
  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/equity/${x}`}
            data-testid={`tab-${x}`}
            className={cn(
              "-mb-px border-b-2 px-d3 py-d2 text-sm",
              x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg",
            )}
          >
            {t(`equity.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "calc" && <Calculator />}
      {current === "analysis" && <Analysis />}
      {current === "textures" && <Textures />}
    </div>
  );
}
