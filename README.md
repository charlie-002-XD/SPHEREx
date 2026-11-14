# SPHEREx Blink

A public web tool that turns the NASA SPHEREx spectral image archive into a
time-lapse: enter any sky position (RA/Dec) and watch the same patch of sky
across every SPHEREx epoch — the classic "blink comparator" technique that
found Pluto, applied to the newest all-sky infrared survey.

Built for the NASA Space Apps Challenge 2026, challenge
[Planet X and SPHEREx](https://www.spaceappschallenge.org/2026/challenges/planet-x-and-spherex/).

<!-- TODO(measured): replace with real screenshot files after integration-live -->
<!-- Screenshot placeholder: docs/submission/screenshots/main-view.png (TODO(measured): capture) -->
<!-- Screenshot placeholder: docs/submission/screenshots/spectrum.png (TODO(measured): capture) -->
<!-- Screenshot placeholder: docs/submission/screenshots/mobile.png (TODO(measured): capture) -->

## What it does

- **Time-lapse from live archive data.** Type RA/Dec (decimal or sexagesimal)
  and a field of view (0.01–0.5°); the backend queries IRSA with a SIA2 cone
  search, downloads image cutouts for every SPHEREx exposure covering the
  position, and streams them to the browser as float32 frames sorted by
  observation time. Playback 1–30 fps with play/pause, loop, frame scrubber
  and a two-frame blink comparator.
- **Like-for-like comparison modes.** `raw` (each exposure as observed),
  `diff` (later minus earlier epoch), `coadd` (inverse-variance mean) and
  `stack` (per-detector stacks). Frames are grouped by detector array (D1–D6)
  and wavelength so motion and faint sources show up against a stable
  background.
- **Click for a spectrum.** Clicking any pixel assembles a spectrum from
  every exposure covering it — aperture-averaged surface brightness per
  wavelength with 1σ error bars from the VARIANCE extension, molecular-ice
  band shading (H₂O, CH₄, CO₂, CO) and per-detector summaries.
- **Honest metadata.** Each frame shows observation time, detector, central
  wavelength, collection and flag badges (saturation, persistence, ghosts…)
  whose names come from the FITS `FLAGS` header legend itself, never
  hardcoded.
- **Catalog overlays.** Optional Gaia DR3 and AllWISE catalog markers via
  Aladin Lite (loaded on demand; the app works fully without it).
- **Shareable state.** Coordinates and every display setting live in the URL,
  so a view can be linked or embedded in an iframe (`?embed=1`).
- **Five official presets.** Barnard's Star, Epsilon Indi B, WISE 0855
  (official challenge coordinates), RW Aurigae and R Draconis — the targets
  named by the challenge — are one click away.
- **Mobile-first.** Responsive from 375 px to desktop and inside iframes;
  dark neutral theme, WCAG AA contrast, `prefers-reduced-motion` and
  `prefers-color-scheme` respected, full keyboard controls.
- **Offline demo mode.** `?mock=1` renders the entire UI from deterministic
  synthetic data — no backend or network needed (useful for demos and tests).

## Quick start

Prerequisites: Rust (stable), Node.js 20+ and npm. Docker is optional.

### Backend only (API on :8080)

```sh
cd backend
cargo run
# GET http://localhost:8080/api/v1/health
```

### Frontend dev server

```sh
cd frontend
npm ci
npm run dev          # Vite dev server; /api is proxied to localhost:8080
```

### Production-style local run (one binary serves everything)

```sh
cd frontend && npm ci && npm run build
cd ../backend && STATIC_DIR=../frontend/dist cargo run --release
# open http://localhost:8080
```

### Docker (single image, hosting-neutral)

```sh
docker build -t spherex-blink .
docker run --rm -p 8080:8080 spherex-blink
# open http://localhost:8080
```

See [`docs/DEPLOY.md`](docs/DEPLOY.md) for the full environment-variable
table and per-host guides (Render, Fly.io, Azure Container Apps, Railway,
plain VPS), plus the FortiGuard classification step for judges.

## URL parameters

Every view is a link. All parameters are validated; malformed values fall
back to defaults (never crash, never echo raw input back).

| Parameter | Values | Default | Meaning |
|---|---|---|---|
| `ra` | decimal deg or sexagesimal (`08h55m10.831s`) | — | Right ascension, `0 ≤ ra < 360` |
| `dec` | decimal deg or sexagesimal (`-07d14m42.53s`) | — | Declination, `-90 ≤ dec ≤ 90` |
| `fov` | `0.01`–`0.5` | `0.05` | Field of view, degrees |
| `mode` | `raw`, `diff`, `coadd`, `stack` | `raw` | Comparison mode |
| `stretch` | `linear`, `log`, `asinh`, `zscale` | `linear` | Display stretch |
| `cmap` | `grayscale`, `viridis`, `inferno`, `coolwarm` | `grayscale` | Colormap (`coolwarm` is diverging, suited to `diff`) |
| `fps` | int `1`–`30` | `5` | Playback speed |
| `frame` | int `≥ 0` | `0` | Initial frame index |
| `embed` | `1` or `true` | off | Chromeless iframe mode (canvas + transport only) |
| `mock` | `1` | off | Synthetic data; no backend needed |

Example — WISE 0855 blinking at 10 fps:
`?ra=08h55m10.831s&dec=-07d14m42.53s&fov=0.05&mode=raw&fps=10`

## API

The Rust backend exposes a small JSON API (pixel data as row-major float32
little-endian, base64):

- `GET /api/v1/health` — service status and configured collections.
- `GET /api/v1/targets` — the five preset targets.
- `GET /api/v1/frames?ra=…&dec=…&fov=…&mode=…&detector=…&max_frames=…&subtract_zodi=…` —
  time series of cutouts, sorted by observation time.
- `GET /api/v1/spectrum?ra=…&dec=…&aperture_px=…` — spectrum at a sky
  position from all covering exposures.

The OpenAPI contract in [`shared/openapi-contract.json`](shared/openapi-contract.json)
is the single source of truth for requests, responses and error codes.

## Architecture

```
Browser (React, WebGL2 canvas, uPlot)          IRSA (NASA/IPAC)
        │  GET /api/v1/*                              ▲
        ▼                                             │ SIA2 + cutouts
axum API ──► validation ──► SIA2 cone search ──► cutout fetch
                                   │                  (concurrency ≤ 3,
                                   ▼                   retry + backoff,
                          byte-capped cache ◄────────── 40 s deadline)
                                   │
                                   ▼
                 FITS reader (MEF, RICE tiles, TAN+SIP WCS,
                 WCS-WAVE wavelength map, FLAGS legend)
                                   │
                                   ▼
              science layer: reproject → diff / coadd /
              stack / aperture spectrum → float32+base64
```

- **Backend** (`backend/`): Rust, axum + tokio. Hand-rolled, bounds-checked
  multi-extension FITS reader (RICE_1 tile decompression, TAN+SIP celestial
  WCS, WCS-TAB wavelength tables). Science on `ndarray`: reprojection onto a
  north-up output grid (max 192 px), inverse-variance stacking, epoch
  differencing, aperture photometry. Bounded concurrency against IRSA,
  exponential backoff on 503/429, weighted in-memory cache (`MAX_CACHE_MB`),
  per-request deadline returning partial results, per-IP rate limit, gzip,
  CORS and a `frame-ancestors *` CSP so embedding works.
- **Frontend** (`frontend/`): Vite + React + strict TypeScript, no UI
  framework. WebGL2 renderer (R32F texture, stretch + colormap in GLSL) with
  a Canvas2D fallback; zod validation at every API boundary; uPlot for the
  spectrum chart (bundled); Aladin Lite loaded lazily behind an error
  boundary.
- **Deployment**: one container — the backend binary serves the built
  frontend and the API from a single origin ([`Dockerfile`](Dockerfile),
  [`docker-compose.yml`](docker-compose.yml), [`docs/DEPLOY.md`](docs/DEPLOY.md)).

## Adding a new data release

No code changes needed — collections and the archive root are configuration:

```sh
IRSA_COLLECTIONS=spherex_dr1,spherex_qr3,spherex_dr2 \
IRSA_BASE_URL=https://irsa.ipac.caltech.edu \
cargo run --release
```

`IRSA_COLLECTIONS` is a comma-separated list queried in order (results are
deduplicated across collections). Anything else — timeouts, retries, cache
size, rate limits, pre-warmed targets (`WARM_TARGETS`) — is documented in
[`docs/DEPLOY.md`](docs/DEPLOY.md).

## Data sources and credits

- **NASA/IPAC Infrared Science Archive (IRSA)** — SPHEREx spectral images
  via the SIA2 service and the IRSA cutout service (`spherex_dr1`,
  `spherex_qr3` collections). Data remain © their respective owners; see the
  archive terms.
- **Gaia DR3** (ESA) and **AllWISE** (NASA/WISE) — catalog overlays queried
  through CDS services (VizieR) via Aladin Lite.
- **Aladin Lite** (CDS, Université de Strasbourg, LGPL-3.0), **uPlot**
  (MIT), plus the Rust crates and npm packages listed with versions and
  licenses in [`docs/REFERENCES.md`](docs/REFERENCES.md).
- SPHEREx documentation and tutorials from IPAC/Caltech; molecular-ice band
  positions from the peer-reviewed literature — all cited in
  [`docs/REFERENCES.md`](docs/REFERENCES.md).

## Limitations

- **Coverage is the archive's.** Current SPHEREx releases do not cover the
  whole sky yet; positions without observations return "no observations".
- **First views are live archive queries.** A cold, uncached position can
  take seconds to tens of seconds; the server enforces a deadline and returns
  partial frame sets marked `truncated` rather than failing. Pre-warmed and
  cached positions are instant.
- **Like-for-like only.** `diff`/`coadd` combine exposures of the same
  detector within a wavelength tolerance — different wavelengths are never
  subtracted from each other.
- **The spectrum is aperture photometry** assembled from archive cutouts
  (mean surface brightness per exposure/wavelength with propagated
  variance), not a pipeline-extracted calibrated spectrum.
- **The catalog overlay needs the CDS CDN** at runtime; on networks that
  block it, the overlay degrades gracefully and the viewer keeps working.
- **The cache is in-memory** and per-process; it does not persist across
  restarts.
- WISE 0855 proper-motion displacement across SPHEREx epochs:
  `TODO(measured):` fill after `integration-live`.

## License

[Apache-2.0](LICENSE). Third-party components keep their own licenses — see
[`docs/REFERENCES.md`](docs/REFERENCES.md). Built with assistance from SI
tools, disclosed in [`SI_USAGE.md`](SI_USAGE.md).
