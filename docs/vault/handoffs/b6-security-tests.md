# Handoff: b6-security-tests

Status: **done**. All gates green on branch `devin/b6-security-tests`.
5 tests are `#[ignore]`d for real bugs found by this battery (listed below, each with a suggested fix). Everything else passes.

## Files
- `backend/tests/support/mod.rs` — shared test infrastructure: seeded `Rng` (xorshift64*, deterministic, no deps), percent-encode/decode helpers, synthetic `Observation`/MEF-cutout builders, `StubSource` (a `FrameSource` stub with per-call sleep, active-call counter, injected `IrsaError`), `state()`/`app()` router builders, oneshot `call`/`call_guarded` helpers, `check_error_body` contract validator, raw-TCP `spawn_server`/`raw_exchange` for requests that cannot be expressed as `http::Uri`, a `LogSink` `tracing` writer, `TempDir`, and `/proc/self/status` RSS reader.
- `backend/tests/security_tests.rs` — 28 tests (25 pass, 3 ignored bugs): headers/CSP/CORS, method handling, SPA-vs-API fallback, static traversal, oversized request line/headers over real TCP, rate limiting incl. per-client-key isolation, deadline/cancellation, resource bounds, error-body and log leak checks.
- `backend/tests/fuzz_tests.rs` — 12 tests (10 pass, 2 ignored bugs): deterministic seeded fuzzing of `/frames` and `/spectrum` (~5,950 executed cases), curated boundary battery, long-value battery, property tests of `parse_coordinates`/`validate_query_params`, panic-safety on arbitrary bytes.

No `src/` edits, no new dependencies (tokio `net`/`io-util` were already unified; the RNG is hand-rolled).

## What was built

### Fuzzing (`fuzz_tests.rs`)
- `fuzz_frames_query_strings` / `fuzz_spectrum_query_strings`: seeded generator produces valid, boundary, unicode, very-long (up to 60 KiB encoded — `http::Uri` caps request targets at `u16::MAX`), duplicated-parameter, percent-encoded NUL/control, SQL/command/path-traversal/HTML payloads, NaN/inf spellings and sexagesimal edge cases. Invariants per case: never 5xx, never a panic (`call_guarded` uses `catch_unwind`), every 4xx body is a valid `ErrorBody` with a contract code and never `internal`, raw inputs are never echoed, per-case latency < 200 ms. Stats print case counts, slowest case and status distribution.
- `curated_boundary_cases`: `ra=360`, `ra=-0`, `dec=90.0000001`, `fov=0.0099`, `max_frames=0/61/-1/99999999999`, `detector` out of 1–6, `mode` garbage, plus injection strings — each pinned to an expected status.
- `very_long_query_values_are_handled`: 8–60 KiB values; documents that >64 KiB targets cannot be represented as `http::Uri` at all (they are exercised over raw TCP in security_tests).
- Property tests: `roundtrip_decimal_sexagesimal` (worst error ra 2.1e-8 deg, dec 1.4e-9 deg, under the 1e-6 bound), `out_of_range_values_always_rejected`, `parser_never_panics_on_arbitrary_bytes`, `parser_is_deterministic`, `sexagesimal_mutants_never_panic`, `health_unaffected_by_adjacent_bad_queries`.

