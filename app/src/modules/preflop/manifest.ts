import { Grid3x3 } from "lucide-react";
import type { ModuleManifest } from "../types";
import { PreflopPage } from "./PreflopPage";

export const manifest: ModuleManifest = {
  id: "preflop",
  name: "modules.preflop.name",
  description: "modules.preflop.description",
  icon: Grid3x3,
  phase: 7,
  replaces: ["GTO Wizard"],
  routes: [
    { path: "", component: PreflopPage },
    { path: ":tab", component: PreflopPage },
  ],
  studyOnly: true,
};
