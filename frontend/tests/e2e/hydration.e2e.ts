// Test 1 — URL hydration: a deep link with coordinates must render the
// canvas with its first frame quickly, and embed mode must strip chrome.
import { expect, frameCanvas, frameLabelRe, mockUrl, test } from "./fixtures";

test("hydrates from URL params and renders the first frame fast", async ({
  page,
}) => {
  const startedAt = Date.now();
  await page.goto(
    mockUrl("ra=133.795&dec=-7.245&fov=0.05&embed=1"),
  );
  const canvas = frameCanvas(page);
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(1, 12));
  const elapsed = Date.now() - startedAt;
  expect(elapsed, `first frame took ${elapsed}ms`).toBeLessThan(3000);

  // embed=1 keeps only the viewer: no banner, no search form.
  await expect(page.getByRole("banner")).toHaveCount(0);
  await expect(page.getByRole("search")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "SPHEREx Blink" }),
  ).toHaveCount(0);
});
