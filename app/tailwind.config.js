/** Colours come from the active theme file (themes/*.toml) as CSS variables. */
const token = (name) => `var(--c-${name})`;

/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        bg: token("background"),
        surface: token("surface"),
        "surface-alt": token("surface_alt"),
        border: token("border"),
        fg: token("text"),
        muted: token("text_muted"),
        primary: token("primary"),
        "primary-fg": token("primary_text"),
        accent: token("accent"),
        success: token("success"),
        warning: token("warning"),
        danger: token("danger"),
      },
      spacing: {
        // Density-aware spacing: compact = 0.75x, comfortable = 1x.
        d1: "calc(0.25rem * var(--density))",
        d2: "calc(0.5rem * var(--density))",
        d3: "calc(0.75rem * var(--density))",
        d4: "calc(1rem * var(--density))",
        d6: "calc(1.5rem * var(--density))",
      },
    },
  },
  plugins: [],
};
