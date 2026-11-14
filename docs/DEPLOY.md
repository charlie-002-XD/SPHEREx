# Deploying SPHEREx Blink

The whole app ships as a single container image: the Rust binary serves the
REST API and the built frontend from `STATIC_DIR`. The image is
hosting-neutral — it works on Render, Fly.io, Railway, Azure Container Apps,
or a plain VPS with Docker.

- Build context: repository root.
- Runtime base: `debian:bookworm-slim` + `ca-certificates` + `curl`
  (HEALTHCHECK only), non-root user `app` (uid 999).
- Measured image size: **~130 MiB** (136,255,379 bytes,
  `docker image inspect --format '{{.Size}}'`, 2026-11-14).
- Exposed port: `8080` (override with `PORT`). Health endpoint:
  `GET /api/v1/health`.

## Build and run locally

```sh
docker build -t spherex-blink .
docker run --rm -p 8080:8080 spherex-blink
# open http://localhost:8080
```

Or with Compose (copy `scripts/container.env.example` to `.env` first to
change defaults — Compose interpolates `.env` automatically):

```sh
cp scripts/container.env.example .env   # optional, sane defaults work as-is
docker compose up --build
```

`scripts/docker-smoke.sh` builds the image, starts a container, waits for the
health endpoint and cleans up — a quick end-to-end check.

## Environment variables

All settings are plain configuration; the app needs **no secrets**.

| Variable | Default | Meaning |
|---|---|---|
| `PORT` | `8080` | TCP port the server listens on. |
| `STATIC_DIR` | `/app/static` | Directory with the built frontend. |
| `RUST_LOG` | `info` | `tracing-subscriber` env filter. |
| `IRSA_BASE_URL` | `https://irsa.ipac.caltech.edu` | NASA/IPAC IRSA service root. |
| `IRSA_COLLECTIONS` | `spherex_dr1,spherex_qr3` | Comma-separated collections, in query order. |
| `IRSA_MAX_CONCURRENCY` | `3` | Max parallel outbound IRSA requests. Keep ≤ 3; IRSA is a shared public service. |
| `MAX_FRAMES` | `60` | Upper bound on frames per `/api/v1/frames` (contract max 120). |
| `MAX_CACHE_MB` | `200` | In-memory cutout cache cap, MiB. |
| `ALLOWED_ORIGINS` | `*` | Comma-separated CORS origins (the UI is served same-origin). |
| `RATE_LIMIT_RPS` | `20` | Per-client API rate limit, requests/second. |
| `WARM_TARGETS` | empty | Comma-separated `ra,dec` targets pre-warmed into the cache at startup. |

`PORT`, `STATIC_DIR`, `RUST_LOG`, `MAX_CACHE_MB` and `IRSA_MAX_CONCURRENCY`
have defaults baked into the image; set the rest per environment as needed.

## Hosts

### Render

1. New → Web Service → point at the repo. Render detects the `Dockerfile`.
2. Health check path: `/api/v1/health`.
3. Render injects `PORT` — the app already reads it; leave the default.
4. Optional: set env vars from the table above in the service settings.

### Fly.io

```sh
fly launch --no-deploy   # detects the Dockerfile, generates fly.toml
fly deploy
```

Minimal `fly.toml` essentials if writing it by hand:

```toml
app = "spherex-blink"

[build]                # uses the repo Dockerfile

[env]
  PORT = "8080"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = "stop"
  auto_start_machines = true
  min_machines_running = 1

  [http_service.concurrency]
    type = "requests"
    soft_limit = 200
    hard_limit = 250
```

### Azure Container Apps

```sh
az containerapp up \
  --name spherex-blink \
  --resource-group <rg> \
  --environment <aca-env> \
  --source .                 # builds the Dockerfile in the cloud
```

Or with a prebuilt image: `az containerapp create --image <registry>/spherex-blink:tag`.
Set ingress to `external`, `targetPort 8080`, and the health probe to
`GET /api/v1/health`.

### Railway / a VPS

Railway auto-detects the Dockerfile on "Deploy from repo". On a VPS: build or
pull the image, then `docker run -d --restart unless-stopped -p 80:8080
spherex-blink` (behind any TLS-terminating reverse proxy).

## FortiGuard classification (required for judges)

Judges' corporate/school networks commonly run FortiGuard web filtering, and
freshly deployed hosting domains are frequently rated `Unrated`,
`Newly Registered Domain` or `Web Hosting` — categories many networks block.
The Space Apps submission guide therefore asks teams to classify the public
URL. Do this **as soon as the URL exists** (reviews take ~24 h):

1. Look up the deployed URL at https://www.fortiguard.com/webfilter
2. If it is unrated or in a blocked category, submit a reclassification at
   https://url.fortinet.net/rate/submit.php — suggest `Education` or
   `Information Technology`, note it is a NASA Space Apps hackathon project.
3. Re-check before submission and record the category in the project page.

## CI

`.github/workflows/ci.yml` runs on every push and PR:

- `backend`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test` (Swatinem/rust-cache).
- `frontend`: `npm ci`, `npx tsc --noEmit`, `npx vitest run`, `npx eslint .`,
  `npm run build` (npm cache on `frontend/package-lock.json`).
- `docker`: `docker build` — build-only, no push, no registry credentials.
