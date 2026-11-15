// Test 11 — reference screenshots at the three viewports in both color
// schemes, saved under frontend/test-results/screens/ (runtime artifacts,
// not committed).
import { expect, frameCanvas, mockUrl, test } from "./fixtures";

const SHOTS_DIR = "test-results/screens";
const VIEWPORTS = [
  { width: 375, height: 667 },
  { width: 768, height: 1024 },
  { width: 1440, height: 900 },
] as const;

test("captures screenshots at reference viewports in dark and light", async ({
  page,
}) => {
  for (const scheme of ["dark", "light"] as const) {
    await page.emulateMedia({ colorScheme: scheme });
    for (const vp of VIEWPORTS) {
      await page.setViewportSize(vp);
      await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
      await expect(frameCanvas(page)).toBeVisible();
      await page.waitForTimeout(150); // settle the first draw
      await page.screenshot({
        path: `${SHOTS_DIR}/home-${vp.width}x${vp.height}-${scheme}.png`,
      });
    }
  }
});
