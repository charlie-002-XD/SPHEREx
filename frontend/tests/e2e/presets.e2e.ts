// Test 3 — the five official targets are reachable from the header target
// menu and load their RA/Dec into the search inputs.
import { expect, mockUrl, test } from "./fixtures";

const TARGETS = [
  { label: "Barnard's Star", ra: "17h57m48.5s", dec: "+04d41m36s" },
  { label: "Epsilon Indi B", ra: "22h04m10.5s", dec: "-56d46m58s" },
  { label: "WISE 0855", ra: "08h55m10.831s", dec: "-07d14m42.53s" },
  { label: "RW Aurigae", ra: "05h07m49.5s", dec: "+30d24m05s" },
  { label: "R Draconis", ra: "16h32m40.2s", dec: "+66d45m17s" },
] as const;

test("all five official targets load their coordinates from the menu", async ({
  page,
}) => {
  await page.goto(mockUrl());
  const raInput = page.getByLabel("Right ascension");
  const decInput = page.getByLabel("Declination");

  for (const t of TARGETS) {
    await page.getByText("Targets", { exact: true }).click();
    // Menu items read "label ra · dec"; the same-named preset pills in the
    // search bar are only "label", so matching on the coordinates picks the
    // menu entry unambiguously.
    await page
      .getByRole("button", {
        name: new RegExp(`${t.label} ${t.ra}`),
      })
      .click();
    await expect(raInput).toHaveValue(t.ra);
    await expect(decInput).toHaveValue(t.dec);
  }
});
