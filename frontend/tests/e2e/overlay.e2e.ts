// Test 7 — catalog overlay toggle: mounts/unmounts the overlay section
// without breaking the viewer. The Aladin Lite script comes from a CDN and
// is blocked by the netGuard fixture, so the app must show its graceful
// fallback message instead of crashing.
import {
  expect,
  frameCanvas,
  mockUrl,
  openControlsPanel,
  test,
} from "./fixtures";

test("overlay toggle mounts, degrades gracefully and unmounts", async ({
  page,
}) => {
  // Known app-side defect found by this test: AladinOverlay's load effect
  // has `status` in its deps, so setStatus("loading") re-runs it and marks
  // its own load handlers disposed — the status never reaches "error" (or
  // "ready") and the fallback below is never rendered ("Loading Aladin
  // Lite…" forever). src/ is outside this task's scope; flagged for the
  // f6/integration agents. Remove this marker when fixed.
  test.fail();
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  const canvas = frameCanvas(page);
  await expect(canvas).toBeVisible();
  await openControlsPanel(page);

  const overlayCheck = page.getByRole("checkbox", {
    name: "Catalog overlay",
  });
  const region = page.getByRole("region", { name: "Sky catalog overlay" });

  await overlayCheck.check();
  await expect(region).toBeVisible();
  // The CDN script is blocked -> graceful fallback inside the overlay
  // (role=alert cannot be matched by accessible name, so filter by text).
  await expect(
    region.getByRole("alert").filter({ hasText: /Sky overlay unavailable/ }),
  ).toBeVisible({ timeout: 10_000 });

  await overlayCheck.uncheck();
  await expect(region).toHaveCount(0);

  // Toggling again remounts; the viewer keeps working throughout.
  await overlayCheck.check();
  await expect(region).toBeVisible();
  await overlayCheck.uncheck();
  await expect(region).toHaveCount(0);
  await expect(canvas).toBeVisible();
  await expect(canvas).toHaveAttribute("aria-label", /^Frame \d+ of \d+/);
});
