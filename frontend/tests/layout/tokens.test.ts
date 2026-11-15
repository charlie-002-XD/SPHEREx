// Design-token guards: WCAG AA contrast of every text/background token
// pair in both color schemes (computed from tokens.css itself), and "tokens
// only" in component CSS (no literal colors outside tokens.css).
// @ts-expect-error -- @types/node is not installed; vitest resolves node
// builtins at runtime, so readFileSync works in this environment.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const SRC = new URL("../../src/", import.meta.url).pathname;
const TOKENS = readFileSync(`${SRC}styles/tokens.css`, "utf8");

type Rgb = [number, number, number];

function block(css: string, marker: string): string {
  const start = css.indexOf(marker);
  if (start < 0) throw new Error(`missing ${marker}`);
  const open = css.indexOf("{", css.indexOf(":root", start));
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}") depth--;
    if (depth === 0) return css.slice(open + 1, i);
  }
  throw new Error("unbalanced");
}

function vars(body: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const m of body.matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) {
    out.set(m[1] ?? "", (m[2] ?? "").trim());
  }
  return out;
}

const DARK = vars(block(TOKENS, ":root {"));
function bodyAt(css: string, at: number): string {
  if (at < 0) throw new Error("missing block");
  const open = css.indexOf("{", at);
  let depth = 0;
  for (let i = open; i < css.length; i++) {
    if (css[i] === "{") depth++;
    if (css[i] === "}") depth--;
    if (depth === 0) return css.slice(open + 1, i);
  }
  throw new Error("unbalanced");
}

// .app-sky: dark block first, light override inside the last media query.
const SKY = new Map([
  ...DARK,
  ...vars(bodyAt(TOKENS, TOKENS.indexOf(".app-sky {"))),
]);
const LIGHT = new Map([
  ...DARK,
  ...vars(block(TOKENS, "@media (prefers-color-scheme: light)")),
]);
const SKY_LIGHT = new Map([
  ...LIGHT,
  ...SKY,
  ...vars(bodyAt(TOKENS, TOKENS.lastIndexOf(".app-sky {"))),
]);

function hex(v: string | undefined): Rgb {
  const m = /^#([0-9a-f]{6})$/i.exec(v ?? "");
  if (m === null) throw new Error(`not a 6-digit hex color: ${v}`);
  const n = parseInt(m[1] ?? "0", 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function lum([r, g, b]: Rgb): number {
  const c = [r, g, b].map((x) => {
    const s = x / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (c[0] ?? 0) + 0.7152 * (c[1] ?? 0) + 0.0722 * (c[2] ?? 0);
}

export function contrast(a: Rgb, b: Rgb): number {
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
}

// Text on surfaces (4.5:1, normal-size text).
const TEXT_PAIRS: Array<[string, string]> = [
  ["fg", "bg"],
  ["fg", "bg-raised"],
  ["fg", "bg-inset"],
  ["fg", "bg-hover"],
  ["fg-dim", "bg"],
  ["fg-dim", "bg-raised"],
  ["fg-dim", "bg-inset"],
  ["fg-dim", "bg-hover"],
  ["fg-faint", "bg"],
  ["fg-faint", "bg-raised"],
  ["fg-faint", "bg-inset"],
  ["accent-fg", "accent"],
  ["fg", "accent-soft"],
  ["accent", "accent-soft"],
  ["accent", "bg"],
  ["accent", "bg-raised"],
  ["danger", "bg"],
  ["danger", "bg-raised"],
  ["danger", "bg-inset"],
  ["warn", "bg"],
  ["warn", "bg-raised"],
];

// Non-text UI that must be perceivable (3:1): focus ring, slider thumb,
// and the borders of interactive controls on every surface (WCAG 1.4.11).
const UI_PAIRS: Array<[string, string]> = [
  ["focus", "bg"],
  ["focus", "bg-raised"],
  ["fg", "bg"], // slider thumb
  ["border-strong", "bg"],
  ["border-strong", "bg-raised"],
  ["border-strong", "bg-inset"],
  ["border-strong", "bg-hover"],
];

describe.each([
  ["dark", DARK],
  ["light", LIGHT],
  ["sky", SKY],
  ["sky light", SKY_LIGHT],
])("%s scheme contrast", (_name, t) => {
  it.each(TEXT_PAIRS)("%s on %s >= 4.5:1", (fg, bg) => {
    expect(contrast(hex(t.get(fg)), hex(t.get(bg)))).toBeGreaterThanOrEqual(
      4.5,
    );
  });
  it.each(UI_PAIRS)("%s on %s >= 3:1", (fg, bg) => {
    expect(contrast(hex(t.get(fg)), hex(t.get(bg)))).toBeGreaterThanOrEqual(3);
  });
});

describe("image overlay legibility", () => {
  // Chips sit on rgb(0 0 0 / 0.7) over arbitrary pixels; the worst case is
  // a pure white pixel underneath: 0.3 * 255 = 77 gray.
  it("scrim text stays >= 4.5:1 over a white pixel", () => {
    const alpha = Number(/\/\s*([\d.]+)\)/.exec(DARK.get("scrim") ?? "")?.[1]);
    expect(alpha).toBeGreaterThan(0);
    const g = Math.round(255 * (1 - alpha));
    const under: Rgb = [g, g, g];
    expect(contrast(hex(DARK.get("scrim-fg")), under)).toBeGreaterThanOrEqual(
      4.5,
    );
    expect(contrast(hex(DARK.get("scrim-warn")), under)).toBeGreaterThanOrEqual(
      4.5,
    );
  });
});

describe("tokens only", () => {
  const files = [
    "styles/base.css",
    "components/layout/app-shell.css",
    "components/viewer/viewer.css",
    "components/controls/controls.css",
    "components/spectrum/spectrum.css",
    "components/metadata/metadata.css",
    "components/overlays/overlays.css",
    "components/compare/compare.css",
    "components/layout/sky.css",
  ];
  it.each(files)("%s has no literal colors", (f) => {
    const css = readFileSync(`${SRC}${f}`, "utf8").replace(
      /\/\*[\s\S]*?\*\//g,
      "",
    );
    expect(css).not.toMatch(/#[0-9a-f]{3,8}\b/i);
    expect(css).not.toMatch(/\brgba?\(/i);
  });

  it("uses the 4-step type scale only (12/14/16/20 px)", () => {
    expect(TOKENS).toContain("--text-xs: 0.75rem");
    expect(TOKENS).toContain("--text-sm: 0.875rem");
    expect(TOKENS).toContain("--text-md: 1rem");
    expect(TOKENS).toContain("--text-lg: 1.25rem");
    expect(TOKENS).not.toContain("--text-xl");
  });
});
