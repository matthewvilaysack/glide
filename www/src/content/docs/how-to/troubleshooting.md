---
title: Troubleshooting
description: The handful of things that go wrong, and what glide says when they do.
---

## "vault path is not set"

`~/.glide/config.toml` has no `[vault] path`, or it points somewhere that does not exist.
Set it (see [Config](/docs/reference/config/)) or export `GLIDE_VAULT_PATH` for one command.

## The strip says "no focus set" but the note has a Focus list

The current focus is the one bullet under `## Focus` that ends with `#now`.
Run `glide focus set <a few words from the bullet>` and the tag moves there.

## Tasks are zero

Glide counts checkbox bullets under `## Tasks` only, including its sub-headings.
Checkboxes under other headings (a Baseline section, say) are left alone on purpose.

## My agent does not mention the focus

Check the hook: `glide setup claude --global --check`.
The hook runs `glide prime --hook-json`; run `glide prime` by hand to see what the agent sees.
The hook only fires on new sessions and after compaction, not mid-session.

## `brew install` cannot download

The tap points at the public releases; if a corporate proxy blocks GitHub downloads, use the [installer](/docs/how-to/install/) with `GLIDE_INSTALL_DIR` set, or build from source.

## `--safe` refused something

That is the point of `--safe`: nothing writes.
Drop the flag when you mean it.

## Reporting a problem

Open an issue on [GitHub](https://github.com/matthewvilaysack/glide/issues) with the output of `glide doctor` and `glide --version`.
