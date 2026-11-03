import { Grid3x3 } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "preflop",
  name: "modules.preflop.name",
  description: "modules.preflop.description",
  icon: Grid3x3,
  phase: 7,
  replaces: ["GTO Wizard"],
  routes: [{ path: "", component: ComingSoon }],
  studyOnly: true,
};
