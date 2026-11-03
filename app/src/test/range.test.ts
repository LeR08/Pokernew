import cases from "../../../crates/equity/data/range_cases.json";
import {
  CELL_COMBOS,
  cellName,
  comboCount,
  formatRange,
  liveCount,
  parseCards,
  parseRange,
  RANKED_CELLS,
  topPercent,
  tryParseRange,
} from "@/lib/range";

describe("range notation (shared cases with crates/equity)", () => {
  it.each(cases as { text: string; combos?: number }[])("$text", ({ text, combos }) => {
    if (combos === undefined) expect(tryParseRange(text)).toBeNull();
    else expect(comboCount(parseRange(text))).toBeCloseTo(combos, 4);
  });
});

describe("range model", () => {
  it("grid cells", () => {
    expect(cellName(0)).toBe("AA");
    expect(cellName(1)).toBe("AKs");
    expect(cellName(13)).toBe("AKo");
    expect(cellName(168)).toBe("22");
    expect(CELL_COMBOS.map((c) => c.length).reduce((a, b) => a + b)).toBe(1326);
    expect(new Set(RANKED_CELLS).size).toBe(169);
  });

  it("top percent", () => {
    expect(topPercent(0)).toHaveLength(0);
    expect(topPercent(100)).toHaveLength(1326);
    expect(topPercent(0.5)).toHaveLength(6);
  });

  it("formats compactly and round-trips", () => {
    const f = (s: string) => formatRange(parseRange(s));
    expect(f("TT+")).toBe("TT+");
    expect(f("99-66")).toBe("99-66");
    expect(f("ATs+")).toBe("ATs+");
    expect(f("A5s-A2s")).toBe("A5s-A2s");
    expect(f("AK")).toBe("AK");
    expect(f("AKs,AQo")).toBe("AKs,AQo");
    expect(f("KQo:50%")).toBe("KQo:50%");
    expect(f("AhKh")).toBe("AhKh");
    expect(f("random")).toBe("random");
    expect(f("")).toBe("");
    for (const s of ["22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+", "AA,KK:50%,AKs:25%,AhKd,AsKc:10%", "15%", "40%", "QQ+,AK,JhTh,98s:75%"]) {
      const w = parseRange(s);
      expect(Array.from(parseRange(formatRange(w)))).toEqual(Array.from(w));
    }
  });

  it("counts live combos", () => {
    expect(liveCount(parseRange("AK"), parseCards("Ah")!)).toBe(12);
    expect(parseCards("AhKd 7c")).toHaveLength(3);
    expect(parseCards("Ax")).toBeNull();
  });
});
