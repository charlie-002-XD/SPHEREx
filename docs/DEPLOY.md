# Deploying SPHEREx Blink

The whole app ships as a single container image: the Rust binary serves the
REST API and the built frontend from `STATIC_DIR`. The image is
hosting-neutral — it works on Render, Fly.io, Railway, Azure Container Apps,
or a plain VPS with Docker.

- Build context: repository root.
- Runtime base: `debian:bookworm-slim` + `ca-certificates` + `curl`
  (HEALTHCHECK only), non-root user `app` (uid 999).
- Measured image size: **139.5 MiB** (146,257,088 bytes,
  `docker image inspect --format '{{.Size}}'`, 2026-11-15).
- Measured build time: **36 s** clean (`docker build --no-cache`),
  **3 s** fully cached, on a 16-thread i7-13620H with the base images
  already pulled (2026-11-15).
- Exposed port: `8080` (override with `PORT`). Health endpoint:
  `GET /api/v1/health`.

## Build and run locally

```sh
docker build -t spherex-blink .
docker run --rm -p 8080:8080 -e WARM_TARGETS=0 spherex-blink
# open http://localhost:8080
```

`WARM_TARGETS=0` only skips the startup cache warm-up, which would fetch
the five presets from IRSA; drop it in production if you want the warm-up.

Or with Compose (copy `scripts/container.env.example` to `.env` first to
change defaults — Compose interpolates `.env` automatically):

```sh
cp scripts/container.env.example .env   # optional, sane defaults work as-is
docker compose up --build
```

`scripts/docker-smoke.sh` builds the image if it is missing, starts a
container on a random localhost port and verifies the whole serving surface:
API JSON routes, the five presets, SPA serving and fallback, gzip on
`/assets`, `/?mock=1`, security headers, JSON error bodies, non-root user,
TLS to IRSA, the Docker healthcheck and graceful SIGTERM shutdown. It exits
non-zero on the first failed check and always removes the container.

## Environment variables

All settings are plain configuration; the app needs **no secrets**. Every
variable below is read by `backend/src/config.rs` or
`backend/src/irsa/config.rs`; the table is kept in sync with those files.

| Variable | Default | Meaning |
|---|---|---|
| `PORT` | `8080` | TCP port the server listens on. |
| `STATIC_DIR` | `frontend/dist` | Directory with the built frontend (SPA fallback to `index.html`). The image sets `/app/static`. |
| `RUST_LOG` | `info` | `tracing-subscriber` env filter. |
| `ALLOWED_ORIGINS` | empty | Comma-separated CORS origins. Empty is strictest: no cross-origin requests. The UI is served same-origin, so CORS is not needed unless the API is called from another site. |
| `RATE_LIMIT_RPS` | `4` | Per-client API GET rate limit, requests/second (also the burst size). |
| `OUTPUT_NPIX` | `192` | Side of the square output raster for `/api/v1/frames` (contract max 256). |
| `REQUEST_DEADLINE_SECS` | `40` | Total upstream-fetch budget per API request; on expiry the API returns partial frames with `truncated=true`. |
| `WARM_TARGETS` | unset | Set to `1` (or `true`/`yes`/`on`) to pre-fetch the five preset targets through the cache at startup. Boolean flag, not a target list. |
| `IRSA_BASE_URL` | `https://irsa.ipac.caltech.edu` | NASA/IPAC IRSA service root. |
| `IRSA_COLLECTIONS` | `spherex_dr1,spherex_qr3` | Comma-separated collections, in query order. |
| `IRSA_MAX_CONCURRENCY` | `3` | Max parallel outbound IRSA requests. Keep ≤ 3; IRSA is a shared public service. |
| `IRSA_TIMEOUT_SECS` | `15` | Timeout for a single IRSA request, seconds. |
| `IRSA_MAX_ATTEMPTS` | `3` | Attempts per IRSA request, with exponential backoff on 503/429. |
| `IRSA_RETRY_BASE_MS` | `400` | Base backoff between IRSA retries, milliseconds. |
| `IRSA_MAX_BODY_MB` | `64` | Maximum accepted IRSA response body, MiB. |
| `MAX_FRAMES` | `60` | Upper bound on frames per `/api/v1/frames` (contract max 60). |
| `MAX_CACHE_MB` | `200` | In-memory cutout cache cap, MiB. |

`PORT`, `STATIC_DIR`, `RUST_LOG`, `MAX_CACHE_MB` and `IRSA_MAX_CONCURRENCY`
have defaults baked into the image; set the rest per environment as needed.

## Quick start on a VPS

The build, `docker run` and smoke-verification commands below were executed
against the real image on 2026-11-15. Any Debian/Ubuntu VPS works.

```sh
# 1. Install Docker and Caddy.
curl -fsSL https://get.docker.com | sudo sh
sudo apt-get install -y caddy

# 2. Clone and build the image (~36 s clean build).
git clone <your-fork-url> spherex-blink && cd spherex-blink
docker build -t spherex-blink .

# 3. Run on localhost only, with a restart policy; Caddy exposes it.
docker run -d --name spherex-blink --restart unless-stopped \
  -p 127.0.0.1:8080:8080 spherex-blink
```

`/etc/caddy/Caddyfile` — replace the domain; Caddy obtains and renews the
TLS certificate automatically:

```caddy
blink.example.com {
	reverse_proxy 127.0.0.1:8080
}
```

```sh
sudo systemctl reload caddy
```

Firewall: only 22/80/443 need to be open; the app port is bound to
localhost so it is never exposed directly.

```sh
sudo ufw allow 22 && sudo ufw allow 80 && sudo ufw allow 443 && sudo ufw enable
```

Verify at any time: `scripts/docker-smoke.sh` runs the full check list
above against the local image. To check the live deployment, browse to
`https://blink.example.com/api/v1/health` — it must return
`{"status":"ok", ...}`.

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

### Railway

Railway auto-detects the Dockerfile on "Deploy from repo". For a plain VPS
with Docker, see "Quick start on a VPS" above.

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
- `docker`: `docker build` plus `scripts/docker-smoke.sh` — builds the
  image and runs the full container check list (API, SPA, gzip, headers,
  non-root, healthcheck, graceful stop). No push, no registry credentials.
