import { createMockBackend } from "@/lib/backend.mock";
import { BackendError } from "@/lib/backend";

describe("mock backend (same schemas as Rust)", () => {
  it("serves every default file as valid", async () => {
    const b = createMockBackend();
    expect(await b.configList("themes")).toEqual(["bordeaux", "custom", "dark", "feutre", "graphite", "light", "ocean", "sable", "violet"]);
    expect(await b.configList("profiles")).toEqual(["cash-6max", "mtt", "spin"]);
    const s = await b.configGet("settings", "settings");
    expect((s.performance as Record<string, number>).worker_threads).toBe(6);
  });

  it("rejects invalid TOML and schema violations with pointers", async () => {
    const b = createMockBackend();
    expect((await b.configValidate("themes", "x", "name = "))[0].pointer).toBe("");
    const raw = (await b.configRaw("settings", "settings")).replace('density = "comfortable"', 'density = "huge"');
    const issues = await b.configValidate("settings", "settings", raw);
    expect(issues[0].pointer).toBe("/appearance/density");
    await expect(b.configSaveText("settings", "settings", raw)).rejects.toBeInstanceOf(BackendError);
  });

  it("emits change events on save", async () => {
    const b = createMockBackend();
    const events: string[] = [];
    b.onConfigEvent((e) => events.push(`${e.kind}:${e.family}/${e.id}`));
    const s = await b.configGet("settings", "settings");
    await b.configSaveValue("settings", "settings", s);
    expect(events).toEqual(["changed:settings/settings"]);
  });
});
