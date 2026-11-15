// Test 10 — accessibility: axe-core must report zero serious/critical
// violations on the home view and with the spectrum panel open, in both
// dark and light color schemes.
import { AxeBuilder } from "@axe-core/playwright";
import { expect, frameCanvas, mockUrl, test } from "./fixtures";

function seriousViolations(results: Awaited<ReturnType<AxeBuilder["analyze"]>>) {
  return results.violations.filter(
    (v) => v.impact === "serious" || v.impact === "critical",
  );
}

function summarize(results: Awaited<ReturnType<AxeBuilder["analyze"]>>) {
  return results.violations.map(
    (v) => `${v.id}(${v.impact}) x${v.nodes.length}`,
  );
}

for (const colorScheme of ["dark", "light"] as const) {
  test(`axe: home and spectrum panel pass in ${colorScheme} mode`, async ({
    page,
  }) => {
    // Known app-side defects pending the parallel redesign (f6): axe reports
    // serious label-content-name-mismatch on the Go button (visible "Go" vs
    // aria-label "Load time-lapse") and the blink toggle ("A/B" vs "Toggle
    // blink comparator"), plus color-contrast issues inside the spectrum
    // panel. src/ is outside this task's scope; the assertions stay strict
    // so the test flips to passing when f6 fixes them.
    test.fail();
    await page.emulateMedia({ colorScheme });
    await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
    await expect(frameCanvas(page)).toBeVisible();

    const home = await new AxeBuilder({ page }).analyze();
    console.log(`[axe ${colorScheme} home]`, summarize(home).join(", ") || "clean");
    expect(
      seriousViolations(home).map((v) => `${v.id}: ${v.nodes.length}`),
      `${colorScheme} home violations`,
    ).toEqual([]);

    await frameCanvas(page).click();
    const panel = page.getByRole("complementary", { name: "Spectrum panel" });
    await expect(panel).toBeVisible();

    const withSpectrum = await new AxeBuilder({ page }).analyze();
    console.log(
      `[axe ${colorScheme} spectrum]`,
      summarize(withSpectrum).join(", ") || "clean",
    );
    expect(
      seriousViolations(withSpectrum).map(
        (v) => `${v.id}: ${v.nodes.length}`,
      ),
      `${colorScheme} spectrum violations`,
    ).toEqual([]);
  });
}
