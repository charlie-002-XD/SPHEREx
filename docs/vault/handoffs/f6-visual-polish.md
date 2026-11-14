# Handoff: f6-visual-polish (expert visual design pass)

Status: **complete**. All gates green on branch `opus/f6-visual-polish`.
Author: Claude Opus 5.5 (agent `spherex-frontend-designer`).

## Files

- `frontend/src/styles/tokens.css`: rewritten token set (palette, type, spacing, motion, layout metrics).
- `frontend/src/styles/base.css`: global defaults, custom range slider, tabular numerals, focus ring.
- `frontend/src/components/layout/app-shell.css`: layout rewrite, docked spectrum drawer, collapsible panel groups, states, embed. The old "token bridge" (`.app .tl-*`, `.app .csb-*`, `.app .ms-*`) is removed because the component CSS now uses tokens directly.
- `frontend/src/components/layout/AppShell.tsx`: new optional `drawer` prop. It renders inside `.app-view`, after `.app-main`. The header gets a small inline-SVG mark.
- `frontend/src/App.tsx`: `SpectrumPanel` is passed as `drawer` and is no longer a fixed sibling. Panel sections are collapsible `<details open>` with micro-label summaries ("View", "Display", "Current frame", "Overlay").
- `frontend/src/components/viewer/TimeLapseCanvas.tsx` + `viewer.css`:
  - inline-SVG transport icons
  - frame counter moved into the transport
  - overlay chips on a scrim
  - grouped transport (play group, scrubber, counter, aux group)
- `frontend/src/components/controls/CoordinateSearchBar.tsx` + `controls.css`:
  - labels drawn inside the fields
  - FOV value shown as `<output>`
  - compact grid (container query)
  - 44px segmented controls
- `frontend/src/components/controls/ModeSelector.tsx`: short labels (Raw / Diff / Co-add / Stack). The t₂−t₁ explanation moves into the `title` tooltip. Detector options get tooltips.
- `frontend/src/components/spectrum/SpectrumPanel.tsx`, `spectrum.css`, `uplot.ts`:
  - compact one-line header
  - chart sized from its host (240px floor)
  - token-based chart colors and font, rebuilt when the color scheme flips
  - loading skeleton
  - scroll-into-view on open
  - `uplot.ts` gains two typed axis options (`size`, `labelSize`)
- `frontend/src/components/metadata/metadata.css`, `overlays/overlays.css`: moved from the undefined `--sx-*` fallbacks to real tokens. Before this, these panels ignored the light scheme.
- `frontend/index.html`: theme-color and favicon updated to the new palette.
- `frontend/tests/layout/app-shell.test.tsx`: 9 new tests for the four defects.
- `frontend/tests/layout/tokens.test.ts` (new):
  - contrast of every token pair in both schemes
  - scrim legibility
  - "no literal colors" check over all component CSS
  - type scale
- `frontend/tests/integration/app.test.tsx`: the spectrum panel is docked inside `.app-view`/`.app-drawer` and contains neither Go nor Play.
- `docs/screens-design/`: before/after JPEGs (24 files, 1.2 MB).
- `SI_USAGE.md`: one row for the design work. This file is outside the declared path scope; the orchestrator asked for it.

## The four PM defects

1. **Spectrum panel covered Go and the controls.** The panel is now part of the layout, with no `position: fixed`.
   - 640px and up: docked under the image (`--drawer-h: clamp(18rem, 36dvh, 24rem)`). The image shrinks; nothing is covered.
   - 1100px and up in landscape (aspect ≥ 5:4): docked *beside* the image, in a 26rem column (`--drawer-w`). It uses the letterbox space the square cutout leaves on wide screens.
   - Phones: in-flow under the transport and above the search. It scrolls into view on open; nothing is ever on top of the playback controls.
   - Measured in all 18 combinations of 3 viewports × 2 schemes × 3 states: **0 controls covered** (`elementFromPoint` at the centre of every control). Before: 5 controls covered at 375px, 13 at 768/1440px.
