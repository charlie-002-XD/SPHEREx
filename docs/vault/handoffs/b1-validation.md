# Handoff: b1-validation

Status: **done**. All gates green on branch `devin/b1-validation`.

## Files
- `backend/src/error.rs` — `AppError` (thiserror) with `IntoResponse` for axum.
- `backend/src/api/validation.rs` — `parse_coordinates`, `validate_query_params`, `Query`, public range constants.
- `backend/tests/validation_tests.rs` — 8 integration tests, all synthetic inputs (no IRSA).

## What was built
- `AppError` variants cover all five `ErrorBody.error.code` values in `shared/openapi-contract.json`: `InvalidParameter` → 400, `NotFound` → 404, `RateLimited` → 429, `UpstreamUnavailable` → 502, `Internal` → 500. `IntoResponse` emits `{"error": {"code", "message"}}`. Helpers `status_code()`, `code()`, `invalid_parameter(msg)`.
- `parse_coordinates(ra, dec) -> Result<(f64, f64), AppError>` accepts decimal degrees (incl. `+` sign and `e`/`E` exponents) and sexagesimal (`08h55m10.831s`, `-7d14m42.53s`, `-07d14m42.53s`), case-insensitive units. RA `h` components scale x15; RA also accepts `d` (no scaling). Minutes/seconds must be < 60, components strictly ordered, no skipping (`10h30s` rejected).
- `validate_query_params(ra_deg, dec_deg, fov_deg) -> Result<Query, AppError>` rejects NaN/Inf and enforces `0 <= ra < 360`, `-90 <= dec <= 90`, `0.01 <= fov <= 0.5`. All violations → 400.
- Rejection of injection-like input is by grammar whitelist (hand-rolled parser, no new dependencies). Error messages describe the constraint only; a test asserts the response body never echoes `"1; DROP TABLE"`.
- No `.unwrap()`/`.expect()` outside `#[cfg(test)]`/tests; no `unsafe` (crate lints already forbid it via `Cargo.toml`).

## Decisions / deviations
- `Query` carries only `{ra_deg, dec_deg, fov_deg}` — the three arguments of the required signature. `mode`, `detector`, `max_frames`, `subtract_zodi` belong to b5 and were left out deliberately.
- `parse_coordinates` does NOT range-check (documented); ranges are enforced by `validate_query_params`. b5 should call both in sequence.
- Uppercase sexagesimal units accepted (e.g. `08H55M10S`) — cheap robustness, still inside a strict grammar.
- `Internal` carries no detail message; later tasks can extend `AppError` in place (variants for all contract codes already exist).
- `SI_USAGE.md` not edited: its existing row already covers "Rust backend modules — Devin".

## Open issues
- None blocking. Minor: `validate_query_params` takes `f64` for fov, so a missing `fov` must be defaulted (0.05) by the caller (b5), per contract.

## Verify
Run in `backend/`:
```
cargo fmt --check && cargo clippy -- -D warnings && cargo test
```
Results (2026-11-14): fmt clean; clippy `-D warnings` clean (also `--all-targets`); 8/8 tests pass, including WISE 0855 (`08h55m10.831s`, `-7d14m42.53s` → 133.795129, −7.245147, tol 1e-6), all five benchmark targets, RA=400 / Dec=−95 / FOV=10 / `"1; DROP TABLE"` / NaN / inf rejections.
