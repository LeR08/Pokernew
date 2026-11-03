import { Database } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "import",
  name: "modules.import.name",
  description: "modules.import.description",
  icon: Database,
  phase: 1,
  replaces: ["PokerTracker 4", "Holdem Manager 3"],
  routes: [{ path: "", component: ComingSoon }],
};
