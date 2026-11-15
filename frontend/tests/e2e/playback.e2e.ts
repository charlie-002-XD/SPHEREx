// Test 4 — transport: play/pause, step buttons, scrubber and the A/B blink
// comparator all move the visible frame index (read from the canvas's
// accessible name "Frame N of M ...").
import {
  expect,
  frameCanvas,
  frameLabelRe,
  mockUrl,
  test,
} from "./fixtures";

test("play, pause, step, scrubber and blink change the frame index", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  const canvas = frameCanvas(page);
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(1, 12));

  // Step buttons.
  await page.getByRole("button", { name: "Next frame" }).click();
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(2, 12));
  await page.getByRole("button", { name: "Previous frame" }).click();
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(1, 12));

  // Scrubber jumps straight to an index.
  await page.getByLabel("Frame scrubber").fill("6");
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(7, 12));

  // Blink A/B: while paused the A anchor is shown; anchoring A to the
  // current frame makes the displayed index follow it.
  await page.getByRole("button", { name: "Toggle blink comparator" }).click();
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(1, 12));
  await page
    .getByRole("button", { name: "Set blink reference to current frame" })
    .click();
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(7, 12));
  await page.getByRole("button", { name: "Toggle blink comparator" }).click();
  await expect(canvas).toHaveAttribute("aria-label", frameLabelRe(7, 12));

  // Play advances; pause stops it.
  await page.getByRole("button", { name: "Play" }).click();
  await expect(canvas).not.toHaveAttribute(
    "aria-label",
    frameLabelRe(7, 12),
    { timeout: 5000 },
  );
  await page.getByRole("button", { name: "Pause" }).click();
  const paused = await canvas.getAttribute("aria-label");
  await page.waitForTimeout(700); // ~3 frames at the default 5 fps
  await expect(canvas).toHaveAttribute("aria-label", paused ?? "");
});
