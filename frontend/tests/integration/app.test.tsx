// @vitest-environment jsdom
// Integration tests for the wired app: ?mock=1 renders the full UI without
// a backend, a click on the cutout canvas opens the spectrum panel with
// data, and the Overlay toggle mounts/unmounts the lazy Aladin overlay.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "../../src/App";
import { makeMockFramesResponse } from "../../src/lib/mock";

// jsdom has no matchMedia; the desktop layout (controls panel open) is the
// deterministic path for these assertions.
function stubMatchMedia(): void {
  window.matchMedia = ((query: string) => ({
    matches: true,
    media: query,
    onchange: null,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

// jsdom returns a zero rect; the click-to-spectrum math needs a real size.
function stubRect(el: Element, size = 200): void {
  el.getBoundingClientRect = () =>
    ({
      left: 0,
      top: 0,
      right: size,
      bottom: size,
      width: size,
      height: size,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    }) as DOMRect;
}

beforeEach(() => {
  stubMatchMedia();
  // jsdom has no canvas; return null so the renderer's NullRenderer
  // fallback engages without "Not implemented" console noise.
  HTMLCanvasElement.prototype.getContext = (() =>
    null) as unknown as typeof HTMLCanvasElement.prototype.getContext;
});

afterEach(() => {
  cleanup();
  window.history.replaceState(null, "", "/");
});

describe("App integration", () => {
  it("renders frames from ?mock=1 with no backend", async () => {
    window.history.replaceState(null, "", "/?mock=1");
    render(<App />);
    const canvas = await screen.findByRole("img", {
      name: /Frame 1 of 12/,
    });
    expect(canvas.tagName).toBe("CANVAS");
    // Current-frame metadata is wired into the secondary-controls panel.
    expect(screen.getByText("spherex_mock")).toBeTruthy();
  });

  it("opens the spectrum panel with mock data on canvas click", async () => {
    render(<App mockData={makeMockFramesResponse()} />);
    const canvas = await screen.findByRole("img", {
      name: /Frame 1 of 12/,
    });
    stubRect(canvas);
    fireEvent.click(canvas, { clientX: 100, clientY: 100 });
    const panel = await screen.findByRole("complementary", {
      name: "Spectrum panel",
    });
    expect(panel).toBeTruthy();
    // Docked into the layout (f6): inside the view column, never a sibling
    // overlay, and it contains none of the search/transport controls.
    expect(panel.closest(".app-view")).not.toBeNull();
    expect(panel.closest(".app-drawer")).not.toBeNull();
    const go = screen.getByRole("button", { name: "Load time-lapse" });
    const play = screen.getByRole("button", { name: "Play" });
    expect(panel.contains(go)).toBe(false);
    expect(panel.contains(play)).toBe(false);
    // Mock spectrum resolves; the summary lists the usable point count.
    await screen.findByText(/\d+ points/);
    // Closing returns to the viewer.
    fireEvent.click(
      screen.getByRole("button", { name: "Close spectrum panel" }),
    );
    expect(
      screen.queryByRole("complementary", { name: "Spectrum panel" }),
    ).toBeNull();
  });

  it("mounts and unmounts the Aladin overlay from the panel toggle", async () => {
    render(<App mockData={makeMockFramesResponse()} />);
    await screen.findByRole("img", { name: /Frame 1 of 12/ });
    const toggle = screen.getByRole("checkbox", {
      name: /catalog overlay/i,
    });
    expect(
      screen.queryByRole("group", { name: "Catalog layers" }),
    ).toBeNull();
    fireEvent.click(toggle);
    // Lazy chunk mounts; the CDN script stays pending in jsdom, which is
    // fine — the section and its catalog toggles render regardless.
    await screen.findByRole("group", { name: "Catalog layers" });
    fireEvent.click(toggle);
    expect(
      screen.queryByRole("group", { name: "Catalog layers" }),
    ).toBeNull();
  });
});
