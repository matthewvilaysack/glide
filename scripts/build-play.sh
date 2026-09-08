#!/usr/bin/env bash
# Build glide-play for the browser: wasm plus the JS glue wasm-bindgen writes.
# Output lands in dist/play/ (git-ignored) with a VERSION sidecar so the site
# repo can say which commit its copy came from.
# Usage: scripts/build-play.sh [out-dir]
set -euo pipefail
out_arg="${1:-}"
cd "$(dirname "$0")/.."
# Resolve a caller-supplied out-dir against the caller's cwd, not the repo root,
# so a relative path is never re-based under the repo before the rm -rf below.
if [ -n "$out_arg" ]; then
  out="$(cd "$OLDPWD" && mkdir -p "$out_arg" && cd "$out_arg" && pwd)"
else
  out="dist/play"
fi
budget=409600 # 400 KB, the spec's page-weight ceiling for the wasm blob

(cd cli && cargo build -p glide-play --release --locked --target wasm32-unknown-unknown)
rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --typescript --out-dir "$out" \
  cli/target/wasm32-unknown-unknown/release/glide_play.wasm

size=$(wc -c < "$out/glide_play_bg.wasm" | tr -d ' ')
if [ "$size" -gt "$budget" ]; then
  echo "glide_play_bg.wasm is $size bytes, over the $budget byte budget" >&2
  exit 1
fi

version=$(grep -m1 '^version' cli/Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
printf 'glide-play %s\ncommit %s\nwasm %s bytes\n' "$version" "$(git rev-parse --short HEAD)" "$size" > "$out/VERSION"
printf 'sha256 %s\n' "$(shasum -a 256 "$out/glide_play_bg.wasm" | cut -d' ' -f1)" >> "$out/VERSION"
cat "$out/VERSION"
