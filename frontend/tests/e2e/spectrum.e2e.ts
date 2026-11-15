// Test 6 — click-to-spectrum: the panel opens with the full mock point
// list, Escape closes it, and while open it must not cover the primary
// controls (Go, search inputs, Controls toggle).
import { makeMockSpectrum } from "../../src/components/spectrum/mockSpectrum";
import { prepareSpectrum } from "../../src/components/spectrum/prepare";
import {
  expect,
  expectNotCovered,
  frameCanvas,
  mockUrl,
  test,
} from "./fixtures";

// Number of usable points in the synthetic spectrum, computed with the same
// preparation the panel uses (one mock point is NaN on purpose).
const EXPECTED_POINTS = prepareSpectrum(makeMockSpectrum(0, 0).points).validCount;

test("canvas click opens the spectrum panel with every mock point", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  await frameCanvas(page).click();

  const panel = page.getByRole("complementary", { name: "Spectrum panel" });
  await expect(panel).toBeVisible();
  await expect(
    panel.getByText(new RegExp(`^${EXPECTED_POINTS} points`)),
  ).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(panel).toHaveCount(0);
});

test("controls stay clickable while the spectrum panel is open", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  await frameCanvas(page).click();
  await expect(
    page.getByRole("complementary", { name: "Spectrum panel" }),
  ).toBeVisible();

  // The docked spectrum panel must not intercept the primary controls.
  await expectNotCovered(
    page.getByRole("button", { name: "Load time-lapse" }),
    "Go button",
  );
  await expectNotCovered(
    page.getByLabel("Right ascension"),
    "RA input",
  );
  await expectNotCovered(
    page.getByRole("button", { name: "Controls" }),
    "Controls panel toggle",
  );
});
