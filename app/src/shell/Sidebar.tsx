import { NavLink } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Home, Settings as SettingsIcon } from "lucide-react";
import { orderedModules } from "@/modules/registry";
import { useSettings } from "@/lib/config";
import { useUi } from "@/stores/ui";
import { cn } from "@/lib/utils";
import { ProfileSwitcher } from "./ProfileSwitcher";

const linkCls = ({ isActive }: { isActive: boolean }) =>
  cn(
    "flex items-center gap-d3 rounded-md px-d3 py-d2 text-sm transition hover:bg-surface-alt",
    isActive ? "bg-surface-alt font-medium text-fg" : "text-muted",
  );

export function Sidebar() {
  const { t } = useTranslation();
  const settings = useSettings().data;
  const open = useUi((s) => s.sidebarOpen);
  if (!open || !settings) return null;
  const modules = orderedModules(settings.modules.order, settings.modules.disabled);
  return (
    <nav aria-label={t("app.title")} className="flex w-60 shrink-0 flex-col gap-d1 border-r border-border bg-surface p-d3">
      <div className="px-d3 pb-d3 pt-d1 text-sm font-semibold tracking-wide">{t("app.title")}</div>
      <NavLink to="/" end className={linkCls}>
        <Home className="h-4 w-4" aria-hidden />
        {t("nav.home")}
      </NavLink>
      <div className="my-d2 border-t border-border" />
      <div className="flex flex-1 flex-col gap-d1 overflow-y-auto" data-testid="sidebar-modules">
        {modules.map((m) => (
          <NavLink key={m.id} to={`/m/${m.id}`} className={linkCls} data-module={m.id}>
            <m.icon className="h-4 w-4" aria-hidden />
            {t(m.name)}
          </NavLink>
        ))}
      </div>
      <div className="my-d2 border-t border-border" />
      <ProfileSwitcher />
      <NavLink to="/settings" className={linkCls}>
        <SettingsIcon className="h-4 w-4" aria-hidden />
        {t("nav.settings")}
      </NavLink>
    </nav>
  );
}
