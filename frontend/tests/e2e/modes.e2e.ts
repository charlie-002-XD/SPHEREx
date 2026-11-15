// Test 5 — every display mode renders without console errors and leaves a
// non-blank canvas (verified by reading back its rasterized pixels).
import {
  canvasPixels,
  collectConsoleErrors,
  expect,
  frameCanvas,
  mockUrl,
  openControlsPanel,
  test,
} from "./fixtures";

const MODES = [/Raw/, /Diff/, /Co-add/, /Stack/] as const;

test("raw, difference, co-add and detector stack all render", async ({
  page,
}) => {
  const errors = collectConsoleErrors(page);
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  const canvas = frameCanvas(page);
  await expect(canvas).toBeVisible();
  await openControlsPanel(page);

  for (const name of MODES) {
    await page
      .getByRole("radiogroup", { name: "Display mode" })
      .getByRole("radio", { name })
      .click();
    await page.waitForTimeout(200); // let the draw effect run
    const stats = await canvasPixels(canvas);
    expect(
      stats.max - stats.min,
      `mode ${String(name)} produced a blank canvas`,
    ).toBeGreaterThan(10);
    expect(stats.std).toBeGreaterThan(1);
  }

  expect(errors.drain()).toEqual([]);
});
