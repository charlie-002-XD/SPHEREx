// Playwright end-to-end configuration. The default projects run fully
// against the app's ?mock=1 synthetic data on a `vite preview` static build;
// nothing here may reach the real IRSA or any CDN (see fixtures.ts, which
// aborts every request outside the preview origin). The opt-in "live"
// project points at a separately started backend via E2E_BASE_URL and is
// skipped unless E2E_LIVE=1.
import { defineConfig, devices } from "@playwright/test";

const PREVIEW_URL = "http://127.0.0.1:4173";

export default defineConfig({
  testDir: "./tests/e2e",
  // ".e2e.ts" keeps these files out of vitest's *.spec/*.test glob.
  testMatch: "**/*.e2e.ts",
  fullyParallel: true,
  timeout: 30_000,
  expect: { timeout: 5_000 },
  reporter: [
    ["list"],
    // Reports/artifacts live under dist/ so `eslint .` never scans the
    // report's bundled JS (eslint.config.js ignores dist; test-results/ is
    // kept in .gitignore for any stray output).
    ["html", { outputFolder: "dist/test-results/report", open: "never" }],
  ],
  outputDir: "dist/test-results/artifacts",
  use: {
    baseURL: PREVIEW_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  webServer: {
    command:
      "npm run build && npx vite preview --host 127.0.0.1 --port 4173 --strictPort",
    url: PREVIEW_URL,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  projects: [
    {
      name: "chromium-desktop",
      grepInvert: /@live/,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
      },
    },
    {
      name: "chromium-mobile",
      grepInvert: /@live/,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 375, height: 667 },
        hasTouch: true,
        isMobile: true,
      },
    },
    // Opt-in live suite: requests a separately started backend serving the
    // same frontend build (or this preview if E2E_BASE_URL is a backend
    // origin that serves the app). Run with E2E_LIVE=1 E2E_BASE_URL=...
    {
      name: "live",
      grep: /@live/,
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 900 },
        baseURL: process.env.E2E_BASE_URL ?? "http://127.0.0.1:8080",
      },
    },
  ],
});
