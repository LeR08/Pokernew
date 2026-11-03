import { Trophy } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "icm",
  name: "modules.icm.name",
  description: "modules.icm.description",
  icon: Trophy,
  phase: 5,
  replaces: ["ICMIZER", "HRC"],
  routes: [{ path: "", component: ComingSoon }],
  studyOnly: true,
};
