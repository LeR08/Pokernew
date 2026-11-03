import { Play } from "lucide-react";
import { ComingSoon } from "../ComingSoon";
import type { ModuleManifest } from "../types";

export const manifest: ModuleManifest = {
  id: "replayer",
  name: "modules.replayer.name",
  description: "modules.replayer.description",
  icon: Play,
  phase: 2,
  replaces: ["PokerTracker 4", "Hand2Note"],
  routes: [{ path: "", component: ComingSoon }],
};
