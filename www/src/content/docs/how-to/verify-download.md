---
title: Verify a download
description: Check an archive against the release's checksums file.
---

Every release publishes `checksums.txt` with one SHA-256 per archive.

```sh
curl -fsSLO https://github.com/matthewvilaysack/glide/releases/download/v0.3.0/checksums.txt
curl -fsSLO https://github.com/matthewvilaysack/glide/releases/download/v0.3.0/glide-0.3.0-aarch64-apple-darwin.tar.gz
shasum -a 256 -c checksums.txt --ignore-missing
```

```
glide-0.3.0-aarch64-apple-darwin.tar.gz: OK
```

The one-line installer does this for you and refuses on a mismatch.
Homebrew checks the sha256 in the formula.
Nothing is signed or notarized; the checksum is the guard, and the release page is the source of truth for it.
