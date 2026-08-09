#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-test}"
TMP=""

cleanup() {
  if [[ -n "$TMP" && -d "$TMP" ]]; then
    rm -rf "$TMP"
  fi
}
trap cleanup EXIT

ensure_cli() {
  CARGO_TARGET_DIR="$ROOT/target" cargo build --manifest-path "$ROOT/Cargo.toml" --bin rustwing > /dev/null
}

run_smoke() {
  echo "=== Smoke test: scaffold + generate resources + cargo check ==="
  ensure_cli
  export CARGO_TARGET_DIR="$ROOT/target/template-smoke"

  TMP="$(mktemp -d)"
  local app="$TMP/rustwing_template_smoke"

  "$ROOT/target/debug/rustwing" new "$app" --local "$ROOT" > /dev/null

  pushd "$app" > /dev/null
  "$ROOT/target/debug/rustwing" g resource post \
    --fields 'title:string:required:length(1,255)' \
    --fields 'body:string:optional' \
    --fields 'score:f64:required:range(0.0,100.0)' \
    --fields 'published_at:datetime:optional' > /dev/null
  "$ROOT/target/debug/rustwing" g resource ticket \
    --tenant org_id \
    --fields 'org_id:uuid:required' \
    --fields 'subject:string:required:length(1,255)' > /dev/null
  "$ROOT/target/debug/rustwing" g resource comment \
    --scope ticket_id \
    --fields 'ticket_id:uuid:required' \
    --fields 'body:string:required' > /dev/null
  "$ROOT/target/debug/rustwing" g resource note \
    --tenant org_id \
    --scope ticket_id \
    --fields 'org_id:uuid:required' \
    --fields 'ticket_id:uuid:required' \
    --fields 'body:string:required' > /dev/null
  cargo check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test
  popd > /dev/null

  echo "Template smoke test passed."
}

case "$MODE" in
  test|smoke)
    run_smoke
    ;;
  *)
    echo "Usage: $0 [test|smoke]" >&2
    exit 1
    ;;
esac
