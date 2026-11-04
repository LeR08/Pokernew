import { useCallback, useEffect } from "react";
import { HashRouter, Navigate, Route, Routes, useNavigate } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useConfigEvents } from "@/lib/config";
import { getBackend, type ConfigEvent } from "@/lib/backend";
import { AppearanceSync } from "@/theme/AppearanceSync";
import { AppShell, WindowShell } from "@/shell/AppShell";
import { Launcher } from "@/shell/Launcher";
import { ModulePage } from "@/shell/ModulePage";
import { SettingsCentre } from "@/settings/SettingsCentre";
import { Welcome } from "@/shell/Welcome";
import { HudOverlay } from "@/hud/HudOverlay";
import { usePlayModeSync } from "@/stores/playMode";

let readySent = false;

/** Routes asked by the backend (e.g. a HUD double-click opens a player's sheet). */
function NavigateListener() {
  const navigate = useNavigate();
  useEffect(() => {
    let off: (() => void) | undefined;
    let alive = true;
    void getBackend().then((b) => {
      if (alive) off = b.onNavigate((path) => navigate(path));
    });
    return () => {
      alive = false;
      off?.();
    };
  }, [navigate]);
  return null;
}

function Root() {
  const onEvent = useCallback((e: ConfigEvent) => {
    if (e.kind === "invalid") console.warn("invalid config", e);
  }, []);
  useConfigEvents(onEvent);
  usePlayModeSync();
  useEffect(() => {
    // Once per page load (StrictMode runs effects twice in development).
    if (readySent) return;
    readySent = true;
    void getBackend().then((b) => b.markReady());
  }, []);
  return (
    <>
      <AppearanceSync />
      <HashRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <NavigateListener />
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<Launcher />} />
            <Route path="m/:id/*" element={<ModulePage />} />
            <Route path="settings/:tab?" element={<SettingsCentre />} />
          </Route>
          <Route path="welcome" element={<Welcome />} />
          <Route path="hud/:id" element={<HudOverlay />} />
          <Route path="window/:id/*" element={<WindowShell />}>
            <Route path="*" element={<ModulePage />} />
          </Route>
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </HashRouter>
    </>
  );
}

export function App({ client }: { client?: QueryClient }) {
  const qc = client ?? new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return (
    <QueryClientProvider client={qc}>
      <Root />
    </QueryClientProvider>
  );
}
