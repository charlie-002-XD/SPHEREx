# Handoff: f1-viewer (time-lapse viewer + controls)

Status: complete. All gates green on branch `devin/f1-viewer`.

## What was built

Stateless, hook-driven React components consuming payloads typed by zod schemas
that mirror `shared/openapi-contract.json` (`src/lib/types.ts`). The API base URL
is always injected (`apiBase` prop); never hardcoded. A `mockData` prop /
`useFrames` fallback (`src/lib/mock.ts`) renders the full pipeline offline.

### lib/
- `types.ts` — zod schemas for Frame, FramesResponse, Target, ErrorBody, ApiMode.
- `base64.ts` — pure-JS base64 codec for float32-LE pixel payloads (no DOM/Buffer).
- `coords.ts` — RA/Dec parsing (decimal + sexagesimal hms/dms) and formatting,
  FOV validation, `mjdToIso`. Ranges identical to the backend contract.
- `stretch.ts` — Linear/Log/Asinh/ZScale. Percentile (1-99) intervals for
  linear/log/asinh; IRAF-style zscale (sorted-sample least-squares with 2.5-sigma
  rejection, 5 iterations, contrast 0.25); `symmetricInterval` for diverging maps;
  NaN/Inf-safe throughout.
- `colormaps.ts` + `colormapData.ts` — Grayscale, Viridis, Inferno, CoolWarm
  (diverging, for diff mode). Data generated from matplotlib v3.9.4
  `_cm_listed.py` / `_cm.py` (BSD-3; CoolWarm design by K. Moreland, ISC 2009).
- `gl.ts` — `WebGL2Renderer` (R32F texture, stretch + colormap in GLSL ES 3.00
  shader, `isnan` masking) and `Canvas2DRenderer` fallback. Capability gate is an
  R32F upload probe; missing float-texture support → 2D canvas.
- `urlState.ts` — pure parse/serialize for `ra, dec, fov, mode, stretch, cmap,
  fps, frame, embed`; per-field zod/coords validation, defaults on any malformed
  value.
- `api.ts` — `fetchFrames`/`fetchTargets` against an injected base; `ApiError`
  carries the contract error code.
- `mock.ts` — deterministic synthetic FramesResponse (moving Gaussian source,
  detector cycle D1-D6, planted NaN/Inf pixels); schema-validated in tests.
- `presets.ts` — the five official targets. WISE 0855 uses the official challenge
  coordinates; the other four come from the team-plan table.

### hooks/
- `useUrlState.ts` — two-way URL sync via `history.replaceState` + `popstate`.
- `useFrames.ts` — fetch + base64-decode with AbortController, race guard, and a
  mock short-circuit; undecodable frames are skipped, not fatal.

### components/
- `viewer/TimeLapseCanvas.tsx` — rAF playback 1-30 fps, play/pause, loop,
  scrubber, blink comparator (anchor "A=current", alternates at fps while
  playing), metadata overlay (UTC date, D#, λ µm, index, flag names), keyboard
  (space/k, arrows, Home/End, l, b), aria labels + `aria-live` status. `embed`
  keeps canvas + play/step/scrub only.
- `viewer/SkyExplorer.tsx` — composition root: search bar, mode selector,
  stretch/cmap selects, fetch status, truncation notice. `embed` (prop or
  `embed=1` URL) hides all chrome.
- `controls/CoordinateSearchBar.tsx` — RA/Dec inputs, FOV slider (0.01-0.5°),
  five preset pills; validation errors via `role="alert"`, no raw input echoed.
- `controls/ModeSelector.tsx` — radio groups for raw/diff/coadd/stack and
  All/D1-D6 detector filter.
- `viewer.css`, `controls.css` — dark theme, flex-wrap layout usable at 375 px.

## Decisions / deviations
- Playing/loop/blink are local component state (not in the URL) — the task's URL
  key list does not include them and restoring a "playing" URL would be jarring.
- Detector filter is local state for the same reason (not in the URL key list);
  it still maps to the `detector` query parameter on refetch.
- WebGL path requires only core WebGL2 (R32F *sampling* needs no extension;
  NEAREST filtering avoids `OES_texture_float_linear`). The spec's
  `EXT_color_buffer_float` gate is implemented as an R32F upload probe, which is
  strictly stronger — it catches drivers that expose the extension but fail on
  upload.
- `serializeUrlState` omits keys equal to the defaults (cleaner shareable URLs).
- Blink comparator: A is an explicit anchor (default frame 0, "A=n" button pins
  the current frame); B is the scrubbed frame; alternation runs at `fps` while
  playing and shows A when paused.
- `SkyExplorer` is a composition component, but `App.tsx`/`main.tsx` were NOT
  touched (outside scope). Integration: render
  `<SkyExplorer apiBase={import.meta.env.VITE_API_BASE ?? ""} />` (or `mockData`)
  from `App.tsx`; also import a global dark background (`viewer.css` sets
  `body { background }` when SkyExplorer's stylesheet is loaded).

## Open issues
- No component-level tests (jsdom/react-testing-library not installed and
  `package.json` is outside scope). Pure logic (stretch, coords, urlState,
  base64/mock) is covered by 43 vitest cases.
- ZScale is an IRAF-style approximation (documented), not a port of
  numdisplay — worth a visual check against the real FITS cutouts in wave 2/3.
- Row-0-at-top orientation assumption in `gl.ts`; if IRSA cutouts need FITS
  bottom-up display, flip `v_uv.y` in the vertex shader.
- Preset coordinates for the four non-WISE targets still need the planned SIMBAD
  check (owned by the team plan, not this task).

## Verify commands and results (run in `frontend/`)
- `npx tsc --noEmit` — clean.
- `npx vitest run` — 4 files, 43 tests passed.
- `npx eslint .` — clean.
- `npm ci` note: environment npm 12 warns on node 20 but install + gates work.
