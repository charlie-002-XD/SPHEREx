# Handoff: f8-about-credits

Status: done. All gates green on branch `devin/f8-about-credits`.

## Files

- `frontend/src/components/about/AboutDialog.tsx` — header "About" button + native `<dialog>` (aria-labelledby) with: two-sentence intro, "How to read it" list (Blink A/B, Diff bright/dark spot, Co-add, Stack, click-for-spectrum), D1–D6 detectors + same-wavelength-only note, flags-legend note (legend lives under "Current frame"), keyboard table copied verbatim from `TimeLapseCanvas` (Space/K, arrows, Home/End, L, B), data credits with IRSA + Aladin Lite links, "Open source, Apache-2.0", "Built for the NASA Space Apps Challenge 2026". `TODO(team): repository link` left in the footer — the remote is not public yet.
- `frontend/src/components/about/FirstRunHint.tsx` — one-line dismissible hint under the header (`role="note"`). Shown only when `localStorage["spherex-blink.hint.dismissed"]` is unset; every storage access is try/catch so the page works with storage blocked (hint hides for the session). Hidden under `?embed=1` and `?mock=1` unless `?hint=1` forces it (existing e2e unaffected).
- `frontend/src/components/about/EmbedCredit.tsx` — tiny "SPHEREx Blink — NASA SPHEREx data via IRSA" link in embed mode; href = current URL minus `embed`, `target="_blank" rel="noopener noreferrer"`.
- `frontend/src/components/about/about.css` — all styles, tokens only; dialog scrolls inside `.about-body`, close/dismiss buttons are `var(--target)` (44px).
- `frontend/src/components/layout/AppShell.tsx` — new optional slots: `headerActions` (after TargetMenu), `notice` (under header), `credit` (embed mode footer). Defaults keep every existing render identical.
- `frontend/src/components/layout/app-shell.css` — `.app-actions` wrapper for the header right side.
- `frontend/src/App.tsx` — wires `<AboutDialog/>`, `<FirstRunHint/>`, `<EmbedCredit/>` into the shell.
- `frontend/eslint.config.js` — ignores `dist/**`, `test-results/**`, `playwright-report/**` (D-024 polish note).
- `frontend/tests/about/about.test.tsx` — 13 vitest/jsdom tests: open/close, Escape, cancel event, backdrop vs inside click, focus into/return, Tab trap fallback, content/credits/links, hint show/dismiss/persist, storage-blocked path, embed+mock+hint matrix, credit href.
- `frontend/tests/e2e/about.e2e.ts` — Playwright, accessible roles only: dialog content + links, Escape + focus restore, backdrop + Close, axe zero serious/critical with dialog open in dark and light, 375px usability (fits viewport, no overflow, body scrolls, close ≥44px), hint visibility rules + cross-reload persistence, embed credit.

## Decisions / deviations

- "How to read it" has 5 items, not 4: the task's own list enumerates blink, diff, co-add, stack and click-for-spectrum; all five were kept.
- jsdom 25 has no `HTMLDialogElement.showModal`/`close`; the component calls the native method when present and falls back to the `open` attribute + a manual Tab-cycle trap, so the same code path is testable in vitest and correct in browsers (native modal still provides trap/Escape/focus-restore there).
- `?hint=1` is read straight from `location.search` inside the component — `src/lib/urlState.ts` is outside this task's scope and unchanged.
- Embed credit is a `<p>` (not `<footer>`) so embed-mode landmark checks (`banner`/`contentinfo` counts) in existing e2e stay at zero.
- SI_USAGE.md records Devin-authored frontend work already; no change needed there (outside scope).

## Open issues

- Repository URL intentionally absent (see TODO in `AboutDialog.tsx`).
- Non-modal fallback (`open` attribute) does not inert the background — only relevant to browsers without `<dialog>` support (and jsdom); acceptable degradation.

## Verify

```
cd frontend
npx tsc --noEmit          # clean
npx vitest run            # 13 files, 191 tests pass (13 new)
npx eslint .              # clean
npx vite build            # ok, index ~82.7 kB gz
npx playwright test       # 48 pass, 2 skipped (@live opt-in); axe clean
```
