# Handoff: ops-docker-ci

Status: **done**. `docker build .` succeeds; CI YAML and compose file validate.

## Files

- `Dockerfile` — multi-stage: `node:22-bookworm-slim` builds `frontend`
  (`npm ci && npm run build`), `rust:1-bookworm` builds `backend` in release
  mode with a dummy-source dependency-cache layer, `debian:bookworm-slim`
  runtime with `ca-certificates` + `curl`, non-root user `app` (uid 999),
  `/app/spherex-backend` + `/app/static`, env defaults `PORT=8080`,
  `STATIC_DIR=/app/static`, `RUST_LOG=info`, `MAX_CACHE_MB=200`,
  `IRSA_MAX_CONCURRENCY=3`, `EXPOSE 8080`, curl `HEALTHCHECK` on
  `/api/v1/health`.
- `.dockerignore` — excludes .git, node_modules/dist/target, tests, docs,
  ops files, `.env`, logs.
- `docker-compose.yml` — one service, all env vars interpolated from `.env`
  with safe defaults, no secrets.
- `scripts/container.env.example` — documented env file template.
- `scripts/docker-smoke.sh` — build → run → wait for `/api/v1/health` →
  report size → cleanup.
- `.github/workflows/ci.yml` — jobs `backend` (fmt, clippy --all-targets
  -D warnings, test; rust-cache), `frontend` (npm ci, tsc, vitest, eslint,
  build; npm cache), `docker` (build only, no push). Actions pinned to major
  tags (checkout@v4, setup-node@v4, rust-cache@v2,
  dtolnay/rust-toolchain@stable).
- `docs/DEPLOY.md` — local build/run, env var table, per-host minimal config
  (Render, Fly.io, Azure Container Apps, Railway/VPS), FortiGuard
  classification step for judges, measured image size.

## Decisions / deviations

- Chose `debian:bookworm-slim` over distroless `cc` so the HEALTHCHECK can
  use curl; distroless has no shell/HTTP tool and the binary has no
  healthcheck subcommand.
- Full `rust:1-bookworm` (not slim) build stage: `ring` (via reqwest rustls)
  needs a C toolchain.
- **Real bug found and fixed:** BuildKit `COPY` preserves old context mtimes,
  so after the dummy-source layer `cargo build` finished in 0.13 s and would
  have shipped a stale dummy binary once b5 lands. Added
  `find src -type f -exec touch {} +` before the real build; verified the
  crate now recompiles (`Compiling spherex-backend`, 0.99 s).
- `docker-compose.yml` uses env interpolation from `.env` instead of
  `env_file:` so it works on older Compose versions (no `required:` key).
- Host-agnostic: no registry push, no cloud resources created.

## Open issues

- `backend/src/main.rs` is still `fn main() {}` (b5-api-routes not merged):
  the image builds and runs, but the process exits immediately and the
  healthcheck will fail until b5 wires the server. Re-run
  `scripts/docker-smoke.sh` after b5 merges.
- Env var defaults for `IRSA_BASE_URL`, `IRSA_COLLECTIONS`, `MAX_FRAMES`,
  `ALLOWED_ORIGINS`, `RATE_LIMIT_RPS`, `WARM_TARGETS` in compose/docs are
  sensible guesses aligned with the contract and facts.md; if b5 names or
  semantics differ, update `docker-compose.yml`, `scripts/container.env.example`
  and `docs/DEPLOY.md` at integration time.
- hadolint was not run locally (not installed; container run rejected).
  Manual review: pinned base tags, `--no-install-recommends`, apt lists
  cleaned, non-root, exec-form CMD — no known violations.

## Verify commands and results

- `docker build -t spherex-blink .` → **OK** (real crate recompiles after
  touch fix; frontend dist 239.6 kB js / 74 kB gz).
- `docker image inspect spherex-blink:local --format '{{.Size}}'` →
  **136,255,379 bytes (~130 MiB)** — recorded in `docs/DEPLOY.md`.
- `docker run --rm --entrypoint sh spherex-blink:local -c 'ldd …; id'` →
  links only against libc/libgcc; runs as `uid=999(app)`.
- `python3 -c 'import yaml,sys; yaml.safe_load(open(sys.argv[1]))' .github/workflows/ci.yml` → **valid YAML**.
- `docker compose config --quiet` → **OK**.
- `bash -n scripts/docker-smoke.sh` → **OK**.
- `docker run --rm spherex-blink:local` → exits 0 immediately (stub main;
  expected until b5).
