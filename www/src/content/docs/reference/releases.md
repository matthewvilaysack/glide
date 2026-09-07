---
title: Releases
description: How a version ships, and how a shipped line gets patched.
---

Every version is a tag.
The tag builds four binaries with `--locked`, smoke-runs each, packages them with checksums, and publishes a GitHub Release with generated notes.
A Homebrew formula is pushed to the tap in the same run.
A red build publishes nothing.

Browse them at [tryglide.net/releases](https://tryglide.net/releases).

## Versioning

`MAJOR.MINOR.PATCH`.
Minor versions come from `main`.
Patch versions come from a release line.

## Patch releases

Once `main` moves on, a fix for what people already run goes through a lazy `release/X.Y` branch: cut from the line's latest tag only when a patch is needed, fed by cherry-picks from `main`, and every merge to it publishes the next `X.Y.Z`.
A patch never carries features.

## Verifying a download

`checksums.txt` on every release lists one SHA-256 per archive.

```sh
shasum -a 256 -c checksums.txt --ignore-missing
```

The one-line installer does this for you and refuses on a mismatch.

## For contributors

The runbook is [`docs/releasing.md`](https://github.com/matthewvilaysack/glide/blob/main/docs/releasing.md) in the repo: `scripts/release.sh X.Y.Z` to cut a release, `scripts/patch.sh <sha>` to open a patch PR.
