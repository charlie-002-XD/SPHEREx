#!/usr/bin/env bash
# Build the image, run it, wait for the health endpoint, report, clean up.
# Usage: scripts/docker-smoke.sh [image-tag]
set -euo pipefail

TAG="${1:-spherex-blink:smoke}"
NAME="spherex-blink-smoke"
PORT="${PORT:-8080}"

cleanup() {
    docker rm -f "$NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "==> Building $TAG"
docker build -t "$TAG" .

echo "==> Image size"
docker image inspect "$TAG" --format '{{.Size}} bytes' | \
    awk '{ printf "%.1f MB\n", $1/1024/1024 }'

echo "==> Starting container on port $PORT"
docker run -d --name "$NAME" -p "$PORT:8080" "$TAG" >/dev/null

echo "==> Waiting for /api/v1/health"
for i in $(seq 1 30); do
    if curl -fsS "http://127.0.0.1:$PORT/api/v1/health" >/dev/null 2>&1; then
        echo "==> Healthy after ${i}s"
        curl -fsS "http://127.0.0.1:$PORT/api/v1/health"
        echo
        exit 0
    fi
    sleep 1
done

echo "!! Health endpoint did not answer in 30s" >&2
docker logs "$NAME" >&2 || true
exit 1
