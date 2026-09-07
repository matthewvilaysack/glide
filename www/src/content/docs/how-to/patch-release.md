---
title: Patch a shipped release
description: "For contributors: get a fix onto a line people already run."
---

Once `main` has moved on, a fix for a shipped version goes through a lazy release branch.

1. Land the fix on `main` through a normal PR, so it is never lost.
2. From a clean checkout of `main`:

   ```sh
   scripts/patch.sh <sha-on-main>          # latest line
   LINE=0.2 scripts/patch.sh <sha-on-main>  # a specific line
   ```

   This cuts `release/X.Y` from the line's latest tag if it does not exist, cherry-picks the commit with `-x`, and opens a PR against the branch labelled `patch`.
3. Merge the PR.
   The patch workflow bumps `X.Y.Z`, tags it, and publishes through the same release workflow `main` uses.

Rules: only cherry-picked fixes, no features; at most one live release branch per line; delete the branch once the next minor ships.
The full runbook is [`docs/releasing.md`](https://github.com/matthewvilaysack/glide/blob/main/docs/releasing.md).
