// About dialog, first-visit hint and embed attribution. Elements are
// located only by accessible role and name; the suite runs at 1440px
// (chromium-desktop) and 375px (chromium-mobile).
import { AxeBuilder } from "@axe-core/playwright";
import { expect, mockUrl, test } from "./fixtures";

function seriousViolations(
  results: Awaited<ReturnType<AxeBuilder["analyze"]>>,
) {
  return results.violations.filter(
    (v) => v.impact === "serious" || v.impact === "critical",
  );
}

test("About dialog explains the tool and credits the data", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245"));
  const about = page.getByRole("button", { name: "About" });
  await about.click();

  const dialog = page.getByRole("dialog", { name: "About SPHEREx Blink" });
  await expect(dialog).toBeVisible();
  await expect(dialog).toContainText("time-lapse");
  await expect(dialog).toContainText("Blink A/B");
  await expect(dialog).toContainText("Co-add");
  await expect(dialog).toContainText("D1–D6");
  await expect(dialog).toContainText("Play / pause");
  await expect(dialog).toContainText(
    "NASA/IPAC Infrared Science Archive (IRSA)",
  );
  await expect(dialog).toContainText("Gaia DR3");
  await expect(dialog).toContainText("AllWISE");
  await expect(dialog).toContainText("Open source, Apache-2.0");
  await expect(dialog).toContainText(
    "Built for the NASA Space Apps Challenge 2026",
  );

  await expect(
    dialog.getByRole("link", { name: /Infrared Science Archive/ }),
  ).toHaveAttribute(
    "href",
    "https://irsa.ipac.caltech.edu/Missions/spherex.html",
  );
  await expect(
    dialog.getByRole("link", { name: /Aladin Lite/ }),
  ).toHaveAttribute("href", "https://aladin.cds.unistra.fr/AladinLite/");
});

test("About dialog closes with Escape and restores focus", async ({
  page,
}) => {
  await page.goto(mockUrl());
  const about = page.getByRole("button", { name: "About" });
  await about.click();
  const dialog = page.getByRole("dialog", { name: "About SPHEREx Blink" });
  await expect(dialog).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();
  await expect(about).toBeFocused();
});

test("About dialog closes on backdrop click and on its Close button", async ({
  page,
}) => {
  await page.goto(mockUrl());
  const about = page.getByRole("button", { name: "About" });
  const dialog = page.getByRole("dialog", { name: "About SPHEREx Blink" });

  await about.click();
  await expect(dialog).toBeVisible();
  await page.mouse.click(4, 4); // top-left corner = backdrop
  await expect(dialog).toBeHidden();

  await about.click();
  await dialog.getByRole("button", { name: "Close" }).click();
  await expect(dialog).toBeHidden();
});

for (const colorScheme of ["dark", "light"] as const) {
  test(`axe: no serious violations with the About dialog open (${colorScheme})`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme });
    await page.goto(mockUrl());
    await page.getByRole("button", { name: "About" }).click();
    await expect(
      page.getByRole("dialog", { name: "About SPHEREx Blink" }),
    ).toBeVisible();
    const results = await new AxeBuilder({ page }).analyze();
    console.log(
      `[axe ${colorScheme} about]`,
      results.violations
        .map((v) => `${v.id}(${v.impact}) x${v.nodes.length}`)
        .join(", ") || "clean",
    );
    expect(
      seriousViolations(results).map((v) => `${v.id}: ${v.nodes.length}`),
      `${colorScheme} about-dialog violations`,
    ).toEqual([]);
  });
}

test("About dialog stays usable on a 375px phone", async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 667 });
  await page.goto(mockUrl());
  await page.getByRole("button", { name: "About" }).click();
  const dialog = page.getByRole("dialog", { name: "About SPHEREx Blink" });
  await expect(dialog).toBeVisible();

  // Fits inside the viewport and does not cause horizontal overflow.
  const box = await dialog.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.x).toBeGreaterThanOrEqual(0);
  expect(box!.x + box!.width).toBeLessThanOrEqual(375);
  expect(box!.height).toBeLessThanOrEqual(667);
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth,
  );
  expect(overflow).toBeLessThanOrEqual(375);

  // Content scrolls inside the dialog body.
  const scrolls = await dialog.evaluate((d) => {
    const body = d.querySelector(".about-body");
    if (body === null) return "no-body";
    const s = getComputedStyle(body);
    return s.overflowY === "auto" || s.overflowY === "scroll"
      ? "scrollable"
      : s.overflowY;
  });
  expect(scrolls).toBe("scrollable");

  // Close button is a >= 44px touch target.
  const closeBox = await dialog
    .getByRole("button", { name: "Close" })
    .boundingBox();
  expect(closeBox).not.toBeNull();
  expect(closeBox!.height).toBeGreaterThanOrEqual(44);
  expect(closeBox!.width).toBeGreaterThanOrEqual(44);
});

test("first-visit hint: hidden in mock, forced by ?hint=1, dismisses for good", async ({
  page,
}) => {
  // Regular e2e runs never see it.
  await page.goto(mockUrl());
  await expect(page.getByRole("note")).toHaveCount(0);

  // ?hint=1 surfaces it; the dismiss button hides it.
  await page.goto(mockUrl("hint=1"));
  const hint = page.getByRole("note");
  await expect(hint).toBeVisible();
  await expect(hint).toContainText(/Press play/);
  await page.getByRole("button", { name: "Dismiss hint" }).click();
  await expect(page.getByRole("note")).toHaveCount(0);
});

test("first-visit hint stays dismissed across reloads", async ({ page }) => {
  // A non-mock URL exercises the real localStorage path (the frame request
  // is aborted by netGuard; the hint does not depend on data).
  await page.goto("/?ra=133.795&dec=-7.245");
  const hint = page.getByRole("note");
  await expect(hint).toBeVisible();
  await page.getByRole("button", { name: "Dismiss hint" }).click();
  await expect(page.getByRole("note")).toHaveCount(0);

  await page.reload();
  await expect(page.getByRole("note")).toHaveCount(0);
});

test("embed mode keeps the IRSA attribution linking to the full app", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05&embed=1"));
  const credit = page.getByRole("link", {
    name: /SPHEREx Blink.*NASA SPHEREx data via IRSA/,
  });
  await expect(credit).toBeVisible();
  const href = await credit.getAttribute("href");
  expect(href).toContain("mock=1");
  expect(href).toContain("ra=133.795");
  expect(href).not.toContain("embed=1");
});
