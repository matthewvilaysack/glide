---
title: MCP server
description: The six verbs as tools over stdio.
---

```sh
glide mcp serve
```

Speaks the Model Context Protocol over stdio.
No daemon, no network: the client starts it and owns it.

## Tools

| Tool | Argument | Effect |
| --- | --- | --- |
| `today` | | Today's snapshot: focus, list, counts |
| `read_today` | | The whole note as Markdown |
| `focus_set` | `text` | Same as `glide focus set` |
| `focus_done` | `text` | Same as `glide focus done` |
| `capture` | `text` | Same as `glide focus capture` |
| `log` | `text` | Same as `glide focus log` |

The server's instructions tell the agent to call `today` at the start of a session, `focus_set` when the person says what they are on, `focus_done` when something finishes, and `log` after every task.

## Client config

Claude Code (`.mcp.json`):

```json
{ "mcpServers": { "glide": { "command": "glide", "args": ["mcp", "serve"] } } }
```

Claude Desktop: the same object under `mcpServers` in its config file.
Warp: Settings, AI, MCP servers, command `glide`, arguments `mcp serve`.

`glide --safe mcp serve` makes every write tool answer with a refusal instead of writing.
