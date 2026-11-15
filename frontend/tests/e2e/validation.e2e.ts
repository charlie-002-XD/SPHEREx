// Test 8 — client-side validation: an out-of-range RA shows a visible
// message and never issues a frames request.
import { expect, test } from "./fixtures";

test("RA 400 shows a validation error and issues no frames request", async ({
  page,
}) => {
  const framesRequests: string[] = [];
  page.on("request", (req) => {
    if (req.url().includes("/api/v1/frames")) framesRequests.push(req.url());
  });

  // No mock here: a real frames request would go to the preview origin and
  // is counted (and would be visible in framesRequests).
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "SPHEREx Blink" }),
  ).toBeVisible();

  await page.getByLabel("Right ascension").fill("400");
  await page.getByLabel("Declination").fill("-7.245");
  await page.getByRole("button", { name: "Load time-lapse" }).click();

  const alert = page.getByRole("alert").filter({ hasText: /Invalid RA/ });
  await expect(alert).toBeVisible();
  // Raw user input is never echoed back in the error text.
  await expect(alert).not.toContainText("400");

  await page.waitForTimeout(300); // settle any stray fetch
  expect(framesRequests).toEqual([]);
});
