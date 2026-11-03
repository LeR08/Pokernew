import { LayoutDashboard } from "lucide-react";
import type { ModuleManifest } from "../types";
import { HudPage } from "./HudPage";

export const manifest: ModuleManifest = {
  id: "hud",
  name: "modules.hud.name",
  description: "modules.hud.description",
  icon: LayoutDashboard,
  phase: 3,
  replaces: ["PokerTracker 4", "Hand2Note", "Holdem Manager 3"],
  routes: [{ path: "", component: HudPage }],
};
