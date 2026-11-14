# Handoff: b5-api-routes

Status: **done**. All gates green on branch `devin/b5-api-routes`.

## Files

- `backend/src/config.rs` — single `Config` struct + `from_env`
  (`PORT` 8080, `IRSA_*`/`MAX_FRAMES`/`MAX_CACHE_MB` via `IrsaConfig`,
  `ALLOWED_ORIGINS` (empty = strictest), `RATE_LIMIT_RPS` 4,
  `STATIC_DIR` `frontend/dist`, `OUTPUT_NPIX` 192 (`<= 256`),
  `REQUEST_DEADLINE_SECS` 40, `WARM_TARGETS`) + `ConfigError`.
- `backend/src/error.rs` — added `From<FitsError>` (malformed upstream
  data -> 502, internals never exposed) and `From<ScienceError>`
  (aperture/detector/wavelength -> 400 with constraint messages,
  `EmptyInput` -> 404, `Wcs` -> 502, grid/shape invariants -> 500).
- `backend/src/api/mod.rs` — `FrameSource` trait (stubbable slice of
  `IrsaClient`: `search` + `fetch_cutout`), `AppState`,
  `build_router` (nest + governor + CORS + gzip + timeout + CSP +
  ServeDir SPA), `/health`, JSON 404 fallback.
- `backend/src/api/params.rs` — query decoding; every failure is 400
  `invalid_parameter`, raw input never echoed.
- `backend/src/api/fetch.rs` — `fetch_cutouts`: deadline-bounded
  parallel fetch (JoinSet + `sleep_until`), aborts in-flight work at
  expiry and returns partial results.
- `backend/src/api/frames.rs` — `/frames` pipeline and mode
  composition (`raw`/`diff`/`coadd`/`stack`).
- `backend/src/api/spectrum.rs` — `/spectrum` (FOV 0.01 deg,
  zodi-subtracted aperture mean, empty -> 404).
- `backend/src/api/targets.rs` — the five presets (same values as
  `frontend/src/lib/presets.ts`), also `warm_targets()` for main.
- `backend/src/api/dto.rs` — serde types matching the contract 1:1.
- `backend/src/main.rs` — bootstrap: tracing, config, `IrsaClient`,
  optional `warm()` spawn, `axum::serve` with
  `into_make_service_with_connect_info` + graceful shutdown
  (SIGINT/SIGTERM).
- `backend/Cargo.toml` — removed unused `fitrs` + `byteorder` (b3's
  pure-Rust reader made them dead); `tower` gained `util` for
  `oneshot` tests. No `flate2` added (tower-http does gzip).
- `backend/tests/api_tests.rs` — 20 oneshot tests, stub `FrameSource`,
  synthetic MEF via `tests/fixtures`.

## What was built

Routes exactly per `shared/openapi-contract.json`:
`GET /api/v1/{health,targets,frames,spectrum}`. Frames pipeline:
b1 `validate` -> `search` -> `select_frames(min(max_frames, cap))` ->
deadline-bounded `fetch_cutout` -> optional `detector` filter ->
`reproject` onto `OutputGrid::centered(ra, dec, fov, OUTPUT_NPIX)` with
`subtract_zodi` per query (contract default false) -> mode composition
-> float32-LE base64 `Frame` DTOs sorted by `obs_mjd_start` ascending.

Middleware: per-IP governor rate limit on the API (`SmartIpKeyExtractor`,
GET only; 429 -> JSON `ErrorBody`), strict CORS (`AllowOrigin::list`,
GET), gzip `CompressionLayer`, backstop `TimeoutLayer`
(deadline + 15 s), `CSP: frame-ancestors *` on every response;
`X-Frame-Options` is NOT set (embedding is a feature). Static:
`ServeDir(STATIC_DIR)` + `ServeFile(index.html)` SPA fallback.
`/api/v1/*` unknown paths -> JSON 404.

## Decisions / deviations

- **`fetch_frames` not used**: it returns only on completion, so the
  D-014 deadline ("return what is already decoded, `truncated=true`")
  cannot be implemented through it. The API composes the client's
  public `search` + `fetch_cutout` under a JoinSet instead — identical
  semaphore/retry/cache behaviour (no new parallelism).
