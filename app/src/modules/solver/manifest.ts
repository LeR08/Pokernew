import { Brain } from "lucide-react";
import type { ModuleManifest } from "../types";
import { SolverPage } from "./SolverPage";

export const manifest: ModuleManifest = {
  id: "solver",
  name: "modules.solver.name",
  description: "modules.solver.description",
  icon: Brain,
  phase: 6,
  replaces: ["PioSOLVER", "GTO+", "TexasSolver"],
  routes: [
    { path: "", component: SolverPage },
    { path: ":tab", component: SolverPage },
  ],
  studyOnly: true,
};