2. **Chart too short.** The plot host fills the panel with a `--plot-min-h: 15rem` (240px) floor. uPlot is absolutely positioned inside the host, so the ResizeObserver cannot loop. Axes are compact and 12px (`size` 32/44, `labelSize` 20); the x label is "Wavelength (µm)". Measured: 240px at 375 and 768, 320px beside the image at 1440. Before: a 220px canvas including axes, about 110px of plot area.
3. **Segmented buttons at 40px.** `.ms-opt` uses `min-height` and `min-width` of `var(--target)` (44px) at every breakpoint. There are no boxes, only hairline dividers, and the active state is `--accent-soft` with a 2px accent underline. To fit All + D1–D6 at ≥ 44px each, the panel is 22rem (352px).
4. **Mobile balance.**
   - Order is: image, transport, spectrum (when open), search, sheet handle.
   - The image keeps a square floor of `min(100vw − 24px, 58dvh)` and grows into any spare height. At 375×800 the stage is 351×487.
   - Search is compact: `[RA][Dec][Go]` in one 44px row, then FOV. Labels sit inside the fields, so no label rows are needed.
   - The transport is two rows: play group + scrubber + counter, then loop, A/B and fps.

## Design decisions

- **Type**:
  - 4 steps: 12 / 14 / 16 / 20 px (`--text-xs/sm/md/lg`). Body text is 14px.
  - Weights 400 and 600 only.
  - Uppercase micro-labels (12px, 600, tracking 0.06em) only for section titles and in-field labels.
  - Tabular numerals (`.num`, text inputs, `<output>`) for coordinates, wavelengths, counters and fps. Coordinates use the mono stack.
  - Mobile text inputs are 16px, so iOS does not zoom on focus.
- **Spacing**:
  - 4/8 grid (`--space-1..6` = 4, 8, 12, 16, 24, 32).
  - Gutters are 12px on phones and 16px from 640px.
  - Groups are separated by proximity and hairlines, not boxes. The metadata card lost its box.
- **Palette (dark default)**:
  - Near-black neutral surfaces: `--bg #0b0d10`, `--bg-raised #111418` (panel and drawer), `--bg-inset #07090b` (stage and inputs).
  - Two border levels: hairline `--border #22272e`, and `--border-strong #3a414b` for inputs.
  - One calm accent, `#8ab4f8`. It is used only for Go/Retry, the active state, focus and the chart line.
  - Semantic colors are used only for data states: `--warn` for flags, `--danger` for errors.
- **Palette (light)**: equal structure. `--bg #f7f8fa`, raised `#fff`, accent `#2858b8`. Implemented as one override block under `prefers-color-scheme: light`.
- **Over-image scrim**: `--scrim rgb(0 0 0 / 0.7)` with `--scrim-fg` and `--scrim-warn`. It is identical in both schemes because the image can be any brightness.
- **Layout rules**:
  - The image is the largest element at every size.
  - Floating layers are only the target menu popover, the only one with a shadow.
  - Panels dock or push content.
  - Panel sections are collapsible (`<details open>`).
- **Controls**:
  - One select style: native `<select>` with `appearance: none` and a chevron drawn with CSS gradients from tokens, so there is no image file.
  - Native checkboxes with `accent-color`.
  - Custom range slider: 4px track, 14px thumb, 44px hit area. The thumb turns accent on focus.
  - Ghost transport buttons; Play is the one outlined control.
- **Motion**: 120ms (`--dur-1`) for hover and colour, 180ms (`--dur-2`) for chevrons and the panel entrance. Everything is disabled under `prefers-reduced-motion` (`base.css`, plus explicit `animation: none` on the skeletons).
- **States**:
  - Empty: crosshair icon, 20px title, one hint.
  - Error: danger icon, calm title, the API message, one Retry action.
  - Loading: a stage-shaped shimmer plus a bar where the transport will appear, so there is no layout jump.
  - Spectrum loading: an in-panel skeleton.
- **Embed (`?embed=1`)**: the stage has no border or radius and fills the iframe; a single-row transport sits below it.

## Contrast (WCAG 2.1, computed from tokens.css and asserted in `tokens.test.ts`)

