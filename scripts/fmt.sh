#!/usr/bin/env bash
# Format (or --check) every crate. glide-llm's root is skipped on purpose: it
# declares `mod anthropic` behind a feature whose file does not exist yet, and
# rustfmt resolves that module before it looks at any flag.
set -euo pipefail
cd "$(dirname "$0")/../cli"
mode=("${@}")
for pkg in glide-common glide-graph glide-core glide-tui glide-vault glide-play glide-cli; do
  if grep -q "\"crates/$pkg\"" Cargo.toml; then
    cargo fmt -p "$pkg" -- "${mode[@]}"
  fi
done
rustfmt --edition 2021 "${mode[@]}" crates/glide-llm/src/provider.rs
