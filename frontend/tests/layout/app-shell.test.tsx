// Component tests for the app shell and the App composition. The
// environment has no jsdom, so assertions run on renderToStaticMarkup
// output plus static CSS checks (breakpoint, touch-target and overflow
// guards). matchMedia and window are stubbed per test.
// @ts-expect-error -- @types/node is not installed; vitest resolves node
// builtins at runtime, so readFileSync works in this environment.
import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { AppShell } from "../../src/components/layout/AppShell";
import {
  EmptyState,
  ErrorState,
  LoadingSkeleton,
} from "../../src/components/layout/States";
import { App } from "../../src/App";

const SRC = new URL("../../src/", import.meta.url).pathname;
const SHELL_CSS = readFileSync(`${SRC}components/layout/app-shell.css`, "utf8");
const TOKENS_CSS = readFileSync(`${SRC}styles/tokens.css`, "utf8");
const BASE_CSS = readFileSync(`${SRC}styles/base.css`, "utf8");

function stubMatchMedia(matches: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({
    matches,
    media: query,
    onchange: null,
    addEventListener() {},
    removeEventListener() {},
    addListener() {},
    removeListener() {},
    dispatchEvent: () => false,
  }));
}

function stubWindow(search: string) {
  vi.stubGlobal("window", {
    location: { search, pathname: "/" },
    history: { replaceState() {} },
    addEventListener() {},
    removeEventListener() {},
  });
}

function renderShell(embed = false) {
  return renderToStaticMarkup(
    <AppShell
      embed={embed}
      targets={[
        { id: "t1", label: "Target One", ra: "10.0", dec: "-20.0", fov: 0.05 },
      ]}
      onSelectTarget={() => {}}
      search={<div className="fake-search">search</div>}
      panel={<div className="fake-panel">panel</div>}
    >
      <div className="fake-viewer">viewer</div>
    </AppShell>,
  );
}

afterEach(() => vi.unstubAllGlobals());

describe("AppShell breakpoints", () => {
  it("opens the side panel by default on desktop (>= 640px)", () => {
    stubMatchMedia(true);
    const html = renderShell();
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain('id="app-panel-body"');
    expect(html).not.toContain('id="app-panel-body" hidden');
    expect(html).toContain("fake-panel");
  });

  it("keeps the bottom sheet closed by default on mobile (< 640px)", () => {
    stubMatchMedia(false);
    const html = renderShell();
    expect(html).toContain('aria-expanded="false"');
    expect(html).toContain('id="app-panel-body" hidden');
    expect(html).toContain("fake-panel");
  });

  it("renders the single-line header with name and target menu", () => {
    stubMatchMedia(true);
    const html = renderShell();
    expect(html).toContain("app-header");
    expect(html).toContain("SPHEREx Blink");
    expect(html).toContain("<summary");
    expect(html).toContain("Target One");
  });

  it("exposes the panel toggle as a button controlling the panel body", () => {
    stubMatchMedia(false);
    const html = renderShell();
    expect(html).toContain("<button");
    expect(html).toContain('aria-controls="app-panel-body"');
  });
});

describe("embed mode", () => {
  it("AppShell embed renders only the viewer area", () => {
    stubMatchMedia(true);
    const html = renderShell(true);
    expect(html).toContain("app-embed");
    expect(html).toContain("fake-viewer");
    expect(html).not.toContain("app-header");
    expect(html).not.toContain("app-search");
    expect(html).not.toContain("app-panel");
    expect(html).not.toContain("SPHEREx Blink");
  });

  it("App strips chrome when the URL has ?embed=1", () => {
    stubMatchMedia(false);
    stubWindow("?embed=1&ra=133.79&dec=-7.24");
    const html = renderToStaticMarkup(<App />);
    expect(html).toContain("app-embed");
    expect(html).not.toContain("app-header");
    expect(html).not.toContain("csb");
  });

  it("App renders the full shell without ?embed=1", () => {
    stubMatchMedia(false);
    stubWindow("?ra=133.79&dec=-7.24");
    const html = renderToStaticMarkup(<App />);
    expect(html).toContain("app-header");
    expect(html).toContain("csb");
    expect(html).toContain("app-sky");
    expect(html).toContain("sky-left");
    expect(html).toContain("sky-right");
    expect(html).toContain("state"); // empty/loading state placeholder
  });
});

