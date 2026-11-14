#!/usr/bin/env bash
# Generates the third-party dependency list (name, version, license) for the
# NASA Space Apps submission field "Space Agency Partner Data and Other
# References". Sources: cargo metadata (Cargo.lock resolve, includes the
# declared license of every crate) and frontend/package-lock.json (v3 locks
# carry a "license" field per package).
#
# Usage: docs/submission/list-deps.sh  (run from anywhere; needs cargo + jq)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

echo "### Rust crates (from backend/Cargo.lock)"
cargo metadata --manifest-path "$ROOT/backend/Cargo.toml" --format-version 1 -q \
  | jq -r '.packages[] | select(.source != null) | "- \(.name) \(.version) (\(.license // "license unspecified"))"' \
  | LC_ALL=C sort -fu

echo
echo "### npm packages (from frontend/package-lock.json)"
jq -r '.packages
       | to_entries[]
       | select(.key != "")
       | .key as $k
       | .value
       | "\($k | split("node_modules/") | last) \(.version) (\(.license // "license unspecified"))"
       ' "$ROOT/frontend/package-lock.json" \
  | LC_ALL=C sort -fu \
  | sed 's/^/- /'
