import { formatEvent, matches, parseHotkey } from "@/hotkeys/keys";

const ev = (key: string, mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {}) => ({
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("hotkeys", () => {
  it("parses combos", () => {
    expect(parseHotkey("Ctrl+Shift+K")).toEqual({ ctrl: true, alt: false, shift: true, meta: false, key: "K" });
    expect(parseHotkey("Ctrl++")?.key).toBe("+");
    expect(parseHotkey("Hyper+K")).toBeNull();
  });

  it("matches keyboard events exactly", () => {
    expect(matches(ev("h", { ctrlKey: true }), "Ctrl+H")).toBe(true);
    expect(matches(ev(",", { ctrlKey: true }), "Ctrl+,")).toBe(true);
    expect(matches(ev("h", { ctrlKey: true, shiftKey: true }), "Ctrl+H")).toBe(false);
    expect(matches(ev("h"), "Ctrl+H")).toBe(false);
  });

  it("formats events for the capture widget", () => {
    expect(formatEvent(ev("k", { ctrlKey: true, altKey: true }))).toBe("Ctrl+Alt+K");
    expect(formatEvent(ev("Control", { ctrlKey: true }))).toBeNull();
    expect(formatEvent(ev("F5"))).toBe("F5");
  });
});