| Pair | Dark | Light |
|---|---|---|
| `--fg` on `--bg` | 15.85 | 16.90 |
| `--fg` on `--bg-raised` | 15.04 | 17.96 |
| `--fg-dim` on `--bg` | 8.21 | 7.43 |
| `--fg-dim` on `--bg-raised` | 7.79 | 7.89 |
| `--fg-dim` on `--bg-inset` | 8.42 | 6.91 |
| `--fg-faint` on `--bg` | 5.80 | 4.98 |
| `--fg-faint` on `--bg-raised` | 5.51 | 5.30 |
| `--fg-faint` on `--bg-inset` | 5.95 | 4.64 |
| `--accent-fg` on `--accent` (Go, Retry) | 9.23 | 6.61 |
| `--fg` on `--accent-soft` (active segment) | 11.76 | 14.99 |
| `--accent` on `--accent-soft` (pressed transport) | 6.85 | 5.52 |
| `--accent` on `--bg` | 9.23 | 6.22 |
| `--danger` on `--bg` | 8.15 | 6.15 |
| `--danger` on `--bg-raised` | 7.73 | 6.54 |
| `--warn` on `--bg` | 12.75 | 5.96 |
| `--warn` on `--bg-raised` | 12.11 | 6.33 |
| `--focus` on `--bg` (UI, 3:1 needed) | 9.23 | 6.22 |
| `--scrim-fg` on scrim over a white pixel | 7.67 | same |
| `--scrim-warn` on scrim over a white pixel | 5.54 | same |

The minimum is 4.64 (light `--fg-faint` on `--bg-inset`, used for placeholders and in-field labels).

## Verification (real browser)

- Built app (`vite build` + `vite preview` on 127.0.0.1:4180). The "before" build was served on 4181. Both servers are stopped.
- **Playwright + Brave headless** (`color_scheme` emulation for dark and light):
  - URLs: `?mock=1&ra=133.795&dec=-7.245&fov=0.05`, plus `&mode=diff`, `&embed=1`, and the spectrum opened by clicking the canvas.
  - Sizes: 375×800, 768×1024 and 1440×900, all in both schemes.
  - Error state: `/api/**` fulfilled with 503. Loading state: `/api/**` left pending. Empty state: no `ra`.
  - Result in all 24 page/state combinations: horizontal overflow 0; interactive elements below 44px: **0** (before: 12 to 13 at ≥ 640px); controls covered by the spectrum: **0**.
- **Chrome via MCP** (own tab, closed afterwards):
  - Same-origin iframes at 375 and 768px.
  - In each iframe I clicked the canvas, opened the spectrum, and measured: overflow 0, small targets 0, covered 0, plot 240px.
  - For the light scheme, I took the `prefers-color-scheme: light` CSSMediaRule from the page's own stylesheet, injected its declarations onto `:root`, and read computed colours. Body changed from `rgb(11,13,16)` to `rgb(247,248,250)`; Go is `rgb(40,88,184)` on `#fff` text; panel is `#fff`.
- Screenshots in `docs/screens-design/`:
  - `before-*`: dark 375 default and spectrum, 768, 1440 default and spectrum, light 1440 spectrum.
  - `after-*`: the same set plus light 375/768, diff, embed 375/1440, loading, error and empty.

## Could not verify / open issues

- Real IRSA data was not checked; only `?mock=1` and synthetic error and loading routes were. Wide or non-square cutouts should still letterbox correctly (object-fit), but this is unverified.
- Firefox and Safari were not run. The range slider styles include `-moz-*` rules, and the container query (`@container search`) needs Safari 16+ or Firefox 110+.
- Molecular-band labels (H₂O and CH₄) overlap when the chart is narrow. Label placement lives in `prepare.ts`/the draw hook data and was not changed.
- Hairline borders (`--border`, `--border-strong`) are below 3:1 against their surfaces on purpose. Inputs are identified by their in-field label and filled background, not by the border alone. If a judge applies WCAG 1.4.11 strictly to input boundaries, raise `--border-strong`.
- The Chrome light-scheme check injected the tokens, which does not fire `matchMedia`, so the chart kept its dark ink there. Under real emulation (Playwright) the chart rebuilds with light ink, as the after-light screenshots show.
- `SkyExplorer.tsx` (unused by App) shares the tokenised CSS but was not visually reviewed.

## Verify commands and results (run in `frontend/`)

- `npx tsc --noEmit`: clean.
- `npx vitest run`: 12 files, 166 tests passed. 66 are new: 9 for the layout defects, 57 for tokens and contrast. The integration assertions were extended.
- `npx eslint .`: clean.
- `npx vite build`: OK (main CSS 5.1 kB gz).
