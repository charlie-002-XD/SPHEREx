# Handoff: f2-spectrum (spectrum panel, flag badges, metadata, Aladin overlay)

Status: **complete**. All gates green on branch `devin/f2-spectrum`.

## Files

### `frontend/src/components/spectrum/`
- `types.ts` — zod `SpectrumPoint`/`SpectrumResponse` schemas mirroring the
  contract (duplicated from what would belong in `src/lib/types.ts`, which is
  outside scope) + `APERTURE_PX = [1,3,5]`.
- `prepare.ts` — pure data prep: `MOLECULAR_BANDS` (H2O 2.95–3.20, CH4
  3.25–3.40, CO2 4.20–4.35, CO 4.60–4.75 µm; cited Boogert+2015 ARA&A 53,541
  and Ehrenfreund & Charnley 2000 ARA&A 38,427), `prepareSpectrum` (sort by λ,
  non-finite flux → null gap, non-positive/NaN sigma → null),
  `detectorsPresent`, `flagCounts`, `bandRects` (band shading math, handles
  reversed axes and off-screen bands).
- `pick.ts` — `pixelToSky` (linear WCS from the contract: 1-based crpix, CD
  matrix, RA wrapped to [0,360)), `clientPointToPixel`, `canvasPickToSky`.
- `spectrumApi.ts` — `SpectrumClient` interface + `createSpectrumClient(apiBase)`
  (injected base URL, zod-validated, `ApiError` with contract codes).
- `mockSpectrum.ts` — deterministic synthetic spectrum (continuum + H2O-like
  dip, planted NaN flux/sigma) for dev/tests.
- `uplot.ts` — lazy loader for uPlot **1.6.32** (pinned) from jsDelivr with
  SHA-384 SRI + `crossorigin`, plus minimal typed API surface.
- `useSpectrum.ts` — `useSpectrum(client)` controller (abort/race guard,
  aperture refetch) and `canvasClickToSpectrum(ctx, onPick)`, the click glue.
- `SpectrumPanel.tsx` — bottom sheet ≤640 px / side panel >640 px, fixed
  position; band toggle, aperture select (1/3/5 px), uPlot chart with `draw`
  hook error bars and `drawClear` band shading, detector/flag summary, Escape
  to close, graceful fallbacks (empty, NaN, chart-load failure).
- `spectrum.css` — tokens `var(--sx-*, fallback)`; no hardcoded colors.

### `frontend/src/components/metadata/`
- `FlagBadges.tsx` — chips with flag names verbatim; `flagBitFor` derives
  name→bit only from the API `flag_legend` (no hardcoded bit meanings).
- `FrameMetadata.tsx` — obs time (ISO + MJD), detector, λ (+Δλ), collection,
  unit, zodi flag, badges, collapsible raw `flag_legend` list.
- `metadata.css` — same token strategy.

### `frontend/src/components/overlays/`
- `aladin.ts` — lazy loader for Aladin Lite **3.8.2** (pinned,
  `api/v3/3.8.2/aladin.js`; CDN dir drops the tag's `v` prefix — verified via
  Wayback), minimal typed `window.A` surface, `CATALOGS` defs (Gaia DR3 =
  `I/355/gaiadr3`, AllWISE = `II/328/allwise`) and attribution strings.
- `AladinOverlay.tsx` — collapsible section, AllWISE HiPS survey, view
  follows `raDeg/decDeg/fovDeg`; per-catalog toggles rebuild cone queries
  (position quantized to 1e-4° to avoid refetch churn); footer attributions;
  exported component wrapped in `OverlayErrorBoundary`.
- `ErrorBoundary.tsx`, `overlays.css`.

### `frontend/tests/spectrum/` (5 files, 39 tests)
Mock payloads incl. empty points, NaN flux/sigma (schema rejects NaN on the
wire — JSON can't carry it — and the display layer tolerates it), unsorted λ,
invalid sigma, rect scaling, WCS rotation, RA wrap, fetch mocked via
`vi.stubGlobal`.

## Decisions / deviations
- **package.json is outside scope** → uPlot and Aladin Lite are loaded lazily
  from pinned CDN URLs instead of npm deps (same choice the task makes for
  Aladin). uPlot has SRI integrity; Aladin Lite does not (CDS host was
  unreachable from this machine today so the bytes could not be hashed — the
  documented URL is correct). To vendor later: `npm i uplot@1.6.32`, replace
  `loadUPlot()` with a static import.
- **`frontend/src/styles/tokens.css` does not exist yet** (f4's task) → all
  CSS uses `var(--sx-*, fallback)`; fallbacks mirror the f1 palette. Tokens
  consumed: `--sx-fg --sx-fg-dim --sx-bg --sx-bg-panel --sx-bg-raised
  --sx-border --sx-border-soft --sx-accent --sx-warn --sx-danger
  --sx-band-fill --sx-band-stroke`. f4 should define them in tokens.css.
- `docs/REFERENCES.md` was updated (outside the component scope) because rule
  8 requires citing every library/data source added.
- Aperture selector (1/3/5 px) added to the panel — the contract supports it
  and it costs one `<select>`.
- Overlay FOV is `max(fov*4, 0.02)` so the Aladin view has useful context
  around the cutout; catalog cone radius = half the FOV diagonal.
- No jsdom/testing-library (package.json out of scope), so tests cover the
  pure logic the components are built on — same approach as f1.

## Integration (for f4 / SkyExplorer)
```tsx
const spec = useSpectrum(apiBase ? createSpectrumClient(apiBase) : null);
const current = frames[frameIndex] ?? null;
// on the element wrapping <TimeLapseCanvas>:
onClick={canvasClickToSpectrum(
  { frame: current, width, height }, spec.requestAt)}
<SpectrumPanel {...spec} onApertureChange={spec.setAperture} onClose={spec.close} />
<AladinOverlay raDeg={ra} decDeg={dec} fovDeg={fov} />
<FrameMetadata frame={current?.meta ?? null} unit={resp?.unit}
               flagLegend={resp?.flag_legend} />
```
`TimeLapseCanvas` needs no changes: the click handler filters for events
whose target is the cutout `<canvas>` (its metadata overlay already has
`pointer-events: none`).

## Open issues
- Aladin CDN URL is correct per official docs but could not be smoke-tested
  from this environment today (CDS returned a "Page unavailable" page; SRI
  therefore not set for Aladin). Worth one live check before demo.
- uPlot axes/series colors are read once at chart creation from CSS custom
  properties; a live theme switch while the panel is open needs a chart
  rebuild (reopen the panel).
- The panel is `position: fixed` (viewport sheet); if the app is ever embedded
  side-by-side with other widgets, f4 may prefer absolute positioning inside
  the app shell — the CSS is self-contained in `spectrum.css`.
- `spectralResponseSchema` duplicated in `spectrum/types.ts`; merge into
  `src/lib/types.ts` at integration time.

## Verify commands and results (run in `frontend/`)
- `npx tsc --noEmit` — clean.
- `npx vitest run` — 9 files, 82 tests passed (39 new).
- `npx eslint .` — clean.
