import { GraduationCap } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "trainer",
  name: "modules.trainer.name",
  description: "modules.trainer.description",
  icon: GraduationCap,
  phase: 8,
  replaces: ["GTO Wizard", "PokerSnowie"],
  routes: [{ path: "", component: ComingSoon }],
  studyOnly: true,
};
