# Handoff: b2-irsa-client

Status: **done**. All gates green. Verified end-to-end against live IRSA
(opt-in test `SPHEREX_LIVE=1`).

## Files

- `backend/src/irsa/mod.rs` — public API surface + module docs.
- `backend/src/irsa/config.rs` — `IrsaConfig` + `from_env()`
  (`IRSA_BASE_URL`, `IRSA_COLLECTIONS` default `spherex_dr1,spherex_qr3`,
  `IRSA_MAX_CONCURRENCY` 3, `MAX_FRAMES` 60, `MAX_CACHE_MB` 200, plus
  `IRSA_TIMEOUT_SECS` 15, `IRSA_MAX_ATTEMPTS` 3, `IRSA_RETRY_BASE_MS`
  400, `IRSA_MAX_BODY_MB` 64).
- `backend/src/irsa/error.rs` — `IrsaError` (thiserror), retry
  classification (`Attempt`), jittered exponential `backoff`, and
  `From<IrsaError> for AppError` (see mapping below).
- `backend/src/irsa/votable.rs` — hand-written minimal VOTable/XML
  reader (no XML crate available in scope): elements, attributes,
  CDATA, comments, PIs, DOCTYPE with internal subset, entity decoding,
  namespace-prefix stripping. Emits `Vec<Votable>` (fields + rows).
- `backend/src/irsa/sia.rs` — `Observation`, `pos_parameter`,
  `search_radius_deg` (half-diagonal of the FOV square),
  `em_to_um`/`time_to_mjd` unit conversion driven by FIELD `unit`,
  `dedup_by_url`, `select_frames`, `fov_to_cutout_size`, `cutout_url`.
- `backend/src/irsa/client.rs` — `IrsaClient` (cloneable, shared
  semaphore + cache), `search`, `fetch_cutout`, `fetch_frames`
  (`FrameSet`/`CutoutFrame`), `warm` (`WarmTarget`/`WarmReport`).
- `backend/tests/irsa_tests.rs` — 19 tests; synthetic VOTable +
  synthetic MEF (reuses `tests/fixtures`), local axum stub.

## What was built

Pipeline: `fetch_frames(ra, dec, fov)` → SIA2 cone search per
collection (`GET {base}/SIA?COLLECTION=<c>&POS=circle <ra> <dec>
<radius>`; `POS` value URL-encoded by reqwest, verified live) →
VOTable rows → `Observation`s, deduped across collections by
`access_url` (first wins) → `select_frames` caps at `MAX_FRAMES`,
evenly spaced over the epoch range, newest first → each cutout
fetched in parallel (`JoinSet`) under a `Semaphore`
(`IRSA_MAX_CONCURRENCY`, default 3) with retry on 503/429/timeouts/
transport errors (exponential backoff + jitter, `max_attempts` total,
15 s reqwest timeout) → bodies streamed via `Response::chunk()`,
rejected past 64 MiB, memory only → `crate::fits::parse_frame` →
cached.

Cache: single `moka::future::Cache`, weighted by `entry_weight` bytes,
capacity `MAX_CACHE_MB` MiB. Key = (sorted collection set, kind) where
kind is Search{ra,dec,radius} or Frame{ra,dec,size,obs_id} as f64/FNV
bits, matching the spec "(collection set, ra, dec, size, obs id)".
`Observation::id()` = FNV-1a-64 hex of `access_url` (stable id per the
contract, no new deps).

`fetch_frames` returns `FrameSet { frames, total_observations,
truncated }` (maps directly onto the contract's response fields).
Individual cutout failures are logged (tracing::warn) and skipped;
the call errors only when zero frames decode, surfacing the first
error. `warm(&[WarmTarget])` prefetches sequentially and returns a
`WarmReport`.

`From<IrsaError> for AppError` (inside `src/irsa/`, `error.rs`
untouched): `InvalidQuery`→400, `NoObservations` and `Http{404}`→404,
`Unavailable` and `Http{5xx}`→502, everything else→500.

## Verified live (2026-11-14, `SPHEREX_LIVE=1`)

- `POS=circle <ra> <dec> <radius>` (lowercase, form-encoded) → 200,
  107 rows for the b3 test position.
- SIA FIELD units confirmed: `em_min/em_max` `unit="m"` (converted to
  µm), `t_min/t_max` `unit="d"` (MJD). `energy_bandpassname` =
  "SPHEREx-D1" style; `access_url` = plain `/ibe/data/...fits` URL.
- Cutout `?center=337.87,-5.19&size=0.05deg` → 200, 29×29 cutout (deg
  suffix accepted; docs say degrees are the default anyway).
- **503 "Server busy, retry later" observed live** on single cutout
  requests — the third-party report is real; retries are mandatory.
- **422 "Arrays do not overlap"** returned when the cutout square
  misses the array even though SIA's `s_region` footprint covers the
  point (edge cases). Per-cutout skip in `fetch_frames` is required;
  a strict all-or-nothing fetch would fail common positions.
- Parent file sizes ~65 MB (content-length 65545920) — cutouts stay
  far below the 64 MiB cap.

## Decisions / deviations

- **No new dependencies**: VOTable parsed by a small hand-written
  scanner (no XML crate in Cargo.toml, which is out of scope); jitter
  derived from wall-clock nanos (no `rand`); FNV-1a for stable ids
  (no hash crate); body streaming via `Response::chunk()` (needs no
  `StreamExt` import).
- **Search radius** = FOV half-diagonal (`fov * sqrt(2)/2`): the
  smallest circle guaranteed to cover the square cutout.
- **"Newest first, spread over time"** implemented as: sort by epoch
  (mid-exposure MJD), keep `max_frames` evenly spaced indices when
  over the cap, return descending. Flagged as interpretation; b5 can
  re-sort for the API response.
- **Retryable set** = 503, 429, timeouts, and transport errors
  (connect/read). Other statuses are fatal immediately. 422 is NOT
  retried (deterministic per file+position).
- **`fetch_frames` skips failed cutouts** (warn-logged) and returns
  partial results; errors only when nothing decoded. Chosen because
  422-off-edge is common live (see above). b5 should surface
  `truncated`/`total_observations` as usual.
- **Single cache** for search results + decoded frames under one
  byte budget, instead of two separate caps.
- `em_min_um/em_max_um` may be NaN when the column is absent (rows
  without `access_url` are skipped outright).

## Open issues

- `fetch_frames` can take minutes over a congested IRSA (60 frames ×
  retries); b5 should consider a smaller default `MAX_FRAMES` or a
  total deadline for API requests. Warm path mitigates presets.
- No coalescing of concurrent identical fetches (get→miss→fetch→
  insert); a duplicate download is possible under a race. Harmless,
  bounded by the semaphore.
- DR1 edge rows may 422 more often than QR3 for off-edge positions —
  worth watching in integration.

## Verify

```bash
cd backend
cargo fmt --check          # ok
cargo clippy -- -D warnings # ok (also --all-targets)
cargo test                 # 13 fits + 19 irsa + 8 validation, all pass
SPHEREX_LIVE=1 cargo test --test irsa_tests live_irsa_smoke -- --nocapture
# live: 1 pass (2 cutouts decoded w/ detector, 1x422, 2x503-then-exhausted)
```
