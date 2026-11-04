import { GraduationCap } from "lucide-react";
import type { ModuleManifest } from "../types";
import { TrainerPage } from "./TrainerPage";

export const manifest: ModuleManifest = {
  id: "trainer",
  name: "modules.trainer.name",
  description: "modules.trainer.description",
  icon: GraduationCap,
  phase: 8,
  replaces: ["GTO Wizard", "PokerSnowie"],
  routes: [
    { path: "", component: TrainerPage },
    { path: ":tab", component: TrainerPage },
  ],
  // Grades: EV lost in % of the pot (above "mistake" it is a blunder).
  settingsSchema: {
    type: "object",
    title: "trainer.settings.title",
    additionalProperties: false,
    properties: {
      correct: { type: "number", title: "trainer.settings.correct", minimum: 0, maximum: 100 },
      inaccuracy: { type: "number", title: "trainer.settings.inaccuracy", minimum: 0, maximum: 100 },
      mistake: { type: "number", title: "trainer.settings.mistake", minimum: 0, maximum: 100 },
    },
  },
  settingsDefaults: { correct: 1, inaccuracy: 3, mistake: 8 },
  studyOnly: true,
};
