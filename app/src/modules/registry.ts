import type { ModuleManifest } from "./types";

/** Auto-discovers every `./<id>/manifest.ts`. */
const found = import.meta.glob<{ manifest: ModuleManifest }>("./*/manifest.ts", { eager: true });

export function validateManifests(list: ModuleManifest[]): string[] {
  const errors: string[] = [];
  const seen = new Set<string>();
  for (const m of list) {
    if (!/^[a-z][a-z0-9-]*$/.test(m.id)) errors.push(`bad module id '${m.id}'`);
    if (seen.has(m.id)) errors.push(`duplicate module id '${m.id}'`);
    seen.add(m.id);
    if (m.routes.length === 0) errors.push(`module '${m.id}' has no route`);
  }
  return errors;
}

export const allModules: ModuleManifest[] = Object.entries(found)
  .map(([path, mod]) => {
    const folder = path.split("/")[1];
    if (mod.manifest.id !== folder) {
      throw new Error(`module folder '${folder}' exports manifest id '${mod.manifest.id}'`);
    }
    return mod.manifest;
  })
  .sort((a, b) => a.phase - b.phase || a.id.localeCompare(b.id));

const problems = validateManifests(allModules);
if (problems.length) throw new Error(problems.join("; "));

export const moduleById = (id: string): ModuleManifest | undefined => allModules.find((m) => m.id === id);

/**
 * Applies the user's order/disabled lists. Modules missing from `order`
 * (e.g. newly added) are appended in default order.
 */
export function orderedModules(order: string[], disabled: string[], includeDisabled = false): ModuleManifest[] {
  const rank = new Map(order.map((id, i) => [id, i]));
  return [...allModules]
    .filter((m) => includeDisabled || !disabled.includes(m.id))
    .sort((a, b) => (rank.get(a.id) ?? 1e6 + a.phase) - (rank.get(b.id) ?? 1e6 + b.phase));
}