### HTTP/security (`security_tests.rs`)
- CSP `frame-ancestors *` present, no `X-Frame-Options`, on API and SPA responses.
- CORS: allow-listed origin echoed; disallowed origin gets no `Access-Control-Allow-Origin` (simple and preflight); preflight advertises GET only; no wildcard with credentials.
- Methods: POST/PUT/DELETE/PATCH on every API route → 405 with `allow: GET,HEAD`; bare OPTIONS → 200 auto-answer (axum `MethodRouter`) with GET-only `allow`/`access-control-allow-methods`; HEAD served.
- Routing: unknown `/api/v1` paths → JSON 404 `not_found`; unknown non-API paths → `index.html`; static traversal (`..%2f`, `%00`, double-encoded, `/..` on a real static dir with a canary file outside it) never escapes `STATIC_DIR`.
- Raw-TCP layer: 1 MiB request line → `431 Request Header Fields Too Large`; oversized header block → 431; malformed bytes → clean 4xx/close, never a hang or 5xx.
- Rate limiting: burst beyond `rate_limit_rps` → 429 with contract `ErrorBody`; a different `X-Forwarded-For` key is not penalised (per-key buckets); a garbage XFF falls back to peer IP and still gets limited.
- Deadline: slow stub beyond a tiny `REQUEST_DEADLINE_SECS` returns partial frames with `truncated=true` (or 502 when nothing decoded), never hangs, and the stub's active-call counter drops back to 0 — in-flight work is cancelled.
- Resource bounds: `max_frames` cap respected and `truncated` flag set; largest allowed response (default `OUTPUT_NPIX`) measured **11.27 MiB (11,817,721 bytes)** decoded < 20 MB; RSS over 200 sequential requests did not grow (42.7 MB → 26.8 MB, delta 0).
- Leaks: error bodies never contain injected markers, upstream host/path, `error sending request for url`, `.rs`, `panicked`, or `RUST_BACKTRACE`; captured `tracing` output contains no env-var marker or panic internals.

