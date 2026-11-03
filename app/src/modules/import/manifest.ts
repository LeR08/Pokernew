import { Database } from "lucide-react";
import type { ModuleManifest } from "../types";
import { ImportPage } from "./ImportPage";

export const manifest: ModuleManifest = {
  id: "import",
  name: "modules.import.name",
  description: "modules.import.description",
  icon: Database,
  phase: 1,
  replaces: ["PokerTracker 4", "Holdem Manager 3"],
  routes: [{ path: "", component: ImportPage }],
};
