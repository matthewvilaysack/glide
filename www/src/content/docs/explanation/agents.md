---
title: Agents
description: How an agent in your terminal keeps your priorities current.
---

Glide gives the agent you already run the same six verbs you have, plus two standing instructions: read the focus when a session starts, and log after every task.
There are two ways to deliver that, and the first is the one to prefer.

## Hook plus CLI

`glide prime` prints today's focus, the Focus list, and the six verbs in a few hundred tokens.
`glide setup <agent>` wires it into the agent's own session-start hook.
Every new session opens with it, and since the hook fires again after context compaction, the agent never forgets the workflow.
This costs almost nothing per request.

## MCP server

`glide mcp serve` offers the same operations as six MCP tools over stdio.
An MCP tool schema rides along on every request, so use it where there is no shell (Claude Desktop, for one) and the hook everywhere else.

## Supported

| Agent | Command |
| --- | --- |
| [Claude Code](/docs/how-to/claude-code/) | `glide setup claude [--global]` |
| [Warp](/docs/how-to/warp/) | `glide setup warp` |
| [Codex, Cursor, Gemini CLI, anything with an instructions file](/docs/how-to/other-agents/) | `glide onboard` |
| [Any MCP client](/docs/how-to/mcp/) | `glide mcp serve` |

The design of this page is borrowed from [beads](https://github.com/gastownhall/beads), whose `bd prime` and `bd setup` showed that a CLI plus a hook beats a tool schema for context cost.