- **`truncated`** = `total_observations > decoded_frames` (covers cap,
  deadline expiry and per-cutout skips such as upstream 422s).
- **`stack` mode emits only per-detector stacks**: the contract
  requires `detector` in 1..=6 and `stack_detectors`'s `combined`
  raster has `detector=None`; emitting it would violate the schema.
- **`coadd`** = one inverse-variance co-add per detector x wavelength
  group (`stack_images`); **`diff`** emits `members[1..]` minus
  `members[0]` per group (the reference epoch itself produces no
  frame). Members that fail the wavelength-tolerance check are skipped
  with a warning.
- **Derived frame ids**: FNV-1a over the mode byte + member access
  URLs (same scheme as `Observation::id`). `collection` for combined
  rasters joins unique member collections with `+`. `bandwidth_um`
  for multi-member rasters is the `em_min`/`em_max` span.
- **Spectrum** fetches on `FOV_MIN_DEG` (0.01 deg ~ 36" still covers a
  5 px aperture) and uses `subtract_zodi = true` (zodi would dominate
  the mean); contract has no such parameter there.
- **Rate limit** uses `SmartIpKeyExtractor` (deploy target is behind a
  reverse proxy); `RATE_LIMIT_RPS` = replenish period `1/rps` s, burst
  `rps`. `ALLOWED_ORIGINS` unset = deny all cross-origin (strictest);
  the SPA is same-origin so it is unaffected.
- `Config::from_env` uses `IrsaConfig::from_env` for `IRSA_*` +
  `MAX_FRAMES` + `MAX_CACHE_MB` (one `Config` struct, no duplicated
  parsing); `ConfigError` is a dedicated thiserror type.
- `health.version` = `env!("CARGO_PKG_VERSION")`. Non-finite f64s
  serialise as `null` (only when upstream metadata is absent).
- `X-Frame-Options`: absent by design; `CSP frame-ancestors *` is the
  embeddable policy.
- `TimeoutLayer` is only a backstop; the in-handler deadline returns
  partial results first.
- `unreachable!` (not `expect`) on the provably-impossible
  `GovernorConfigBuilder::finish()` `None` arm.

## Open issues

- `Observations` fetch is not deduplicated across in-flight identical
  requests (pre-existing note from b2 handoff).
- `diff` with single-member groups produces zero frames -> `200` with
  `frames: []` (contract-conformant; frontend should handle empty).
- Detector filter on `diff`/`coadd`/`stack` that empties the set maps
  to 404 via `EmptyInput`; raw mode yields `200 frames: []`.
- Governor keys per forwarded IP: behind an untrusted proxy clients
  could spoof `X-Forwarded-For` — acceptable for the hackathon
  deployment, documented in `mod.rs`.
- `frontend/dist` is not built in this worktree; `STATIC_DIR` was
  smoke-tested against a scratch dir (index 200, SPA fallback 200,
  `/api/v1/nope` -> JSON 404).

## Verify

```bash
cd backend
cargo fmt --check                     # ok
cargo clippy --all-targets -- -D warnings  # ok
cargo test                            # 75 pass: 20 api + 13 fits + 19 irsa + 15 science + 8 validation
```

Smoke test (binary): `health` 200, `targets` 5 presets, `ra=400` ->
400 `invalid_parameter`, CSP header `frame-ancestors *` present,
`X-Frame-Options` absent, SPA root/fallback 200, unknown API path ->
JSON 404, graceful SIGINT shutdown logged.

Tests present (all via tower `oneshot`, stub `FrameSource`, synthetic
MEF from `tests/fixtures`, no network): RA=400 / Dec=-95 / FOV=10 /
injection `1; DROP TABLE` -> 400 without echo; `mode`/`detector`/
`max_frames`/`subtract_zodi`/`aperture_px` violations -> 400; missing
`ra`/`dec` -> 400; 200 shapes for `health`, `targets` (5 presets,
decimal values asserted), `frames` raw/diff/coadd/stack (data_b64 =
`width*height*4` bytes, frames time-sorted), `spectrum`; 404 for empty
coverage; 502 for upstream unavailability and for deadline expiry with
nothing decoded; `truncated=true` when observations exceed
`max_frames`; detector filter; sexagesimal WISE 0855; rate limit ->
429 `rate_limited` JSON.
