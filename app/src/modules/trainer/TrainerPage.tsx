import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { QuizTab } from "../icm/QuizTab";
import { Analysis } from "./Analysis";
import { History } from "./History";
import { PostflopDrill } from "./PostflopDrill";
import { PreflopDrill } from "./PreflopDrill";

const TABS = ["postflop", "preflop", "pushfold", "analysis", "history"] as const;

export function TrainerPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { tab = "postflop" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "postflop";
  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/trainer/${x}`}
            data-testid={`trainer-tab-${x}`}
            className={cn("-mb-px border-b-2 px-d3 py-d2 text-sm", x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg")}
          >
            {t(`trainer.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "postflop" && <PostflopDrill />}
      {current === "preflop" && <PreflopDrill />}
      {current === "pushfold" && <QuizTab />}
      {current === "analysis" && <Analysis />}
      {current === "history" && <History />}
    </div>
  );
}
