// @vitest-environment jsdom
// Component tests for the About dialog, the first-visit hint and the
// embed credit line. jsdom has no HTMLDialogElement.showModal/close, so
// these tests exercise the component's fallback path (open attribute +
// manual focus handling); in real browsers the same handlers delegate to
// the native modal behavior.
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { AboutDialog } from "../../src/components/about/AboutDialog";
import { FirstRunHint } from "../../src/components/about/FirstRunHint";
import { EmbedCredit } from "../../src/components/about/EmbedCredit";

const HINT_KEY = "spherex-blink.hint.dismissed";

function setSearch(search: string): void {
  window.history.replaceState(null, "", `/${search}`);
}

function aboutDialog(): HTMLDialogElement {
  const d = document.querySelector("dialog");
  expect(d).not.toBeNull();
  return d as HTMLDialogElement;
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  window.localStorage.clear();
  window.history.replaceState(null, "", "/");
});

describe("AboutDialog", () => {
  it("opens a labelled dialog from the About button and closes it", () => {
    render(<AboutDialog />);
    const dialog = aboutDialog();
    expect(dialog.hasAttribute("open")).toBe(false);

    fireEvent.click(screen.getByRole("button", { name: "About" }));
    expect(dialog.hasAttribute("open")).toBe(true);
    expect(
      screen.getByRole("dialog", { name: "About SPHEREx Blink" }),
    ).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(dialog.hasAttribute("open")).toBe(false);
  });

  it("closes on Escape", () => {
    render(<AboutDialog />);
    const dialog = aboutDialog();
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(dialog.hasAttribute("open")).toBe(false);
  });

  it("closes when the cancel event fires (native Escape path)", () => {
    render(<AboutDialog />);
    const dialog = aboutDialog();
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    fireEvent(dialog, new Event("cancel", { bubbles: false }));
    expect(dialog.hasAttribute("open")).toBe(false);
  });

  it("closes on a backdrop click but not on a click inside the box", () => {
    render(<AboutDialog />);
    const dialog = aboutDialog();
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    // jsdom reports a zero rect: a click at 0,0 counts as "inside".
    fireEvent.click(dialog, { clientX: 0, clientY: 0 });
    expect(dialog.hasAttribute("open")).toBe(true);
    fireEvent.click(dialog, { clientX: 100, clientY: 100 });
    expect(dialog.hasAttribute("open")).toBe(false);
  });

  it("moves focus into the dialog and returns it to the About button", () => {
    render(<AboutDialog />);
    const trigger = screen.getByRole("button", { name: "About" });
    trigger.focus();
    fireEvent.click(trigger);
    const close = screen.getByRole("button", { name: "Close" });
    expect(document.activeElement).toBe(close);
    fireEvent.click(close);
    expect(document.activeElement).toBe(trigger);
  });

  it("traps Tab inside the dialog on the fallback path", () => {
    render(<AboutDialog />);
    const dialog = aboutDialog();
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    const focusables = dialog.querySelectorAll<HTMLElement>(
      'a[href], button:not([disabled]), input, select, textarea, summary, [tabindex]:not([tabindex="-1"])',
    );
    const first = focusables[0];
    const last = focusables[focusables.length - 1];
    expect(first).toBeDefined();
    expect(last).toBeDefined();
    last?.focus();
    fireEvent.keyDown(dialog, { key: "Tab" });
    expect(document.activeElement).toBe(first);
    first?.focus();
    fireEvent.keyDown(dialog, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it("shows modes, detectors, shortcuts and the data credits", () => {
    render(<AboutDialog />);
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    const dialog = aboutDialog();
    expect(dialog.textContent).toContain("Blink A/B");
    expect(dialog.textContent).toContain("Diff");
    expect(dialog.textContent).toContain("Co-add");
    expect(dialog.textContent).toContain("Stack");
    expect(dialog.textContent).toContain("spectrum");
    expect(dialog.textContent).toContain("D1–D6");
    expect(dialog.textContent).toContain("FITS header");
    expect(dialog.textContent).toContain("Play / pause");
    expect(dialog.textContent).toContain("NASA/IPAC Infrared Science Archive");
    expect(dialog.textContent).toContain("Gaia DR3");
    expect(dialog.textContent).toContain("AllWISE");
    expect(dialog.textContent).toContain("CDS VizieR");
    expect(dialog.textContent).toContain("Aladin Lite");
    expect(dialog.textContent).toContain("Open source, Apache-2.0");
    expect(dialog.textContent).toContain(
      "Built for the NASA Space Apps Challenge 2026",
    );
  });

  it("links to IRSA and Aladin Lite", () => {
    render(<AboutDialog />);
    fireEvent.click(screen.getByRole("button", { name: "About" }));
    expect(
      screen
        .getByRole("link", { name: /Infrared Science Archive/ })
        .getAttribute("href"),
    ).toBe("https://irsa.ipac.caltech.edu/Missions/spherex.html");
    expect(
      screen
        .getByRole("link", { name: /Aladin Lite/ })
        .getAttribute("href"),
    ).toBe("https://aladin.cds.unistra.fr/AladinLite/");
  });
});

describe("FirstRunHint", () => {
  beforeEach(() => setSearch(""));

  it("shows a one-line hint for a first-time visitor", () => {
    render(<FirstRunHint />);
    expect(screen.getByRole("note")).toBeTruthy();
    expect(
      screen.getByText(/Press play, or click the image/),
    ).toBeTruthy();
  });

  it("hides on dismissal, stores the preference and never reappears", () => {
    render(<FirstRunHint />);
    fireEvent.click(screen.getByRole("button", { name: "Dismiss hint" }));
    expect(screen.queryByRole("note")).toBeNull();
    expect(window.localStorage.getItem(HINT_KEY)).toBe("1");

    cleanup();
    render(<FirstRunHint />);
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("still renders and dismisses when storage is blocked", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("storage denied");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("storage denied");
    });
    render(<FirstRunHint />);
    expect(screen.getByRole("note")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss hint" }));
    expect(screen.queryByRole("note")).toBeNull();
  });

  it("is hidden in embed mode and under ?mock=1 unless ?hint=1", () => {
    setSearch("?embed=1");
    render(<FirstRunHint />);
    expect(screen.queryByRole("note")).toBeNull();
    cleanup();

    setSearch("?mock=1");
    render(<FirstRunHint />);
    expect(screen.queryByRole("note")).toBeNull();
    cleanup();

    setSearch("?mock=1&hint=1");
    render(<FirstRunHint />);
    expect(screen.getByRole("note")).toBeTruthy();
  });
});

describe("EmbedCredit", () => {
  it("links back to the full app without the embed flag", () => {
    setSearch("?mock=1&ra=133.79&dec=-7.24&embed=1");
    render(<EmbedCredit />);
    const link = screen.getByRole("link", {
      name: /NASA SPHEREx data via IRSA/,
    });
    const href = link.getAttribute("href") ?? "";
    expect(href).toContain("mock=1");
    expect(href).toContain("ra=133.79");
    expect(href).not.toContain("embed");
    expect(link.getAttribute("target")).toBe("_blank");
    expect(link.getAttribute("rel")).toContain("noopener");
  });
});
