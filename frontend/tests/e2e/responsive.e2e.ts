// Test 9 — responsive layout: no horizontal overflow at the three reference
// widths, and every visible interactive element is a >=44px touch target on
// the mobile viewport (native range inputs exempt per the brief; checkbox
// and radio inputs are measured through their wrapping label, which is the
// real hit area).
import { expect, frameCanvas, mockUrl, test } from "./fixtures";

const WIDTHS = [
  { width: 375, height: 667 },
  { width: 768, height: 1024 },
  { width: 1440, height: 900 },
] as const;

test("no horizontal overflow at 375, 768 and 1440 px", async ({ page }) => {
  for (const vp of WIDTHS) {
    await page.setViewportSize(vp);
    await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
    await expect(frameCanvas(page)).toBeVisible();
    const overflow = await page.evaluate(() => ({
      doc: document.documentElement.scrollWidth,
      docClient: document.documentElement.clientWidth,
      body: document.body.scrollWidth,
    }));
    expect(
      overflow.doc,
      `document scrollWidth ${overflow.doc} > clientWidth ${overflow.docClient} at ${vp.width}px`,
    ).toBeLessThanOrEqual(overflow.docClient);
    expect(
      overflow.body,
      `body scrollWidth ${overflow.body} > clientWidth ${overflow.docClient} at ${vp.width}px`,
    ).toBeLessThanOrEqual(overflow.docClient);
  }
});

test("interactive elements are >=44px tall on the mobile viewport", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 667 });
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  await expect(frameCanvas(page)).toBeVisible();

  const offenders = await page.evaluate(() => {
    const bad: Array<{ tag: string; text: string; height: number }> = [];
    const els = document.querySelectorAll<HTMLElement>(
      'button, select, summary, input:not([type="range"]), a[href]',
    );
    for (const el of els) {
      const rect = el.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue; // hidden
      let height = rect.height;
      // For checkbox/radio inputs the label is the tap target.
      if (
        el instanceof HTMLInputElement &&
        (el.type === "checkbox" || el.type === "radio")
      ) {
        const label = el.closest("label");
        if (label !== null) height = label.getBoundingClientRect().height;
      }
      if (height < 44) {
        bad.push({
          tag: el.tagName.toLowerCase(),
          text: (el.textContent ?? "").trim().slice(0, 40),
          height: Math.round(height),
        });
      }
    }
    return bad;
  });
  expect(offenders, JSON.stringify(offenders)).toEqual([]);
});
