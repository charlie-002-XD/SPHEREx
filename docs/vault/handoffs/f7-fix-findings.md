# f7-fix-findings — repair the app defects found by the e2e suite

Status: done. `npx playwright test` is fully green: 30 passed, 2 skipped
(opt-in @live), 0 failures, and **no `test.fail()`/`test.fixme()` markers
left anywhere in `frontend/tests/`**. axe reports zero violations
(any impact) on home and spectrum-open, dark and light, at 1440px and
375px.

## What was wrong and what changed

### App defects fixed in `src/`

1. **`AladinOverlay` never left "loading"**
   (`src/components/overlays/AladinOverlay.tsx`). The init effect declared
   `[open, status]` and called `setStatus("loading")` inside itself, so it
   re-ran immediately and its own `disposed` flag invalidated the only
   `loadAladin().then/.catch` handlers — status could never reach
   "ready" or "error". The effect now depends on `[open]` only; the close
   path already resets `status`/`aladin.current` explicitly, and
   `loadAladin()` dedupes the CDN script load, so StrictMode remounts and
   close/reopen cycles (including reopening during a pending load) are
   safe. ErrorBoundary, lazy chunk, pinned version/URL and SRI-less
   single-script load are unchanged. Result verified e2e: with the CDN
   blocked by `netGuard` the fallback alert "Sky overlay unavailable —
   could not load Aladin Lite from CDS." now renders.

2. **axe `label-content-name-mismatch` (serious x2)** — accessible names
   now start with the visible text:
   - `.csb-go` (`controls/CoordinateSearchBar.tsx`):
     `aria-label={loading ? "Loading…" : "Go: load time-lapse"}` — the
     label tracks the rendered text in both states.
   - Blink toggle (`viewer/TimeLapseCanvas.tsx`):
     `aria-label="A/B: toggle blink comparator"`.
   - Blink anchor (same file, defensive — only rendered with blink on):
     `aria-label={`A=${anchor + 1}: set blink reference to current frame`}`.
   The new strings keep the old phrases as substrings, so the existing
   Playwright locators ("Load time-lapse", "Toggle blink comparator",
   "Set blink reference to current frame") still match.

3. **WCAG 1.4.11 `--border-strong`** (`styles/tokens.css`) — measured at
   1.65:1 (light) / 1.79–1.89:1 (dark) against its surfaces. Now
   `#788089` light (>= 3.38:1 on bg/raised/inset/hover) and `#66707d` dark
   (>= 3.33:1 on all four). Decorative hairlines (`--border`) unchanged.
   `tests/layout/tokens.test.ts` `UI_PAIRS` now asserts
   `border-strong` vs bg/bg-raised/bg-inset/bg-hover at >= 3:1 in both
   schemes.

   Note on the reported `color-contrast` finding: on the merged f6 build
   axe-core 4.14 reports **zero** color-contrast violations — the listed
   elements (`.sp-title`, `.sp-pos`, band/aperture labels, aperture
   select, PERSIST badge) already meet 4.5:1 in the light scheme
   (measured: `--fg-dim` 7.89, `--fg-faint` 5.30, `--warn` 6.33 on the
   drawer surface). Verified by direct AxeBuilder runs at 375/768/1440 px,
   dark + light, home + spectrum open. No panel color changes were needed;
   the a11y e2e test now enforces zero serious/critical violations going
   forward.

4. **Molecular band label overlap** (`spectrum/prepare.ts` +
   `spectrum/SpectrumPanel.tsx`) — new pure `layoutBandLabels(rects,
   measure, maxRows = 2, gap = 6)`: anchors each label at its band's left
   edge, greedily assigns the first row whose previous label ends at
   least `gap` px earlier, and drops labels that fit on no row (the band
   shading, Bands toggle and citation remain). `drawBands` feeds it real
   `ctx.measureText` widths and draws row-staggered labels. Regression
   tests in `tests/spectrum/prepare.test.ts` (4 new cases, 178 vitest
   total).

### Test suite repairs (f6 redesign fallout)

- `modes.e2e.ts`: `/Det stack/` -> `/Stack/` (new label).
- `url-state.e2e.ts`: the old "FOV 0.05°" label text is gone; asserts the
  slider `Field of view in degrees` has value `0.05` plus the visible
  `0.05°` output.
- `spectrum.e2e.ts`: removed `test.fail()` (the overlap is fixed).
  `expectNotCovered` in `fixtures.ts` now calls
  `scrollIntoViewIfNeeded()` first — on the 375px viewport the Go button
  was never covered, it was below the fold after the drawer scrolls the
  panel into view (`elementFromPoint` returns null off-screen). The
  assertion's intent is preserved.
- `overlay.e2e.ts`, `a11y.e2e.ts`: removed `test.fail()` and the stale
  defect comments; both tests now pass for real.
- `tests/integration/app.test.tsx`: Testing Library `name` matching is
  exact — updated to `"Go: load time-lapse"`.
- `screenshots.e2e.ts` + `playwright.config.ts`: Playwright report,
  artifacts and screenshots now write under `dist/test-results/` —
  `eslint .` scans the whole `frontend/` dir and the flat config only
  ignores `dist`/`node_modules` (`eslint.config.js` is outside this
  task's scope), so reports in `test-results/` made every *second* run of
  the lint gate fail on the report's bundled JS. Under `dist/` the gate
  is idempotent. `.gitignore` gained `playwright-report/` (and keeps
  `test-results/` for stray output).

## Files

All inside the allowed scope: `src/components/overlays/AladinOverlay.tsx`,
`src/components/controls/CoordinateSearchBar.tsx`,
`src/components/viewer/TimeLapseCanvas.tsx`,
`src/components/spectrum/{SpectrumPanel.tsx,prepare.ts}`,
`src/styles/tokens.css`, `playwright.config.ts`, `.gitignore`,
`tests/e2e/{a11y,fixtures,modes,overlay,screenshots,spectrum,url-state}`,
`tests/integration/app.test.tsx`, `tests/layout/tokens.test.ts`,
`tests/spectrum/prepare.test.ts`.

## Verify commands and results

- `npx tsc --noEmit` — clean.
- `npx vitest run` — 178 passed / 12 files (was 174; +4 band-label tests).
- `npx eslint .` — clean, also after a Playwright run (artifacts under
  `dist/`).
- `npx vite build` — 259 kB (80.6 kB gz).
- `npx playwright test` — 30 passed, 2 skipped (@live opt-in), 0 failed,
  ~7 s. axe logs `[axe <scheme> home|spectrum] clean` for all 4 scans.

## Decisions / deviations

- Kept `aria-label`s (prefixed with the visible text) rather than
  dropping them, and chose new names that still contain the old phrases —
  the existing Playwright/Testing-Library locators needed no churn beyond
  the one exact-match string.
- The reported light-scheme `color-contrast` violations could not be
  reproduced on this build (see item 3): nothing needed darkening beyond
  `--border-strong`, which was a genuine failure. Documented here rather
  than silently "fixing" what already passed.
- Artifact relocation to `dist/test-results/` deviates from the literal
  "add to .gitignore" instruction's spirit but keeps every gate runnable
  repeatedly; `.gitignore` still covers both requested dirs.

## Open issues

- The Aladin "ready" path is not covered e2e (netGuard always blocks the
  CDN); only the error path is asserted. The code path is unchanged apart
  from now being reachable — worth one manual check with the CDN
  reachable before the demo.
- `@live` tests remain opt-in (`E2E_LIVE=1`) and were not run here.
