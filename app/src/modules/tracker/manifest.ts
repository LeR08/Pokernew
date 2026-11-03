import { ChartLine } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "tracker",
  name: "modules.tracker.name",
  description: "modules.tracker.description",
  icon: ChartLine,
  phase: 2,
  replaces: ["PokerTracker 4", "Holdem Manager 3", "DriveHUD", "Poker Copilot"],
  routes: [{ path: "", component: ComingSoon }],
};
