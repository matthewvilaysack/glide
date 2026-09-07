---
title: Claude Code
description: One command installs the SessionStart hook.
---

```sh
glide setup claude --global   # ~/.claude/settings.json, every project
glide setup claude            # .claude/settings.json, this project only
glide setup claude --check    # is it installed?
glide setup claude --remove   # take it out
```

The hook it installs:

```json
{
  "hooks": {
    "SessionStart": [
      { "matcher": "", "hooks": [{ "type": "command", "command": "glide prime --hook-json", "timeout": 5 }] }
    ]
  }
}
```

`glide prime --hook-json` wraps the context in the envelope Claude Code expects.
No PreCompact hook is needed: SessionStart fires again after compaction.

## What the agent sees

```
## glide: this person's priorities

Today (2026-09-07): ▶ ship portal tests · focus 1/3 · 4 open · 2 logged
Current focus: ship portal tests
Focus list:
- [ ] ship portal tests (now)
- [x] clock in
- [ ] write roadmap

Workflow: mention the current focus in one line at the start. When the person says what they are on, run `glide focus set <text>`. When something finishes, `glide focus done <text>`. After every task you complete, `glide focus log <one or two sentences>`, without being asked. ...
```

## The tools as MCP too

Optional, for when you want the verbs as tools rather than shell commands.
Add to `.mcp.json` (project) or `~/.claude/.mcp.json`:

```json
{ "mcpServers": { "glide": { "command": "glide", "args": ["mcp", "serve"] } } }
```

## Verify

Start a new session and ask "what am I on?".
The answer should be today's focus, and after the next task there should be a new line under Record.
