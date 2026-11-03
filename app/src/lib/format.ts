import type { ColourRange, I18nText } from "./tracker";

export function text(t: I18nText | null | undefined, lang: string): string {
  if (!t) return "";
  if (typeof t === "string") return t;
  return lang === "fr" ? t.fr : t.en;
}

const nf = (lang: string, digits: number) =>
  new Intl.NumberFormat(lang === "fr" ? "fr-FR" : "en-GB", { minimumFractionDigits: digits, maximumFractionDigits: digits });

/** Formats a stat value according to its `format`. */
export function formatStat(v: number | null | undefined, format: string, lang: string): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  switch (format) {
    case "pct0":
      return `${nf(lang, 0).format(v * 100)} %`;
    case "pct1":
      return `${nf(lang, 1).format(v * 100)} %`;
    case "dec1":
      return nf(lang, 1).format(v);
    case "dec2":
      return nf(lang, 2).format(v);
    case "int":
      return nf(lang, 0).format(v);
    case "bb100":
      return `${v > 0 ? "+" : ""}${nf(lang, 1).format(v)}`;
    default:
      return nf(lang, 2).format(v);
  }
}

export function euros(cents: number, lang: string, signed = true): string {
  const s = new Intl.NumberFormat(lang === "fr" ? "fr-FR" : "en-GB", { style: "currency", currency: "EUR" }).format(cents / 100);
  return signed && cents > 0 ? `+${s}` : s;
}

export function bb(v: number, lang: string, signed = true): string {
  const s = nf(lang, 1).format(v);
  return `${signed && v > 0 ? "+" : ""}${s} bb`;
}

export function int(v: number, lang: string): string {
  return nf(lang, 0).format(v);
}

/** CSS class for a stat value from its colour ranges (first match wins). */
export function colourClass(v: number | null | undefined, ranges: ColourRange[]): string {
  if (v === null || v === undefined) return "";
  const r = ranges.find((x) => (x.min === undefined || v >= x.min) && (x.max === undefined || v < x.max));
  const map: Record<string, string> = { red: "text-danger", green: "text-success", blue: "text-primary", orange: "text-warning", yellow: "text-warning" };
  return r ? (map[r.colour] ?? "") : "";
}

/** Text colour for a result: green when winning, red when losing. */
export const resultClass = (v: number) => (v > 0 ? "text-success" : v < 0 ? "text-danger" : "");
