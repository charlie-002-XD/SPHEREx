#!/usr/bin/env bash
# Build (if needed) and smoke-test the all-in-one image against the real app.
# Verifies: health JSON, preset targets, SPA serving + fallback, gzip on
# /assets, security headers, JSON error bodies, non-root user, TLS to IRSA
# (CA certs), Docker healthcheck and graceful SIGTERM shutdown.
#
# Usage: scripts/docker-smoke.sh [image-tag]
# Exits non-zero on the first failed check; the container is always removed.
set -euo pipefail

TAG="${1:-spherex-blink:smoke}"
NAME="spherex-blink-smoke-$$"
TMP="$(mktemp -d)"
BASE=""

pass() { echo "ok: $*"; }
fail() {
    echo "FAIL: $*" >&2
    docker logs "$NAME" >&2 2>/dev/null || true
    exit 1
}
cleanup() {
    docker rm -f "$NAME" >/dev/null 2>&1 || true
    rm -rf "$TMP"
}
trap cleanup EXIT

# fetch <path> -> writes body to $TMP/body, echoes the HTTP status.
# Retries on 429: the default rate limit is 4 req/s per client IP and the
# container-side HEALTHCHECK shares the API, so bursts can be throttled.
fetch() {
    local path="$1" i code
    for i in 1 2 3 4 5; do
        code=$(curl -sS -o "$TMP/body" -w '%{http_code}' "$BASE$path")
        [ "$code" != "429" ] && { echo "$code"; return; }
        sleep 1.2
    done
    echo "$code"
}

expect() { # expect <actual> <wanted> <label>
    [ "$1" = "$2" ] || fail "$3: wanted $2, got $1"
}

if ! docker image inspect "$TAG" >/dev/null 2>&1; then
    echo "==> Image $TAG not found locally; building"
    docker build -t "$TAG" .
fi

echo "==> Starting $TAG on a random localhost port (WARM_TARGETS=0)"
docker run -d --name "$NAME" -p 127.0.0.1::8080 -e WARM_TARGETS=0 "$TAG" >/dev/null
PORT="$(docker port "$NAME" 8080/tcp | awk -F: '{print $NF}')"
BASE="http://127.0.0.1:$PORT"

echo "==> Waiting for /api/v1/health"
for _ in $(seq 1 30); do
    curl -fsS "$BASE/api/v1/health" >"$TMP/body" 2>/dev/null && break
    sleep 1
done
[ -s "$TMP/body" ] || fail "health endpoint did not answer in 30s"
grep -q '"status":"ok"' "$TMP/body" || fail "health body: $(cat "$TMP/body")"
pass "GET /api/v1/health -> 200 JSON status ok"

code=$(fetch /api/v1/targets)
expect "$code" 200 "GET /api/v1/targets"
n=$(grep -o '"id"' "$TMP/body" | wc -l)
expect "$n" 5 "preset target count"
pass "GET /api/v1/targets -> 200, five presets"

code=$(fetch /)
expect "$code" 200 "GET /"
grep -q 'id="root"' "$TMP/body" || fail "/ did not return the SPA index.html"
cp "$TMP/body" "$TMP/index.html"
pass "GET / -> SPA index.html"

code=$(fetch '/?mock=1')
expect "$code" 200 "GET /?mock=1"
grep -q 'id="root"' "$TMP/body" || fail "/?mock=1 did not return HTML"
pass "GET /?mock=1 -> SPA index.html (mock mode loads client-side)"

asset=$(grep -oE '/assets/[^"]+\.js' "$TMP/index.html" | head -1)
[ -n "$asset" ] || fail "no /assets/*.js reference found in index.html"
# HEAD responses are never encoded by tower-http; the check needs a real GET.
curl -sS -o /dev/null -D "$TMP/h" -H 'Accept-Encoding: gzip' "$BASE$asset"
grep -qi '^content-encoding: gzip' "$TMP/h" \
    || fail "$asset not gzip-encoded: $(grep -i '^content-' "$TMP/h")"
pass "GET $asset -> content-encoding: gzip"

curl -sS -o /dev/null -D "$TMP/h" "$BASE/"
grep -qi '^content-security-policy: *frame-ancestors \*' "$TMP/h" \
    || fail "missing CSP frame-ancestors * on /"
grep -qi '^x-frame-options:' "$TMP/h" \
    && fail "x-frame-options must not be set (embedding uses CSP)"
pass "security headers: CSP frame-ancestors *, no X-Frame-Options"

code=$(fetch /definitely-not-a-ui-route)
expect "$code" 200 "GET unknown path"
grep -q 'id="root"' "$TMP/body" || fail "unknown path did not fall back to SPA"
pass "GET unknown path -> SPA fallback"

code=$(fetch /api/v1/no-such-route)
expect "$code" 404 "GET /api/v1/no-such-route"
grep -q '"error":{"code":"not_found"' "$TMP/body" \
    || fail "API 404 body is not the contract JSON: $(cat "$TMP/body")"
pass "GET /api/v1/no-such-route -> 404 JSON error"

code=$(fetch '/api/v1/frames?ra=400&dec=0')
expect "$code" 400 "GET /api/v1/frames?ra=400"
grep -q '"code":"invalid_parameter"' "$TMP/body" \
    || fail "frames 400 body is not the contract JSON: $(cat "$TMP/body")"
pass "GET /api/v1/frames?ra=400&dec=0 -> 400 JSON invalid_parameter"

uid=$(docker exec "$NAME" id -u)
[ "$uid" != "0" ] || fail "container runs as root (uid 0)"
pass "container runs as non-root (uid $uid)"

echo "==> Waiting for the Docker healthcheck"
for _ in $(seq 1 45); do
    status=$(docker inspect "$NAME" --format '{{.State.Health.Status}}')
    [ "$status" = "healthy" ] && break
    sleep 2
done
expect "$status" healthy "docker healthcheck"
pass "docker healthcheck -> healthy"

# TLS to IRSA proves ca-certificates are installed; any HTTP status counts.
irsa_code=$(docker exec "$NAME" curl -sI -o /dev/null -w '%{http_code}' \
    https://irsa.ipac.caltech.edu/) \
    || fail "TLS handshake to irsa.ipac.caltech.edu failed inside the container"
[ -n "$irsa_code" ] || fail "empty HTTP status from IRSA HEAD request"
pass "TLS to https://irsa.ipac.caltech.edu/ works inside container (HTTP $irsa_code)"

echo "==> Testing graceful shutdown (SIGTERM, 10s budget)"
start=$(date +%s%N)
docker stop "$NAME" >/dev/null
elapsed_ms=$(( ($(date +%s%N) - start) / 1000000 ))
exit_code=$(docker inspect "$NAME" --format '{{.State.ExitCode}}')
expect "$exit_code" 0 "container exit code after docker stop"
[ "$elapsed_ms" -lt 10000 ] || fail "docker stop took ${elapsed_ms}ms (>= 10s)"
pass "docker stop -> exit 0 in ${elapsed_ms}ms (graceful SIGTERM)"

size=$(docker image inspect "$TAG" --format '{{.Size}}')
echo "==> All checks passed. Image size: $((size / 1024 / 1024)) MiB ($size bytes)"
