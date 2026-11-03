import { readdirSync } from "node:fs";
import { join } from "node:path";
import { allModules, orderedModules, validateManifests } from "@/modules/registry";

describe("module registry", () => {
  it("discovers one manifest per module folder", () => {
    const dir = join(__dirname, "../modules");
    const folders = readdirSync(dir, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => d.name)
      .sort();
    expect(allModules.map((m) => m.id).sort()).toEqual(folders);
    expect(folders).toHaveLength(10);
  });

  it("applies the user's order and hides disabled modules", () => {
    const ids = orderedModules(["solver", "import"], ["tracker"]).map((m) => m.id);
    expect(ids.slice(0, 2)).toEqual(["solver", "import"]);
    expect(ids).not.toContain("tracker");
    // Modules missing from the order list are appended, none is lost.
    expect(ids).toHaveLength(9);
    expect(orderedModules([], ["tracker"], true)).toHaveLength(10);
  });

  it("rejects duplicate or malformed ids", () => {
    const m = allModules[0];
    expect(validateManifests([m, m])).toEqual([`duplicate module id '${m.id}'`]);
    expect(validateManifests([{ ...m, id: "Bad Id" }])[0]).toMatch(/bad module id/);
  });

  it("marks study modules (fair-play lock from phase 3)", () => {
    const study = allModules.filter((m) => m.studyOnly).map((m) => m.id).sort();
    expect(study).toEqual(["equity", "icm", "preflop", "solver", "trainer"]);
  });
});
