import { LayoutDashboard } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "hud",
  name: "modules.hud.name",
  description: "modules.hud.description",
  icon: LayoutDashboard,
  phase: 3,
  replaces: ["PokerTracker 4", "Hand2Note", "Holdem Manager 3"],
  routes: [{ path: "", component: ComingSoon }],
};
