# Handoff: ops-verify-image

Status: **done**. The container works end to end with the real app
(b5 backend + f-series frontend). Every required check passed against the
running image; no source fixes under `backend/` or `frontend/` were needed
and none were made.

## Files

- `scripts/docker-smoke.sh` — rewritten: builds the image if absent, runs it
  on a random localhost port with `WARM_TARGETS=0`, and asserts the full
  serving surface (see check list below). Exits non-zero on the first failed
  check; always removes the container. Retries on 429 because the default
  rate limit is 4 GET/s per client IP.
- `.github/workflows/ci.yml` — `docker` job now runs
  `scripts/docker-smoke.sh spherex-blink:ci` after `docker build`.
- `docker-compose.yml` — environment block corrected to match
  `backend/src/config.rs` + `backend/src/irsa/config.rs`: added
  `IRSA_TIMEOUT_SECS`, `IRSA_MAX_ATTEMPTS`, `IRSA_RETRY_BASE_MS`,
  `IRSA_MAX_BODY_MB`, `OUTPUT_NPIX`, `REQUEST_DEADLINE_SECS`; fixed
  `RATE_LIMIT_RPS` default 20→4, `ALLOWED_ORIGINS` `*`→empty (strictest is
  the app default), `WARM_TARGETS` list→boolean flag (`0`).
- `scripts/container.env.example` — same corrections, with semantics
  comments; `MAX_FRAMES` comment fixed (contract max is 60, not 120).
- `docs/DEPLOY.md` — measured size/time updated, env table verified against
  config sources (17 variables, none missing/invented), new "Quick start on
  a VPS" (build → run with `--restart unless-stopped` → Caddyfile
  `reverse_proxy` with automatic HTTPS → ufw note), Railway section folded,
  smoke/CI descriptions updated.
- `Dockerfile`, `.dockerignore` — unchanged; both were already correct
  (`package-lock.json` exists, cache layers work, mtime-touch fix from
  ops-docker-ci makes the real crate rebuild).

## What was verified (actual outputs)

Build: `docker build -t spherex-blink:test .` → OK, 39 s (deps/npm/vite
rebuilt). `docker build --no-cache` → OK, **36 s**. Cached no-change
rebuild → **3 s** (i7-13620H, 16 threads, base images local).
Image size `docker image inspect --format '{{.Size}}'` → **146,257,088
bytes = 139.5 MiB**. `Config.User=app`, exposes 8080.

Runtime (`docker run -d -p 127.0.0.1:18080:8080 -e WARM_TARGETS=0`):

- `GET /api/v1/health` → 200 `application/json`
  `{"status":"ok","version":"0.1.0","collections":["spherex_dr1","spherex_qr3"]}`
- `GET /api/v1/targets` → 200, five presets (barnards-star, epsilon-indi-b,
  wise-0855, rw-aurigae, r-draconis)
- `GET /` → 200 `text/html`, index.html with `id="root"`
- `GET /?mock=1` → 200 `text/html` (mock mode is client-side)
- `GET /assets/index-CYngJwIp.js` with `Accept-Encoding: gzip` →
  `content-encoding: gzip`; 259,420 → 80,555 bytes wire size.
  Note: tower-http does not encode HEAD responses — the check must use GET.
- `GET /definitely-not-a-route` → 200 `text/html` (SPA fallback)
- `GET /api/v1/x` (no-such-route) → 404 `application/json`
  `{"error":{"code":"not_found",...}}`
- `GET /` headers → `content-security-policy: frame-ancestors *` present,
  no `x-frame-options`
- `GET /api/v1/frames?ra=400&dec=0` → 400 `application/json`
  `{"error":{"code":"invalid_parameter","message":"ra: out of range [0, 360) deg"}}`
- `docker exec … id -u` → 999 (`uid=999(app)`), non-root
- `docker inspect … .State.Health.Status` → `healthy`
- `docker exec … curl -sI https://irsa.ipac.caltech.edu/` → HTTP 302 in
  ~2 s: TLS handshake works (ca-certificates present). No IRSA API request
  was made; this HEAD only proves the CA path.
- `docker stop` → SIGTERM graceful shutdown: exit code 0 in ~117–137 ms
  (log: "shutdown requested"), well under the 10 s budget.

`scripts/docker-smoke.sh spherex-blink:test` → **all 13 checks `ok`,
exit 0**. Negative run against `debian:bookworm-slim` → exits 1 as
expected.

`docker compose config` → valid; `PORT=18081 docker compose up -d` started
the container (`health: starting` at first probe) and `down` removed it;
the serving path itself was verified via `docker run`, and `config`
confirms all env/ports render correctly.

## Decisions / deviations

- Smoke binds `127.0.0.1::8080` (docker-assigned free port) instead of a
  fixed port — no collisions in CI or dev machines.
- `docker run` in the smoke uses a named container without `--rm` so the
  exit code after `docker stop` can be inspected; the trap always removes it.
- `WARM_TARGETS` is a boolean flag in `config.rs` (`1`/`true`/`yes`/`on`),
  not a target list — compose/env-example/docs were wrong and were fixed.
- `RATE_LIMIT_RPS`/`ALLOWED_ORIGINS`/`STATIC_DIR`/`MAX_FRAMES`-comment and
  the six IRSA/limits knobs were missing or wrong in docs and compose —
  fixed to match `config.rs`/`irsa/config.rs` exactly.
- CI docker job still does `docker build` explicitly (visible step/layer
  timing), then smoke reuses the local image without rebuilding.

## Open issues

- None blocking. Known app-level findings remain owned by b7-fix-api-bugs
  (`/api/v2`, `/api/` fall through to the SPA instead of JSON 404 —
  `/api/v1/*` correctly 404s, which is what this task required).
- `SI_USAGE.md` not touched (outside allowed scope); its existing row
  "Dockerfile/CI, tests, documentation — Devin" already covers this task.

## Verify commands and results

- `bash -n scripts/docker-smoke.sh` → OK
- `scripts/docker-smoke.sh spherex-blink:test` → 13/13 checks pass, rc 0
- `docker image inspect` size → 146,257,088 B; `--no-cache` build 36 s;
  cached rebuild 3 s
- `docker compose config --quiet` → OK; `python3 yaml` on ci.yml → OK
- Cleanup: `spherex-blink:test|cold|cache-hit` and the
  `debian:bookworm-slim` pulled for the negative test all removed; no
  `spherex-blink*` containers remain (`docker ps -a` clean). Pre-existing
  `spherex-blink:local` from ops-docker-ci was left untouched.