describe("viewer states", () => {
  it("error state announces itself and offers Retry", () => {
    const html = renderToStaticMarkup(
      <ErrorState message="boom" onRetry={() => {}} />,
    );
    expect(html).toContain('role="alert"');
    expect(html).toContain("Retry");
    expect(html).toContain("boom");
  });

  it("loading state is a single skeleton block announced politely", () => {
    const html = renderToStaticMarkup(<LoadingSkeleton />);
    expect(html).toContain('role="status"');
    expect(html).toContain("skel");
    expect(html).toContain("Loading frames");
  });

  it("empty state explains how to load a time-lapse", () => {
    const html = renderToStaticMarkup(<EmptyState />);
    expect(html).toContain("Point at the sky");
  });
});

describe("responsive CSS guards (static assertions for 375px)", () => {
  it("uses a mobile-first 640px breakpoint for the side panel", () => {
    expect(SHELL_CSS).toContain("@media (min-width: 640px)");
    expect(SHELL_CSS).toContain("--panel-w");
  });

  it("keeps flex children shrinkable to avoid horizontal overflow", () => {
    // Without min-width:0, flex items refuse to shrink below content size
    // and overflow narrow viewports.
    expect(SHELL_CSS).toContain("min-width: 0");
    expect(SHELL_CSS).toContain("min-height: 0");
  });

  it("never fixes the shell to a width wider than a phone", () => {
    // No fixed px widths >= 375 in the base (mobile) layer; the only large
    // fixed width is --panel-w, applied inside the >=640px media query.
    const mobileLayer = SHELL_CSS.split("@media (min-width: 640px)")[0] ?? "";
    const fixed = mobileLayer.match(/(?<![a-z-])width:\s*(\d+)(px|rem)/g) ?? [];
    for (const rule of fixed) {
      const n = Number(rule.replace(/width:\s*/, "").replace(/(px|rem)/, ""));
      const px = rule.endsWith("rem") ? n * 16 : n;
      expect(px).toBeLessThanOrEqual(375);
    }
  });

  it("defines a 44px touch-target token and uses it", () => {
    expect(TOKENS_CSS).toContain("--target: 2.75rem");
    expect(SHELL_CSS).toContain("min-height: var(--target)");
  });

  it("honors prefers-color-scheme and prefers-reduced-motion", () => {
    expect(TOKENS_CSS).toContain("prefers-color-scheme: light");
    expect(BASE_CSS).toContain("prefers-reduced-motion: reduce");
    expect(BASE_CSS).toContain("focus-visible");
  });
});

// f6 visual-polish regressions: the four PM defects (D-016, D-019).
const SPECTRUM_CSS = readFileSync(
  `${SRC}components/spectrum/spectrum.css`,
  "utf8",
);
const CONTROLS_CSS = readFileSync(
  `${SRC}components/controls/controls.css`,
  "utf8",
);

function remToPx(v: string): number {
  const m = /([\d.]+)(rem|px)/.exec(v);
  if (m === null) return NaN;
  return m[2] === "rem" ? Number(m[1]) * 16 : Number(m[1]);
}

