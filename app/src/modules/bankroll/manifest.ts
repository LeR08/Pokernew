import { Wallet } from "lucide-react";
import type { ModuleManifest } from "../types";
import { BankrollPage } from "./BankrollPage";

export const manifest: ModuleManifest = {
  id: "bankroll",
  name: "modules.bankroll.name",
  description: "modules.bankroll.description",
  icon: Wallet,
  phase: 2,
  replaces: ["Poker Copilot", "Xeester", "Poker Analytics"],
  routes: [{ path: "", component: BankrollPage }],
};
