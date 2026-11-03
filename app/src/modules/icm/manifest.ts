import { Trophy } from "lucide-react";
import type { ModuleManifest } from "../types";
import { IcmPage } from "./IcmPage";

export const manifest: ModuleManifest = {
  id: "icm",
  name: "modules.icm.name",
  description: "modules.icm.description",
  icon: Trophy,
  phase: 5,
  replaces: ["ICMIZER", "HRC"],
  routes: [
    { path: "", component: IcmPage },
    { path: ":tab", component: IcmPage },
  ],
  studyOnly: true,
};
