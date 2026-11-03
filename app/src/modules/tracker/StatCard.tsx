import { useTranslation } from "react-i18next";
import { Info } from "lucide-react";
import { Card } from "@/components/ui/card";
import { colourClass, formatStat, int, text } from "@/lib/format";
import type { StatMeta, StatValue } from "@/lib/tracker";
import { cn } from "@/lib/utils";

/** A stat with its value, sample size and plain-language explanation. */
export function StatCard({ meta, value }: { meta: StatMeta; value: StatValue | undefined }) {
  const { t, i18n } = useTranslation();
  const lang = i18n.language;
  const small = meta.minSample !== null && (value?.sample ?? 0) < meta.minSample;
  const desc = text(meta.description, lang);
  return (
    <Card className="flex flex-col gap-d1 p-d3" data-testid={`stat-${meta.id}`}>
      <div className="flex items-center justify-between gap-d2 text-xs text-muted">
        <span className="font-medium uppercase tracking-wide">{meta.name}</span>
        {desc && (
          <span title={desc} aria-label={desc} className="cursor-help">
            <Info className="h-3.5 w-3.5" />
          </span>
        )}
      </div>
      <span className={cn("text-2xl font-semibold tabular-nums", colourClass(value?.value, meta.colourRanges), small && "opacity-50")}>
        {formatStat(value?.value, meta.format, lang)}
      </span>
      <span className="text-xs text-muted">{t("tracker.sample", { n: int(value?.sample ?? 0, lang) })}</span>
      {desc && <p className="text-xs leading-snug text-muted">{desc}</p>}
    </Card>
  );
}
