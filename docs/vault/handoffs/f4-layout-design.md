# Handoff: f4-layout-design (app shell + minimalist responsive design system)

Status: **done**. All gates green on branch `devin/f4-layout-design`.

## Files

New:
- `frontend/src/styles/tokens.css` — all design tokens: dark palette by default,
  light palette under `prefers-color-scheme: light` (one accent per scheme),
  spacing/radius/type scales (system font stacks), motion durations, layout
  metrics (`--target: 44px`, `--header-h`, `--panel-w: 320px`, `--rail-w`).
- `frontend/src/styles/base.css` — minimal reset, `html body` background/color
  (two-element selector to beat f1's hardcoded `body{background}` regardless
  of stylesheet order), `body{overflow:hidden}` (shell owns all scroll
  regions), visible `:focus-visible` ring, `prefers-reduced-motion` reset,
  `.visually-hidden` utility, `accent-color` for native controls.
- `frontend/src/components/layout/useMediaQuery.ts` — matchMedia hook safe
  without a DOM (SSR/tests), live subscription in the browser.
- `frontend/src/components/layout/TargetMenu.tsx` — header preset menu on a
  native `<details>` popover; closes on select, Escape and outside click.
- `frontend/src/components/layout/States.tsx` — `EmptyState` (crosshair SVG),
  `LoadingSkeleton` (single shimmer block, no spinners, reduced-motion safe),
  `ErrorState` (`role="alert"` + Retry).
- `frontend/src/components/layout/AppShell.tsx` — the shell.
- `frontend/src/components/layout/app-shell.css` — layout + token bridge.
- `frontend/tests/layout/app-shell.test.tsx` — 15 tests.

Edited: `frontend/src/App.tsx`, `frontend/src/main.tsx`, `frontend/index.html`
(viewport + `viewport-fit=cover`, description, dual `theme-color`, inline SVG
data-URI favicon, title "SPHEREx Blink — SPHEREx sky time-lapse", `lang="en"`).

## What was built

- Mobile (<640px, default layer): viewer on top, compact search row under it,
  secondary controls in a bottom sheet with a grab-handle toggle (closed by
  default). Desktop (>=640px, single `min-width` media query): search row
  under the header, image fills the main area, the same controls in a slim
  320px side panel (open by default) that collapses to a vertical rail.
- `AppShell` gets the open/closed default from `useMediaQuery("(min-width:
  640px)")`; the toggle is a real `<button>` with `aria-expanded` +
  `aria-controls`, and the closed body is `hidden` (unreachable by keyboard).
- `App.tsx` wires f1 pieces directly (not `SkyExplorer`, whose fixed order
  does not fit this layout): `useUrlState` + `useFrames` in an inner
  `Explorer` component that owns transport state; `apiBase` prop defaults to
  `import.meta.env.VITE_API_BASE ?? ""`, i.e. same-origin `/api/v1/*`.
- Target menu prefers `/api/v1/targets` (mapped through `formatRaDeg`/
  `formatDecDeg`), silently keeps `PRESET_TARGETS` when the backend is down.
- States: `idle` → EmptyState; `error` → ErrorState + Retry (Retry remounts
  `Explorer` via `key`, which re-runs the fetch effect — `useFrames` has no
  refetch API and hooks/ is outside scope); first load → skeleton;
  `success` with 0 frames → "No observations" EmptyState.
- Embed (`?embed=1`): only `.app-main` + f1's embed transport (prev/play/
  next/scrub); fills the iframe, `overflow:hidden`, no scrollbars.
- Token bridge (`.app .tl-*`, `.app .csb-*`, `.app .ms-*`): remaps f1's
  hardcoded dark colors to tokens so light scheme and AA contrast hold
  without touching f1 files; all controls bumped to >=44px targets;
  `.ms-group` gets `flex-wrap: wrap` (the 320px panel clipped D6 otherwise).

## Decisions / deviations

- Bottom sheet is implemented as an in-flow bottom region with a 44px
  handle button (task allows "drag handle or button"), not a drag-gesture
  overlay — simpler, keyboard operable, no backdrop.
- `.csb-presets` (preset pills inside CoordinateSearchBar) is hidden via
  CSS because presets now live in the header target menu; f1 files were
  read-only for this task.
- "Overlays" controls are not in the panel: no overlay feature exists yet
  (`src/components/overlays/` is empty). No dead UI added; the panel has a
  section-per-group structure so overlays slot in later.
- Mobile visual order (image above search) uses `order:-1`; DOM stays
  search-first so keyboard order matches the task flow.
- Retry remounts the fetch component (documented above) instead of
  modifying `useFrames`, which is outside scope.
- `SI_USAGE.md` unchanged: the existing Devin row already covers frontend
  components (same criterion as b1's handoff).

## Open issues

- Component tests are markup-level (`renderToStaticMarkup`) + CSS static
  assertions: `package.json`/vitest config are outside scope, so no jsdom.
  A real click-through test belongs to f3 (Playwright).
- Visual verification done ad hoc via Playwright+Brave headless (not in
  the repo): no horizontal overflow at 375/480/1280 px
  (`scrollWidth == innerWidth` everywhere), panel scrollWidth == clientWidth
  after the `.ms-group` wrap fix, dark + light schemes render correctly.

## Verify commands and results (run in `frontend/`)

- `npx tsc --noEmit` — clean.
- `npx vitest run` — 5 files, 58 tests passed (15 new layout tests).
- `npx eslint .` — clean.
- `npx vite build` — builds (dist ~74 kB gz JS, 3.5 kB gz CSS).
- `npx vite preview` + headless Chromium at 375/480/1280 px — no horizontal
  overflow; screenshots verified mobile sheet, desktop panel, embed and
  both color schemes.
