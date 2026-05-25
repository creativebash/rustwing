#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

fail() {
  echo "version drift: $*" >&2
  exit 1
}

package_version() {
  awk '
    /^\[package\]/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^version = / {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' "$1"
}

framework_version="$(package_version "$ROOT/rustwing/Cargo.toml")"
[[ -n "$framework_version" ]] || fail "could not read rustwing package version"

cli_framework_version="$(sed -n 's/.*"\([0-9][^"]*\)",[[:space:]]*\/\/ FRAMEWORK_VERSION.*/\1/p' "$ROOT/cli/src/main.rs")"
[[ -n "$cli_framework_version" ]] || fail "could not read CLI FRAMEWORK_VERSION"

if [[ "$cli_framework_version" != "$framework_version" ]]; then
  fail "cli/src/main.rs FRAMEWORK_VERSION is $cli_framework_version, expected $framework_version"
fi

IFS=. read -r major minor _patch <<< "$framework_version"
framework_series="$major.$minor"

for manifest in "$ROOT/cli/template/api/Cargo.toml" "$ROOT/cli/template/worker/Cargo.toml"; do
  req="$(sed -n 's/^rustwing = "\(.*\)"/\1/p' "$manifest")"
  [[ -n "$req" ]] || fail "could not read rustwing dependency from ${manifest#$ROOT/}"
  if [[ "$req" != "$framework_series" && "$req" != "$framework_version" ]]; then
    fail "${manifest#$ROOT/} pins rustwing = $req, expected $framework_series or $framework_version"
  fi
done

echo "Version drift check passed: rustwing $framework_version"
