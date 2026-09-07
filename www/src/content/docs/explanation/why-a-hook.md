---
title: Why a hook, not a tool schema
description: The context-cost argument behind glide prime.
---

There are two ways to give an agent a tool: describe it in the request as a tool schema, or tell the agent once how to run a command it already has.

An MCP tool schema rides along on every request.
Six tools with descriptions is a few thousand tokens, paid on every turn, whether or not the agent uses them.
Across a long session that is real cost, real latency, and a little less attention for the work.

`glide prime` takes the other road.
It prints today's focus, the list, and the six verbs in a few hundred tokens, once, when the session starts.
Because Claude Code's SessionStart hook fires again after context compaction, the agent gets it back exactly when it would otherwise forget.
After that the agent runs `glide focus ...` like any other shell command.

This is the design [beads](https://github.com/gastownhall/beads) proved with `bd prime`, and glide borrows it deliberately.
The MCP server still exists for clients that have no shell, Claude Desktop for one.
Where there is a shell, use the hook.

The same reasoning is why glide ships no agent-specific skill file: a plain CLI plus a paragraph works in Claude Code, Codex, Cursor, Warp, and whatever comes next.
