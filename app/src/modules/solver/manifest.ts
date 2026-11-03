import { Brain } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "solver",
  name: "modules.solver.name",
  description: "modules.solver.description",
  icon: Brain,
  phase: 6,
  replaces: ["PioSOLVER", "GTO+", "TexasSolver"],
  routes: [{ path: "", component: ComingSoon }],
  studyOnly: true,
};
