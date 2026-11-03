import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import fr from "@/i18n/locales/fr.json";
import en from "@/i18n/locales/en.json";
import { allModules } from "@/modules/registry";

type Tree = { [k: string]: string | Tree };

function flatten(t: Tree, prefix = ""): string[] {
  return Object.entries(t).flatMap(([k, v]) =>
    typeof v === "string" ? [`${prefix}${k}`] : flatten(v, `${prefix}${k}.`),
  );
}

const frKeys = new Set(flatten(fr as Tree));
const enKeys = new Set(flatten(en as Tree));

function walk(dir: string, ext: RegExp): string[] {
  return readdirSync(dir).flatMap((n) => {
    const p = join(dir, n);
    return statSync(p).isDirectory() ? walk(p, ext) : ext.test(n) ? [p] : [];
  });
}

describe("i18n", () => {
  it("French and English have the same keys", () => {
    expect([...frKeys].filter((k) => !enKeys.has(k))).toEqual([]);
    expect([...enKeys].filter((k) => !frKeys.has(k))).toEqual([]);
  });

  it("every module name and description is translated", () => {
    for (const m of allModules) {
      expect(frKeys.has(m.name), m.name).toBe(true);
      expect(frKeys.has(m.description), m.description).toBe(true);
    }
  });

  it("every schema title/description is translated", () => {
    const dir = join(__dirname, "../../../config/defaults/schemas");
    const missing: string[] = [];
    for (const f of readdirSync(dir)) {
      const text = readFileSync(join(dir, f), "utf8");
      for (const m of text.matchAll(/"(?:title|description)":\s*"([^"]+)"/g)) {
        if (!frKeys.has(m[1])) missing.push(`${f}: ${m[1]}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("no hard-coded text between JSX tags", () => {
    const offenders: string[] = [];
    for (const file of walk(join(__dirname, ".."), /\.tsx$/)) {
      if (file.includes(".test.")) continue;
      const src = readFileSync(file, "utf8");
      // Text node made of letters directly between > and < (ignores {expressions}).
      for (const m of src.matchAll(/>\s*([A-Za-zÀ-ÿ][^<>{}();=&|]*[A-Za-zÀ-ÿ.!?])\s*</g)) {
        offenders.push(`${file.split("/src/")[1]}: "${m[1].trim()}"`);
      }
    }
    expect(offenders).toEqual([]);
  });
});
