---
title: Other agents
description: Codex, Cursor, Gemini CLI, and anything with an instructions file.
---

```sh
glide onboard
```

prints one paragraph.
Paste it into whatever file your agent reads at the start of a session: `AGENTS.md`, `.cursorrules`, `GEMINI.md`, a project `CLAUDE.md`.

If the agent has a session-start hook of its own, point it at `glide prime` instead; the output is plain Markdown, and `--hook-json` wraps it in the envelope Claude Code, Codex, and Gemini CLI share.

If the agent speaks MCP and has no shell, use the [MCP server](/docs/how-to/mcp/).
