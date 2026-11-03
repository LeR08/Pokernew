import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { AlertTriangle } from "lucide-react";
import { useIssues } from "@/lib/config";

/** Lists invalid config files (their last valid version stays active). */
export function IssuesBanner() {
  const { t } = useTranslation();
  const issues = useIssues().data ?? [];
  if (issues.length === 0) return null;
  return (
    <div role="alert" data-testid="issues-banner" className="m-d4 rounded-md border border-warning bg-surface p-d3 text-sm">
      <div className="mb-d1 flex items-center gap-d2 font-medium text-warning">
        <AlertTriangle className="h-4 w-4" aria-hidden />
        <Link to="/settings">{t("settings.issues_title")}</Link>
      </div>
      <ul className="list-inside list-disc text-muted">
        {issues.map((i, n) => (
          <li key={n}>
            <code>
              {i.family}/{i.id}
              {i.pointer}
            </code>{" "}
            — {i.message}
          </li>
        ))}
      </ul>
    </div>
  );
}
