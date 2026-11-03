import { Percent } from "lucide-react";
import type { ModuleManifest } from "../types";
import { EquityPage } from "./EquityPage";

export const manifest: ModuleManifest = {
  id: "equity",
  name: "modules.equity.name",
  description: "modules.equity.description",
  icon: Percent,
  phase: 4,
  replaces: ["Equilab", "PokerStove", "Flopzilla"],
  routes: [
    { path: "", component: EquityPage },
    { path: ":tab", component: EquityPage },
  ],
  // Fair play: equity during a hand is live assistance — locked while a table is open.
  studyOnly: true,
};
