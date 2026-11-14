# Demo slide deck — 7-slide outline

Rules that constrain this deck: **≤ 7 slides total, including the
title/intro slide**; slides must show the team name, member names and the
chosen challenge; they must show the project image, the UX, and how NASA /
partner data make it possible. All text in English.

Screenshot placeholders: capture real UI screenshots after `integration-live`
and save them under `docs/submission/screenshots/` with the filenames below.
Speaker notes are ≤ 40 words each — keep them that way when rehearsing.

---

## Slide 1 — Title

- **Team name:** `TODO(team): <team name>`
- **Members:** `TODO(team): <member names, as registered on the team>`
- **Challenge:** Planet X and SPHEREx — NASA Space Apps Challenge 2026
- **Project:** SPHEREx Blink
- Tagline: "A time-lapse window into the SPHEREx all-sky survey."
- Image: `TODO(measured): screenshots/title-hero.png` — the app showing a
  colorful SPHEREx time-lapse frame as the backdrop.

**Speaker notes (≈30 words):**
We are `<team name>` and this is SPHEREx Blink, our answer to Planet X and
SPHEREx: a public web tool that turns the archive into a blink comparator
anyone can use.

---

## Slide 2 — The problem

- SPHEREx maps the whole sky every six months in 102 infrared bands, but the
  data lives in multi-extension FITS files behind archive APIs.
- Finding moving or variable objects means comparing epochs — until now an
  expert-only workflow.
- No public, instant answer to "does anything change at this position?".
- Image: `TODO(measured): screenshots/problem.png` — a plain FITS header or
  archive table screenshot, visually dry on purpose.

**Speaker notes (≈35 words):**
SPHEREx images the entire sky twice a year, but every product is an expert
format. Blinking epochs is how Pluto and the Backyard Worlds planets were
found — yet nobody without FITS skills could try it on SPHEREx.

---

## Slide 3 — The solution: blink the sky

- One sentence: type coordinates, get a time-lapse of every SPHEREx epoch.
- Playback 1–30 fps, scrubber, and a two-frame blink comparator.
- Comparison modes: raw, epoch difference, co-add, per-detector stack.
- Image: `TODO(measured): screenshots/timelapse.gif` or `main-view.png` —
  the viewer mid-play on WISE 0855; if a GIF, show a source visibly shifting
  between frames.

**Speaker notes (≈35 words):**
SPHEREx Blink does the expert workflow in the browser: we fetch every
exposure of your patch and play it as a time-lapse. Blink, difference or
stack — moving sources announce themselves.

---

## Slide 4 — It really works: WISE 0855

- WISE 0855-0714, a sub-brown dwarf, at the official challenge coordinates.
- `TODO(measured):` measured displacement across epochs (arcsec and pixels),
  number of epochs/frames returned, cold vs. warm load time.
- Image: `TODO(measured): screenshots/wise0855-blink.png` — two side-by-side
  or blinking frames with the source circled.

**Speaker notes (≈35 words):**
Here is WISE 0855, one of the challenge's own targets, at the published
coordinates. `<N>` epochs, fetched live: the source moves `<X>` arcseconds —
Planet-X-style discovery, in a link.

---

## Slide 5 — More than a movie: spectra, flags, overlays

- Click any pixel → spectrum assembled from every covering exposure, with
  error bars and molecular-ice band markers.
- Per-frame metadata: epoch, detector, wavelength, flag badges read from the
  FITS headers — not hardcoded.
- Optional Gaia DR3 / AllWISE catalog overlay.
- Image: `TODO(measured): screenshots/spectrum.png` — spectrum panel open
  over the viewer.

**Speaker notes (≈35 words):**
Click a pixel and we assemble its spectrum from every exposure, with
uncertainties and instrument flags read from the FITS headers themselves.
Gaia and AllWISE markers tell you what is already catalogued there.

---

## Slide 6 — How NASA data makes it possible

- IRSA SIA2 cone search + cutout service → every epoch, fetched live.
- FITS IMAGE / VARIANCE / FLAGS / ZODI / WCS-WAVE decoded in our own
  bounds-checked reader.
- Partner data: Gaia DR3 and AllWISE via CDS VizieR and Aladin Lite.
- Polite client: concurrency ≤ 3, retries, cache, deadline — IRSA stays a
  shared public service.
- Image: `TODO(measured): screenshots/architecture.png` — the pipeline
  diagram from the README, or an annotated screenshot of the flag legend.

**Speaker notes (≈35 words):**
Everything on screen is real archive data: IRSA's SIA2 and cutout services
serve the images, and we decode the full FITS product — variance, flags,
wavelength tables. Gaia and AllWISE come through CDS.

---

## Slide 7 — Anyone, anywhere: embed, share, grow

- Every view is a URL — share it, or embed the live viewer in an iframe.
- New SPHEREx releases are adopted by configuration, no code change.
- Open source, Apache-2.0. Try it: `TODO(team): <deployed URL>`.
- Image: `TODO(measured): screenshots/embed-or-mobile.png` — the embed view
  inside a webpage, or the app at 375 px.

**Speaker notes (≈35 words):**
A discovery view is a link, so teachers, amateurs and researchers share the
same sky. Future data releases need only configuration. SPHEREx Blink is
open source — the URL is on screen.

---

## Production notes (not part of the deck)

- Suggested tools: reveal.js / Markdown slides, Google Slides or plain HTML —
  any format the team can export to a public, login-free link.
- Keep each slide to one idea; the image is the hero (matches the app's own
  design rule).
- Verify the final public link in a private/incognito window before
  submitting (see `checklist.md`).
