# Problem and solution

## Problem
Since May 2025 SPHEREx maps the whole sky every six months in 102 near-infrared bands and has imaged over a billion objects. Moving and variable sources (brown dwarfs, asteroids, comets, high-proper-motion stars, Mira and T Tauri variables) are found by comparing images taken at different times, as with Pluto and, later, the Backyard Worlds discoveries with WISE. But SPHEREx data lives in multi-extension FITS files behind archive APIs: only experts can look at it, and no single person or script can sift it all. There is no public, instant way to answer "what does SPHEREx show at this position, and does anything change?".

## Users
Citizen scientists, students and teachers, amateur astronomers, and professionals who want a quick look or an embeddable view.

## Solution: SPHEREx Blink
Type RA/Dec (sexagesimal or decimal) and a field of view; get a short time-lapse of that patch built live from the IRSA archive.
- Blink, difference (t2 - t1) and co-add modes to reveal motion and faint sources; frames grouped by detector/wavelength so comparisons are like-for-like.
- Click any pixel to get a spectrum assembled from every exposure covering it, with flag badges (saturation, persistence, ghosts, etc.) read from the FITS headers.
- Every setting lives in the URL, so a view can be shared or embedded in an iframe.
- Five official test targets as one-click presets: Barnard's Star, Epsilon Indi B, WISE 0855, RW Aurigae, R Draconis.
- New releases: collection names and endpoints are configuration, so DR1/QR3/future releases need no code change.
- Optional: Gaia DR3 / AllWISE overlay via Aladin Lite.

## Architecture
Rust (axum, tokio) backend: SIA2 search + cutout fetch from IRSA with bounded concurrency and a byte-capped cache, FITS parsing, ndarray science (inverse-variance stacking, differencing, aperture spectra). Vite + React + TypeScript frontend with WebGL2 rendering, served by the same binary. Pre-warmed cache for the five targets.

## Success criteria
A visitor with no astronomy background loads WISE 0855 from a link and sees it move within seconds; all five targets work; mobile and iframe views work; data sources, licenses and SI use are documented.
