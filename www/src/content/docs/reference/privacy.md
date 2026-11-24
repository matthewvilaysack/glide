---
title: Privacy
description: What glide reads, writes, and never touches.
---

## Reads

- Today's daily note in the folder you configured.
- For the Teams tier, when you run `glide index build`: `CODEOWNERS`, git history, and the team docs globs in your config, in the repo you run it in.

## Writes

- Four sections of today's daily note, and only through the verbs on the [Focus](/docs/reference/focus/) page.
- `~/.glide/config.toml` when you run `glide config set`.
- `.claude/settings.json` (or the global one) when you run `glide setup claude`, and only the one hook entry.
- For the Teams tier: `.glide/glide.db` in the repo.

## Never

- Nothing leaves your machine. There is no telemetry, no account, no server, unless you run `glide mcp up`.
- No network calls at all in the focus features. The Teams tier's optional LLM provider is off by default.
- `--safe` makes every write a refusal.

## When you run `glide mcp up`

- Glide listens on 127.0.0.1 and Tailscale Funnel forwards public HTTPS to it. It stops when you stop it.
- An app gets in only after GitHub sign-in as a login on `[mcp.http] allow`. GitHub tells glide your login and nothing else.
- Apps read notes one request at a time, and a note an app reads goes to that app's AI provider. The vault is never copied or uploaded, and hidden folders and `[mcp.http] exclude` are invisible.
- Glide keeps hashed tokens in `~/.glide/mcp-auth.db` and one line per request (time, login, tool) in `~/.glide/mcp-access.log`.
- The GitHub OAuth App secret stays in the macOS Keychain.

## Questions

hello@tryglide.dev.
