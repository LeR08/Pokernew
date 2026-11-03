import { NavLink, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { ModuleManifest } from "../types";
import { CalcTab } from "./CalcTab";
import { PayoutsTab } from "./PayoutsTab";
import { PushFoldTab } from "./PushFoldTab";
import { QuizTab } from "./QuizTab";
import { ReviewTab } from "./ReviewTab";

const TABS = ["pushfold", "calc", "review", "quiz", "payouts"] as const;

export function IcmPage({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const { tab = "pushfold" } = useParams();
  const current = (TABS as readonly string[]).includes(tab) ? tab : "pushfold";
  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-d4 p-d6">
      <h1 className="text-2xl font-semibold">{t(manifest.name)}</h1>
      <nav className="flex flex-wrap gap-d1 border-b border-border" aria-label={t(manifest.name)}>
        {TABS.map((x) => (
          <NavLink
            key={x}
            to={`/m/icm/${x}`}
            data-testid={`icm-tab-${x}`}
            className={cn("-mb-px border-b-2 px-d3 py-d2 text-sm", x === current ? "border-primary font-medium" : "border-transparent text-muted hover:text-fg")}
          >
            {t(`icm.tabs.${x}`)}
          </NavLink>
        ))}
      </nav>
      {current === "pushfold" && <PushFoldTab />}
      {current === "calc" && <CalcTab />}
      {current === "review" && <ReviewTab />}
      {current === "quiz" && <QuizTab />}
      {current === "payouts" && <PayoutsTab />}
    </div>
  );
}
