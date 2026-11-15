// Test 2 — URL state round trip: control changes are serialized into the
// query string, survive a reload, and malformed values fall back to
// defaults instead of crashing.
import {
  expect,
  frameCanvas,
  mockUrl,
  openControlsPanel,
  test,
} from "./fixtures";

test("mode, stretch, colormap and fps persist in the URL across reload", async ({
  page,
}) => {
  await page.goto(mockUrl("ra=133.795&dec=-7.245&fov=0.05"));
  await expect(frameCanvas(page)).toBeVisible();
  await openControlsPanel(page);

  await page
    .getByRole("radiogroup", { name: "Display mode" })
    .getByRole("radio", { name: /Diff/ })
    .click();
  await page.getByLabel("Stretch").selectOption("log");
  await page.getByLabel("Colormap").selectOption("viridis");
  await page.getByLabel("Playback speed").fill("12");

  await expect(page).toHaveURL(/mode=diff/);
  await expect(page).toHaveURL(/stretch=log/);
  await expect(page).toHaveURL(/cmap=viridis/);
  await expect(page).toHaveURL(/fps=12/);

  // The mock flag is not part of UrlState, so re-add it before reloading to
  // keep this test offline.
  const u = new URL(page.url());
  u.searchParams.set("mock", "1");
  await page.goto(`${u.pathname}${u.search}`);

  await expect(frameCanvas(page)).toBeVisible();
  await openControlsPanel(page);
  await expect(
    page
      .getByRole("radiogroup", { name: "Display mode" })
      .getByRole("radio", { name: /Diff/ }),
  ).toBeChecked();
  await expect(page.getByLabel("Stretch")).toHaveValue("log");
  await expect(page.getByLabel("Colormap")).toHaveValue("viridis");
  await expect(page.getByText("12 fps")).toBeVisible();
});

test("invalid URL values fall back to defaults without crashing", async ({
  page,
}) => {
  await page.goto(
    mockUrl(
      "ra=400&dec=-7.245&fov=9&mode=zzz&stretch=nope&cmap=nah&fps=999&frame=-3",
    ),
  );

  // The app still renders: heading, viewer (mock frames) and controls.
  await expect(
    page.getByRole("heading", { name: "SPHEREx Blink" }),
  ).toBeVisible();
  await expect(frameCanvas(page)).toBeVisible();

  // ra=400 is out of range -> the field falls back to empty; dec stays.
  await expect(page.getByLabel("Right ascension")).toHaveValue("");
  await expect(page.getByLabel("Declination")).toHaveValue("-7.245");

  await openControlsPanel(page);
  await expect(
    page
      .getByRole("radiogroup", { name: "Display mode" })
      .getByRole("radio", { name: "Raw" }),
  ).toBeChecked();
  await expect(page.getByLabel("Stretch")).toHaveValue("linear");
  await expect(page.getByLabel("Colormap")).toHaveValue("grayscale");
  await expect(
    page.getByLabel("Field of view in degrees"),
  ).toHaveValue("0.05");
  await expect(page.getByText("0.05°", { exact: true })).toBeVisible();
  await expect(page.getByText("5 fps")).toBeVisible();
});
