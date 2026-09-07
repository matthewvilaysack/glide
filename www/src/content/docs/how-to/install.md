---
title: Install
description: Homebrew, the one-line installer, cargo, or from source.
---

Prebuilt for Apple Silicon, Intel Mac, and Linux x86_64 and arm64.
Every archive ships with a checksum that the installer and Homebrew verify.

## Homebrew

```sh
brew install matthewvilaysack/glide/glide
brew upgrade matthewvilaysack/glide/glide
```

The tap is updated by the release workflow the moment a version is published.

## One line

```sh
curl -fsSL https://tryglide.net/install | sh
```

Detects your OS and CPU, downloads the matching archive and `checksums.txt`, refuses on a mismatch, and installs to `~/.local/bin`.
Pin a version with `GLIDE_VERSION=0.3.0`, or choose a directory with `GLIDE_INSTALL_DIR`.
It never asks for sudo and writes nothing outside that directory.

## Cargo

```sh
cargo install --git https://github.com/matthewvilaysack/glide glide-cli
```

## From source

```sh
git clone https://github.com/matthewvilaysack/glide
cd glide/cli
cargo build --release -p glide-cli
./target/release/glide --version
```

## Every release

[tryglide.net/releases](https://tryglide.net/releases) lists every version with direct downloads and notes.
Nothing is signed or notarized: a curl or brew install is not quarantined by macOS, and the checksum is the guard.
Windows is not built yet.
