import i18n from "i18next";

const nf = (digits: number) =>
  new Intl.NumberFormat(i18n.language === "fr" ? "fr-FR" : "en-GB", { minimumFractionDigits: digits, maximumFractionDigits: digits });

/** 0.4567 → "45,67 %" (fr) / "45.67 %" (en). */
export const pct = (x: number, digits = 2) => `${nf(digits).format(x * 100)} %`;

/** Weighted combo counts: integers as is, otherwise one decimal. */
export const combos = (n: number) => (Number.isInteger(n) ? String(n) : nf(1).format(n));

/** Integer with thousands separators. */
export const count = (n: number) => nf(0).format(n);