// Bodies of every @media / @container block (balanced braces).
function atRuleBlocks(css: string): string[] {
  const out: string[] = [];
  for (const m of css.matchAll(/@(?:media|container)[^{]*\{/g)) {
    let depth = 1;
    const start = (m.index ?? 0) + m[0].length;
    let i = start;
    for (; i < css.length && depth > 0; i++) {
      if (css[i] === "{") depth++;
      if (css[i] === "}") depth--;
    }
    out.push(css.slice(start, i));
  }
  return out;
}

function rule(css: string, selector: string): string {
  const i = css.indexOf(`${selector} {`);
  if (i < 0) return "";
  return css.slice(i, css.indexOf("}", i));
}

describe("defect 1: spectrum panel is docked, never covering controls", () => {
  it("renders the drawer inside the view, after the image", () => {
    stubMatchMedia(true);
    const html = renderToStaticMarkup(
      <AppShell
        embed={false}
        targets={[]}
        onSelectTarget={() => {}}
        search={<div className="fake-search">search</div>}
        panel={<div className="fake-panel">panel</div>}
        drawer={<div className="fake-drawer">spectrum</div>}
      >
        <div className="fake-viewer">viewer</div>
      </AppShell>,
    );
    expect(html).toContain("app-view app-view-drawer");
    const view = html.indexOf("app-view");
    const main = html.indexOf("app-main");
    const drawer = html.indexOf("app-drawer");
    const panel = html.indexOf("app-panel");
    expect(view).toBeLessThan(main);
    expect(main).toBeLessThan(drawer);
    // The drawer closes before the controls panel column starts.
    expect(drawer).toBeLessThan(panel);
  });

  it("omits the drawer wrapper when the spectrum is closed", () => {
    stubMatchMedia(true);
    const html = renderShell();
    expect(html).not.toContain("app-drawer");
    expect(html).not.toContain("app-view-drawer");
  });

  it("never positions the spectrum panel as a fixed overlay", () => {
    expect(SPECTRUM_CSS).not.toMatch(/position:\s*fixed/);
    expect(SHELL_CSS).not.toMatch(/position:\s*fixed/);
  });
});

describe("defect 2: spectrum chart fills the panel with a >= 220px floor", () => {
  it("defines a chart floor token >= 220px and uses it", () => {
    const m = /--plot-min-h:\s*([^;]+);/.exec(TOKENS_CSS);
    expect(m).not.toBeNull();
    expect(remToPx(m?.[1] ?? "")).toBeGreaterThanOrEqual(220);
    expect(rule(SPECTRUM_CSS, ".sp-plot")).toContain(
      "min-height: var(--plot-min-h)",
    );
    // The chart is sized from its host (no feedback growth loop).
    expect(SPECTRUM_CSS).toContain(".sp-plot > .uplot");
  });
});

describe("defect 3: segmented buttons are >= 44px at every breakpoint", () => {
  it("sizes segmented options with the touch-target token", () => {
    const opt = rule(CONTROLS_CSS, ".ms-opt");
    expect(opt).toContain("min-height: var(--target)");
    expect(opt).toContain("min-width: var(--target)");
  });

  it("has no breakpoint or bridge rule shrinking them", () => {
    expect(SHELL_CSS).not.toContain(".ms-opt");
    for (const block of atRuleBlocks(CONTROLS_CSS)) {
      expect(block).not.toContain(".ms-opt");
    }
  });
});

describe("defect 4: balanced mobile layout", () => {
  const mobile = SHELL_CSS.split("@media (min-width: 640px)")[0] ?? "";

  it("orders image, spectrum, search, sheet on phones", () => {
    expect(rule(mobile, ".app-main")).toContain("order: -1");
    expect(rule(mobile, ".app-drawer")).toContain("order: -1");
    expect(rule(mobile, ".app-search")).not.toContain("order");
  });

  it("keeps a square image floor on phones", () => {
    expect(mobile).toContain("min-height: min(calc(100vw - 2 * var(--gutter))");
  });

  it("uses a compact two-row search: [RA][Dec][Go] then FOV", () => {
    expect(CONTROLS_CSS).toContain('"ra dec go"');
    expect(CONTROLS_CSS).toContain('"fov fov fov"');
  });
});
