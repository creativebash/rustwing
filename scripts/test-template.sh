#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

case "${1:-test}" in
  sync)
    echo "=== Sync root api/ from template (keeps extra files) ==="
    TMP=$(mktemp -d)
    "$ROOT/target/debug/rustwing" new --local "$ROOT" "$TMP/x" > /dev/null 2>&1
    rsync -a "$TMP/x/api/" "$ROOT/api/"
    cargo check -p api
    rm -rf "$TMP"
    echo "✅ Done"
    ;;
  test)
    echo "=== Smoke test: scaffold + cargo check (against local rustwing) ==="
    TMP=$(mktemp -d)
    "$ROOT/target/debug/rustwing" new --local "$ROOT" "$TMP/x" > /dev/null 2>&1
    cargo check --manifest-path "$TMP/x/Cargo.toml"
    rm -rf "$TMP"
    echo "✅ Done"
    ;;
  *)
    echo "Usage: $0 [test|sync]"
    exit 1
    ;;
esac
