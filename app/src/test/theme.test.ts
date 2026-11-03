import { applyAppearance, effectiveThemeId } from "@/theme/apply";

describe("appearance", () => {
  it("writes theme tokens, density and font scale", () => {
    const el = document.createElement("html");
    applyAppearance(el, { name: "x", base: "light", colors: { background: "#ffffff", primary: "#123456" } }, "compact", 1.25);
    expect(el.style.getPropertyValue("--c-background")).toBe("#ffffff");
    expect(el.style.getPropertyValue("--c-primary")).toBe("#123456");
    expect(el.dataset.theme).toBe("light");
    expect(el.style.getPropertyValue("--density")).toBe("0.75");
    expect(el.style.fontSize).toBe("20px");
  });

  it("clamps the font scale", () => {
    const el = document.createElement("html");
    applyAppearance(el, undefined, "comfortable", 9);
    expect(el.style.fontSize).toBe("24px");
  });

  it("lets the profile override the theme", () => {
    expect(effectiveThemeId("dark", "light")).toBe("light");
    expect(effectiveThemeId("dark", undefined)).toBe("dark");
  });
});
