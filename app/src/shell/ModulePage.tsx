import { Route, Routes, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { moduleById } from "@/modules/registry";
import { useSettings } from "@/lib/config";

/** Mounts a module's routes under /m/<id>/* (or /window/<id>/* in its own window). */
export function ModulePage() {
  const { t } = useTranslation();
  const { id = "" } = useParams();
  const settings = useSettings().data;
  const m = moduleById(id);
  if (!m) return <div className="p-d6 text-danger">{t("module.not_found", { id })}</div>;
  if (settings?.modules.disabled.includes(id)) return <div className="p-d6 text-muted">{t("module.disabled")}</div>;
  return (
    <Routes>
      {m.routes.map((r) => (
        <Route key={r.path} path={r.path === "" ? "/" : r.path} element={<r.component manifest={m} />} />
      ))}
    </Routes>
  );
}
