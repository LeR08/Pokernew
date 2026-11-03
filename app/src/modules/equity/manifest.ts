import { Percent } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "equity",
  name: "modules.equity.name",
  description: "modules.equity.description",
  icon: Percent,
  phase: 4,
  replaces: ["Equilab", "PokerStove", "Flopzilla"],
  routes: [{ path: "", component: ComingSoon }],
};
