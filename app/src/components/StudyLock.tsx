import { useTranslation } from "react-i18next";
import { Lock } from "lucide-react";
import { Card } from "@/components/ui/card";

/** Shown instead of a study tool while a poker table is open. */
export function StudyLock() {
  const { t } = useTranslation();
  return (
    <Card className="mx-auto flex max-w-xl flex-col items-center gap-d3 p-d6 text-center" data-testid="study-lock" role="status">
      <Lock className="h-8 w-8 text-warning" aria-hidden />
      <h2 className="text-lg font-semibold">{t("playmode.title")}</h2>
      <p className="text-sm text-muted">{t("playmode.why")}</p>
    </Card>
  );
}
