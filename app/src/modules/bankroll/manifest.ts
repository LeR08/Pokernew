import { Wallet } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "bankroll",
  name: "modules.bankroll.name",
  description: "modules.bankroll.description",
  icon: Wallet,
  phase: 2,
  replaces: ["Poker Copilot", "Xeester", "Poker Analytics"],
  routes: [{ path: "", component: ComingSoon }],
};
