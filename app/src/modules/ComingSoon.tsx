import { useTranslation } from "react-i18next";
import type { ModuleManifest } from "./types";
import { Card } from "@/components/ui/card";

/** Placeholder page for modules not implemented yet. */
export function ComingSoon({ manifest }: { manifest: ModuleManifest }) {
  const { t } = useTranslation();
  const Icon = manifest.icon;
  return (
    <div className="mx-auto max-w-2xl p-d6">
      <Card className="flex flex-col items-center gap-d3 p-d6 text-center">
        <Icon className="h-10 w-10 text-primary" aria-hidden />
        <h1 className="text-xl font-semibold">{t(manifest.name)}</h1>
        <p className="text-muted">{t(manifest.description)}</p>
        <p className="text-sm" data-testid="coming-soon">
          {t("module.coming_soon", { phase: manifest.phase })}
        </p>
        <p className="text-xs text-muted">{t("module.replaces", { tools: manifest.replaces.join(", ") })}</p>
      </Card>
    </div>
  );
}
