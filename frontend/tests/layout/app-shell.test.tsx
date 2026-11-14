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
    expect(html).toContain("app-panel");
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
    const fixed =
      mobileLayer.match(/(?<![a-z-])width:\s*(\d+)(px|rem)/g) ?? [];
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
