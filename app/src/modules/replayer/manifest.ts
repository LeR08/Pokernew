import { Play } from "lucide-react";
import type { ModuleManifest } from "../types";
import { ReplayerPage } from "./ReplayerPage";

export const manifest: ModuleManifest = {
  id: "replayer",
  name: "modules.replayer.name",
  description: "modules.replayer.description",
  icon: Play,
  phase: 2,
  replaces: ["PokerTracker 4", "Hand2Note"],
  routes: [
    { path: "", component: ReplayerPage },
    { path: ":handId", component: ReplayerPage },
  ],
};
