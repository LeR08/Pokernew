/** Hotkey strings look like "Ctrl+Shift+K" (modifiers in any order, then one key). */
const MODS = ["Ctrl", "Alt", "Shift", "Meta"] as const;

export interface Combo {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  key: string;
}

export function parseHotkey(s: string): Combo | null {
  const parts = s.split("+");
  // "Ctrl++" means Ctrl and the "+" key.
  if (s.endsWith("++")) parts.splice(parts.length - 2, 2, "+");
  const key = parts.pop();
  if (!key) return null;
  const set = new Set(parts);
  if ([...set].some((m) => !MODS.includes(m as (typeof MODS)[number]))) return null;
  return { ctrl: set.has("Ctrl"), alt: set.has("Alt"), shift: set.has("Shift"), meta: set.has("Meta"), key: normKey(key) };
}

function normKey(k: string): string {
  return k.length === 1 ? k.toUpperCase() : k;
}

/** True when the keyboard event matches the hotkey string. */
export function matches(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "key">, hotkey: string): boolean {
  const c = parseHotkey(hotkey);
  if (!c) return false;
  return (
    c.ctrl === e.ctrlKey && c.alt === e.altKey && c.shift === e.shiftKey && c.meta === e.metaKey && c.key === normKey(e.key)
  );
}

/** Formats a keyboard event as a hotkey string, or null for a bare modifier press. */
export function formatEvent(e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "key">): string | null {
  if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Meta");
  parts.push(e.key === " " ? "Space" : normKey(e.key));
  return parts.join("+");
}
