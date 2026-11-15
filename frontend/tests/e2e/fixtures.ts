// Shared e2e fixture + helpers. Every test gets `netGuard` (auto): a
// page.route('**/*') hook that aborts any request whose origin is not the
// configured baseURL. Mock-mode tests therefore fail fast if the app tries
// to reach IRSA, a CDN or any external host. Elements are located ONLY by
// accessible role, label or visible text so the suite survives the f6
// visual redesign.
import { test as base, expect, type Locator, type Page } from "@playwright/test";
import { PNG } from "pngjs";

export const MOCK_QUERY = "mock=1";
// WISE 0855 official challenge coordinates, decimal degrees.
export const MOCK_RA = "133.795";
export const MOCK_DEC = "-7.245";

export function mockUrl(extra = ""): string {
  return `/?${MOCK_QUERY}${extra === "" ? "" : `&${extra}`}`;
}

export const test = base.extend<{ netGuard: void }>({
  netGuard: [
    async ({ page, baseURL }, use) => {
      const allowedOrigin = new URL(
        baseURL ?? "http://127.0.0.1:4173",
      ).origin;
      await page.route("**/*", (route) => {
        const url = route.request().url();
        if (url.startsWith(allowedOrigin)) {
          return route.continue();
        }
        return route.abort("blockedbyclient");
      });
      await use();
    },
    { auto: true },
  ],
});

export { expect };

// The secondary-controls region is a bottom sheet on mobile (closed by
// default) and an open side panel on desktop. Expand it when needed.
export async function openControlsPanel(page: Page): Promise<void> {
  const toggle = page.getByRole("button", { name: "Controls" });
  if ((await toggle.getAttribute("aria-expanded")) === "false") {
    await toggle.click();
  }
}

// The rendered time-lapse canvas: role=img with the accessible name
// "Frame N of M, detector D, X µm, observed ...".
export function frameCanvas(page: Page): Locator {
  return page.getByRole("img", { name: /^Frame \d+ of \d+/ });
}

export function frameLabelRe(index: number, total: number): RegExp {
  return new RegExp(`^Frame ${index} of ${total}\\b`);
}

// Collects console errors and uncaught page errors for the duration of a
// test. `drain()` returns the messages that are not explainable network
// noise (a request blocked by netGuard logs net::ERR_* in the console).
export function collectConsoleErrors(page: Page): { drain: () => string[] } {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(err.message));
  return {
    drain: () =>
      errors.filter(
        (e) =>
          !e.includes("net::ERR") && !e.includes("Failed to load resource"),
      ),
  };
}

export interface CanvasPixels {
  width: number;
  height: number;
  min: number;
  max: number;
  std: number;
}

// Reads back what the user actually sees: an element-level screenshot of the
// canvas decoded with pngjs. This works for both the WebGL2 path (whose
// drawing buffer cannot be read via toDataURL) and the 2D fallback.
export async function canvasPixels(locator: Locator): Promise<CanvasPixels> {
  const buf = await locator.screenshot();
  const png = PNG.sync.read(buf);
  const { width, height, data } = png;
  let min = 255;
  let max = 0;
  let sum = 0;
  let sum2 = 0;
  let n = 0;
  for (let i = 0; i + 3 < data.length; i += 4) {
    const l = ((data[i] ?? 0) + (data[i + 1] ?? 0) + (data[i + 2] ?? 0)) / 3;
    if (l < min) min = l;
    if (l > max) max = l;
    sum += l;
    sum2 += l * l;
    n += 1;
  }
  const mean = n === 0 ? 0 : sum / n;
  const std = n === 0 ? 0 : Math.sqrt(Math.max(sum2 / n - mean * mean, 0));
  return { width, height, min, max, std };
}

// Asserts the element is the topmost hit target at the centre of its box —
// i.e. it is not covered by an overlaying panel — using
// document.elementFromPoint in the page.
export async function expectNotCovered(
  locator: Locator,
  description: string,
): Promise<void> {
  const result = await locator.evaluate((el) => {
    const r = el.getBoundingClientRect();
    const x = r.left + r.width / 2;
    const y = r.top + r.height / 2;
    const top = document.elementFromPoint(x, y);
    const coveredBy =
      top === null
        ? "nothing"
        : `${top.tagName.toLowerCase()}${
            top instanceof HTMLElement && top.className !== ""
              ? `.${String(top.className).split(" ").join(".")}`
              : ""
          }`;
    return {
      clickable: top === el || (top !== null && el.contains(top)),
      coveredBy,
      x,
      y,
    };
  });
  expect(
    result.clickable,
    `${description} is covered by <${result.coveredBy}> at (${Math.round(
      result.x,
    )}, ${Math.round(result.y)})`,
  ).toBe(true);
}
