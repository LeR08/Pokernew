import { Navigate, Outlet } from "react-router-dom";
import { useSettings } from "@/lib/config";
import { useAppHotkeys } from "@/hotkeys/useAppHotkeys";
import { Sidebar } from "./Sidebar";
import { IssuesBanner } from "./IssuesBanner";

export function AppShell() {
  useAppHotkeys();
  const settings = useSettings().data;
  if (settings && !settings.general.first_run_done) return <Navigate to="/welcome" replace />;
  return (
    <div className="flex h-screen overflow-hidden bg-bg text-fg">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <IssuesBanner />
        <Outlet />
      </main>
    </div>
  );
}

/** Bare layout for a module opened in its own window. */
export function WindowShell() {
  return (
    <div className="h-screen overflow-y-auto bg-bg text-fg">
      <Outlet />
    </div>
  );
}
