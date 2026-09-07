---
title: Privacy
description: What glide reads, writes, and never touches.
---

## Reads

- Today's daily note in the folder you configured.
- For the Teams tier, when you run `glide index build`: `CODEOWNERS`, git history, and the team docs globs in your config, in the repo you run it in.

## Writes

- Four sections of today's daily note, and only through the verbs on the [Focus](/docs/usage/focus/) page.
- `~/.glide/config.toml` when you run `glide config set`.
- `.claude/settings.json` (or the global one) when you run `glide setup claude`, and only the one hook entry.
- For the Teams tier: `.glide/glide.db` in the repo.

## Never

- Nothing leaves your machine. There is no telemetry, no account, no server.
- No network calls at all in the focus features. The Teams tier's optional LLM provider is off by default.
- `--safe` makes every write a refusal.

## Questions

hello@tryglide.dev.
