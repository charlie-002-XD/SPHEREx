// Opt-in @live suite — runs only on the "live" project against a separately
// started backend at E2E_BASE_URL (default http://127.0.0.1:8080) and only
// when E2E_LIVE=1. It hits real IRSA through the backend, so it is slow and
// never part of CI. Assertions are minimal (>=2 frames, no error state);
// scientific values are logged, not asserted.
import { expect, frameCanvas, test } from "./fixtures";

// @types/node is intentionally absent (see pngjs.d.ts); read env vars
// through a local declaration instead.
declare const process:
  | { env: Record<string, string | undefined> }
  | undefined;
const env = typeof process === "undefined" ? {} : process.env;
const LIVE = env.E2E_LIVE === "1";

test.describe("@live backend integration", { tag: "@live" }, () => {
  test.skip(!LIVE, "opt-in live tests: set E2E_LIVE=1");
  test.setTimeout(300_000);

  const TARGETS = [
    { name: "WISE 0855", ra: "133.795158", dec: "-7.245147" },
    { name: "Barnard's Star", ra: "269.452", dec: "4.69333" },
  ] as const;

  for (const t of TARGETS) {
    test(`${t.name}: backend returns at least 2 frames and no error state`, async ({
      page,
    }) => {
      await page.goto(`/?ra=${t.ra}&dec=${t.dec}&fov=0.05`);

      const canvas = frameCanvas(page);
      await expect(canvas).toHaveAttribute(
        "aria-label",
        /^Frame \d+ of ([2-9]|\d{2,})\b/,
        { timeout: 240_000 },
      );
      await expect(page.getByText("Could not load frames")).toHaveCount(0);

      // Log the science metadata for the record; never asserted.
      const label = await canvas.getAttribute("aria-label");
      console.log(`[live] ${t.name}: ${label ?? "no frame label"}`);
    });
  }
});
