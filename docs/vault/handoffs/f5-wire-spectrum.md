# Handoff: f5-wire-spectrum (wire spectrum, metadata, overlay into the app; bundle uPlot)

Status: **complete**. All gates green on branch `devin/f5-wire-spectrum`.

## Files

- `frontend/src/App.tsx` — wiring (see "What was built").
- `frontend/src/components/spectrum/uplot.ts` — rewritten: uPlot is now a
  bundled npm dependency (`uplot@1.6.32`, MIT, pinned exact) loaded lazily
  via `import("uplot")` + `import("uplot/dist/uPlot.min.css")`. The typed
  surface (`UPlotConstructor`, `UPlotOptions`, `UPlotInstance`, `UPlotData`,
  `loadUPlot()`, `UPLOT_VERSION`) is unchanged, so `SpectrumPanel.tsx`
  needed no edits. All CDN/SRI code removed; a failed dynamic import can
  be retried (pending promise reset on rejection).
- `frontend/src/components/layout/app-shell.css` — added `.app-stage`
  (click-to-spectrum wrapper that fills `.app-main`), `.app-check`
  (checkbox row, 44px target), `cursor: crosshair` on `.tl-canvas`.
- `frontend/package.json` / `package-lock.json` — `uplot@1.6.32`
  dependency; dev deps `jsdom@25.0.1`, `@testing-library/react@16.1.0`,
  `@testing-library/dom@10.4.0` (all pinned exact, matching repo style).
- `frontend/tests/integration/app.test.tsx` — 3 jsdom integration tests.
- `docs/REFERENCES.md` — one-line update: uPlot is bundled via npm, no
  longer loaded from jsDelivr (outside the declared path scope but
  explicitly required by the task; flagged here).

`frontend/vite.config.ts` needed **no change**: the jsdom environment is
declared per-file via the `// @vitest-environment jsdom` pragma in the
integration test.

## What was built

- **Click -> spectrum**: `Explorer` wraps `TimeLapseCanvas` in
  `.app-stage` and attaches `canvasClickToSpectrum({frame, width, height},
  spec.requestAt)` from f2 (only canvas-target clicks are converted;
  disabled in embed mode). `App` owns `useSpectrum(client)` and renders
  `SpectrumPanel` as a sibling of `AppShell` (it is `position:fixed`).
- **Metadata**: `Explorer` reports its `FramesResult` up to `App` via a new
  `onResult` callback (the f4 remount-on-Retry mechanism is preserved;
  `useFrames` untouched). The secondary-controls panel now has a "Current
  frame" section with `FrameMetadata` (which renders `FlagBadges`
  internally) fed by the clamped current frame + `flag_legend`.
- **Overlay**: new "Overlay" section with a checkbox (default off) that
  mounts `AladinOverlay` through `React.lazy` + `Suspense` — the chunk is
  only downloaded when enabled. `defaultOpen` is passed so the inner
  Aladin section is already expanded. Position comes from the frames
  response `query` first, else the parsed search-bar coordinates; a hint
  is shown when no position is available. Gaia DR3 / AllWISE toggles live
  inside `AladinOverlay` as delivered by f2.
- **Mock mode**: `?mock=1` (read once from `location.search` at mount) or
  the existing `mockData` prop builds `makeMockFramesResponse()` and a
  mock `SpectrumClient` returning `makeMockSpectrum()` — full UI without
  a backend, including click-to-spectrum. Without mock, production keeps
  the f4 error state (unreachable backend -> `ErrorState` + Retry).
- **uPlot**: bundled (build emits a lazy `uPlot.esm-*.js` chunk, 23 kB
  gz, loaded only when the spectrum panel has data). Aladin Lite stays
  CDN-loaded behind `OverlayErrorBoundary`, unchanged.
- **Mobile floor fix**: with the sheet open on 375x667 the panel is now
  ~412px tall and the cutout collapsed to 0px. `.app-body` scrolls on
  mobile, `.app-view` keeps `flex-basis:auto` (never shrinks below its
  content) and `.app-main` gets a `min-height: 15rem` floor; all three
  are reset in the >=640px layer. Verified live: canvas keeps ~110px and
  clicks still open the spectrum sheet.

## Decisions / deviations

- **`overlay`/`spectrum` URL keys: NOT added.** `src/lib/urlState.ts` is
  outside this task's scope, so both remain local React state
  (`overlayOn` in `App`, `open` inside `useSpectrum`). `mock` is likewise
  read directly from `location.search` once at mount — it disappears from
  the URL on the first `setUrl` rewrite but mock mode persists for the
  session. Add the keys when urlState.ts is in scope.
- `docs/REFERENCES.md` edited outside the declared path scope because the
  task explicitly requires it (same justification f2 used via rule 8).
- The click handler is attached to a wrapper div inside `Explorer`
  because `AppShell`'s `<main>` cannot take handlers from outside; the
  handler itself already filters for clicks on the cutout canvas only.
- `SpectrumPanel` mounts the real `uplot` package in vitest/jsdom; chart
  construction failures (no canvas 2D, no ResizeObserver) hit the
  existing `.catch` -> `plotError` graceful path, which the integration
  test tolerates while asserting the data summary.
- `package.json` dev deps were added for the integration test (task says
  "testing utilities already in the repo" but only vitest was present).

## Open issues

- Carried over from f2: the Aladin Lite CDN URL is still unverified from
  this environment (no SRI); worth one live check before the demo.
- uPlot axes colors are read once at chart creation; a theme switch while
  the panel is open needs a reopen (unchanged from f2).
- `spectrum/types.ts` still duplicates the contract schemas; merge into
  `src/lib/types.ts` at integration time (f2 note).
- `npm install` warns that the `esbuild` postinstall script is blocked by
  npm allowScripts on this machine; harmless — `@esbuild/linux-x64`
  ships the binary and `vite build` works.
- The mobile bottom sheet is in-flow: when it is open the page scrolls so
  the sheet content is reachable; the image keeps a ~15rem floor rather
  than staying fully on screen. A drag-dismiss or overlay variant is a
  possible f3+ polish item.

## Verify commands and results (run in `frontend/`)

- `npx tsc --noEmit` — clean.
- `npx vitest run` — 11 files, 100 tests passed (3 new integration tests:
  `?mock=1` renders frames + metadata; canvas click opens spectrum panel
  with mock data; overlay toggle mounts/unmounts the Aladin section).
- `npx eslint .` — clean.
- `npx vite build` — `index-*.js` 79.8 kB gz, `uPlot.esm-*.js` 23.2 kB gz
  (lazy), `AladinOverlay-*.js` 2.0 kB gz (lazy), CSS 4.3 kB gz.
- Live check (vite preview + headless Brave, 375x667 and 1280x800):
  no horizontal overflow; canvas click opens the spectrum panel on both;
  overlay toggle mounts `section.ao`; sheet-open canvas keeps ~110px and
  remains clickable.