## Decisions / deviations
- **`http::Uri` bound**: request targets >64 KiB cannot be built with `http::Uri`, so the "1 MB string" requirement is covered in two layers — up to ~60 KiB encoded through the router (fuzz + `very_long_query_values_are_handled`) and a 1 MiB raw request line over a real TCP socket (`oversized_request_line_is_handled`, answered with hyper's 431).
- **Fuzz 4xx codes**: the battery accepts any contract code but asserts `internal` never leaks. Valid-but-uncovered coordinates legitimately return 404 `not_found` (e.g. `ra=337.8783221&dec=0.5` against a stubbed search), and the `detector` empty-group bug also produces 404 — both are distinguished from genuine contract violations by the pinned ignored tests below.
- **5xx allowed in `error_bodies` test**: `upstream_unavailable` (502) is itself a contract code, so the leak check asserts a valid `ErrorBody` and `code != "internal"` rather than 4xx-only.
- **Log-marker isolation**: each leak test uses a unique marker (`B6-BODY-LEAK-MARKER`, `B6-SECRET-PATH-MARKER`, `B6-SECRET-ENV-MARKER`) so parallel tests cannot contaminate each other's log assertions.
- **Client-key rate-limit test**: `SmartIpKeyExtractor` honours `X-Forwarded-For`, so per-key isolation is tested by sending a different XFF (documented in the test).
- `SI_USAGE.md` unchanged: the existing Devin row already covers "tests".

## Bugs found (tests kept, `#[ignore]`d — gate stays green)

| Test | Input | Observed | Expected | Suggested fix |
|---|---|---|---|---|
| `spectrum_range_checks_coordinates` | `GET /api/v1/spectrum?ra=400&dec=0` (also `ra=-1`, `dec=±91`, `dec=inf`) | 404 `not_found` "no observations at this position" | 400 `invalid_parameter` | `backend/src/api/params.rs::spectrum_params` parses but never calls `validate_query_params`; call it on the parsed values like `frames` does. |
| `detector_filtered_empty_group_is_200_not_404` | `GET /api/v1/frames?ra=337.878&dec=-5.189&detector=1&mode=diff` with only detector-3 cutouts | 404 `not_found` (raw mode returns 200 + `frames: []`) | 200 + empty `frames` | `backend/src/api/frames.rs`: when the `detector` filter empties the set, return an empty frame list before `compose`, or map `ScienceError::EmptyInput` to an empty 200 result instead of `NotFound` in `group_frames`/`stack_detectors` callers. |
| `other_api_prefixes_also_return_json_404` | `GET /api/v2/frames?ra=1&dec=0`, `/api/`, `/api/v9`, `/api/v1/` | 200 `index.html` (SPA fallback; empty 404 when no `index.html` exists) | 404 JSON `ErrorBody` | `backend/src/api/mod.rs`: `nest("/api/v1", api)` only matches `/api/v1` and `/api/v1/<non-empty>`; everything else falls to `fallback_service(spa)`. Add a route/middleware that returns the API `NotFound` for any `/api/…` path before the SPA fallback (and cover the `/api/v1/` trailing-slash gap). |
| `logs_do_not_leak_upstream_url_paths` | fetch error whose `IrsaError::Unavailable.detail` embeds `http://127.0.0.1:PORT/B6_SECRET_PATH_MARKER/x.fits` | captured `tracing` output contains the marker (full URL incl. path) via `warn!("cutout skipped: {e}")` | logs may show host/error kind, never the full upstream URL path | `backend/src/api/fetch.rs`: log a sanitised form — e.g. `warn!("cutout skipped: {}", e.kind())`/status only — or strip the URL from `IrsaError` detail when logging. |
| `largest_payload_at_max_npix_also_under_20mb` | `max_frames=60`, `OUTPUT_NPIX=256` (the documented maximum) | 20,991,841 bytes ≈ 20.0 MB — exceeds the documented 20 MiB bound | < 20 MiB | `backend/src/api/frames.rs`/config: lower `OUTPUT_NPIX` max, cap `max_frames` for large npix, or document a higher bound. At the default npix the payload is 11.27 MiB. |

Note: `/api/v1/` (trailing slash) escaping to the SPA is a routing edge of the same `nest` bug — `matchit` does not match an empty `*rest`.

## Open issues
- None blocking. Follow-up owners should un-ignore the five tests once the fixes land in `src/` (out of this task's scope; another agent edits `src/` concurrently).

## Verify
Run in `backend/`:
```
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```
Results (2026-11-14, this branch): fmt clean; clippy `-D warnings --all-targets` clean; `cargo test` → **110 passed, 0 failed, 5 ignored** across unit + integration suites (security_tests 25/3, fuzz_tests 10/2, api_tests 20, irsa_tests 13, fixtures 13, validation_tests 19, science 15, lib unit 8).

## Test / protection / status / measurements

| Test | Protects | Status | Measured |
|---|---|---|---|
| `fuzz_frames_query_strings` | /frames input robustness, no 5xx/panic/echo, contract ErrorBody, <200 ms/case | pass | 3,572 cases (28 unbuildable URIs), slowest 3.5 ms, max body 7,079 B, statuses {200:67, 400:3504, 404:1} |
| `fuzz_spectrum_query_strings` | same for /spectrum | pass | 2,380 cases (20 unbuildable), slowest 2.7 ms, max body 114 B, statuses {400:2281, 404:99} |
| `curated_boundary_cases` | boundary values (ra=360, dec=90.0000001, fov=0.0099, max_frames 0/61/-1/99999999999, injections) | pass | ~40 pinned cases, all expected statuses |
| `very_long_query_values_are_handled` | long values (up to 60 KiB encoded) | pass | slowest 3.6 ms; >64 KiB proven unrepresentable as `http::Uri` |
| `roundtrip_decimal_sexagesimal` | coordinate grammar correctness | pass | worst error: ra 2.1e-8°, dec 1.4e-9° (bound 1e-6°), 4,000 pairs |
| `out_of_range_values_always_rejected` | every OOR value rejected | pass | 2,000 generated + edge sweeps |
| `parser_never_panics_on_arbitrary_bytes` | panic safety on raw bytes | pass | 500 random byte strings, 0 panics |
| `parser_is_deterministic` | same input → same verdict | pass | 200 pairs re-parsed |
| `sexagesimal_mutants_never_panic` | sexagesimal edge grammar | pass | 2,000 mutants |
| `health_unaffected_by_adjacent_bad_queries` | state poisoning | pass | interleaved bad queries, health stays 200 |
| `csp_frame_ancestors_and_no_xfo` | clickjacking headers | pass | `frame-ancestors *` present, XFO absent |
| `non_get_methods_on_api_get_405` | method allowlist | pass | POST/PUT/DELETE/PATCH ×4 routes → 405 + `allow: GET,HEAD` |
| `options_gets_informative_200_with_get_only_allow` | OPTIONS behaviour | pass | 200, `allow: GET,HEAD`, ACAM GET-only |
| `head_is_served_like_get` | HEAD semantics | pass | 200, empty body |
| `unknown_api_paths_return_json_404` | API fallback contract | pass | 5 paths → 404 JSON `not_found` |
| `other_api_prefixes_also_return_json_404` | API fallback for /api/* | **ignored — BUG** | `/api/v2/frames…` → 200 index.html |
| `spa_fallback_serves_index_html` | SPA routing | pass | non-API paths → index.html |
| `static_path_traversal_never_leaks_files` | static dir escape | pass | 8 traversal payloads, canary file never served |
| `oversized_request_line_is_handled` | request-line DoS | pass | 1 MiB line → 431 |
| `oversized_headers_are_handled` | header DoS | pass | large header block → 431 |
| `malformed_request_bytes_are_handled` | parser robustness | pass | garbage bytes → 4xx/close, no hang |
| `cors_allowed_origin_is_echoed` | CORS allowlist | pass | ACAO = origin |
| `cors_disallowed_origin_gets_no_acao` | CORS deny | pass | no ACAO header |
| `cors_preflight_allows_get_only` | preflight surface | pass | ACAM = GET only |
| `cors_preflight_from_disallowed_origin_not_echoed` | preflight deny | pass | no AC* headers |
| `cors_never_wildcard_and_no_credentials` | CORS+credentials | pass | no `*` ACAO, no ACAC |
| `rate_limit_burst_returns_429_contract_body` | rate limiting | pass | 429 + contract `rate_limited` body |
| `rate_limit_other_client_key_not_penalised` | per-key buckets | pass | other XFF → 200 under same burst |
| `rate_limit_garbage_xff_falls_back_to_peer_ip` | XFF spoof safety | pass | garbage XFF still limited on peer IP |
| `deadline_partial_results_are_truncated` | deadline/partial | pass | 1 fast + slow stubs → `truncated=true`, no hang |
| `deadline_nothing_decoded_is_502_and_cancels` | deadline/empty + cancel | pass | 502, active stub calls → 0 |
| `max_frames_cap_and_truncation_flag` | max_frames cap | pass | request 999 → served ≤60, `truncated` set |
| `largest_default_payload_under_20mb` | payload bound | pass | 11.27 MiB (11,817,721 B) decoded |
| `largest_payload_at_max_npix_also_under_20mb` | bound at OUTPUT_NPIX=256 | **ignored — BUG** | 20,991,841 B ≈ 20.0 MB > 20 MiB |
| `rss_growth_bounded_over_200_requests` | memory growth | pass | RSS 42,655,744 → 26,783,744 B (Δ 0.00 MiB) |
| `error_bodies_never_echo_input_or_internals` | error-body leaks | pass | 5 hostile URIs, no marker/path/trace in bodies |
| `logs_do_not_leak_upstream_url_paths` | log leaks | **ignored — BUG** | marker URL path present in warn! output |
| `logs_do_not_leak_secrets_or_internals` | log leaks (env/panic) | pass | no env marker, `panicked at`, `RUST_BACKTRACE` |
| `spectrum_range_checks_coordinates` | spectrum input contract | **ignored — BUG** | `ra=400` → 404, expected 400 |
| `detector_filtered_empty_group_is_200_not_404` | frames consistency | **ignored — BUG** | `detector=1&mode=diff` → 404 vs 200-empty in raw |
