---
title: CLI
description: Every verb, in one table.
---

```
glide [OPTIONS] [COMMAND]
```

## Global options

| Option | Meaning |
| --- | --- |
| `-p, --prompt <TEXT>` | One-shot: route a natural-language line to the right verb and exit |
| `--json` | Structured output |
| `--stream <human\|ndjson\|off>` | Streaming mode |
| `--no-color` | No ANSI (`NO_COLOR` also honoured) |
| `-q, --quiet` | Only the result |
| `-s, --safe` | Refuse every write and say why |
| `--debug` | Verbose tracing (`GLIDE_LOG=debug`) |

## Priorities

| Command | What it does |
| --- | --- |
| `glide show` (alias `status`) | The strip and the Focus list with the current one marked |
| `glide today` | Print today's note |
| `glide focus` | The one-line strip (`--json` for the snapshot) |
| `glide focus set <text>` | Make this the current focus |
| `glide focus done <text>` | Check off a Focus or Tasks bullet |
| `glide focus capture <text>` | Append to Notes |
| `glide focus log <text>` | Append a timestamped line to Record |
| `glide focus today` | Print the whole note |

## Agents

| Command | What it does |
| --- | --- |
| `glide prime [--hook-json]` | Session-start context for an agent |
| `glide setup claude [--global] [--check] [--remove]` | Install, verify, or remove the Claude Code hook |
| `glide setup warp` | Print the Warp rule |
| `glide onboard` | Print the paragraph for any instructions file |
| `glide mcp serve` | Serve the verbs over MCP (stdio) |

## Console and config

| Command | What it does |
| --- | --- |
| `glide` / `glide console` | Open the block console |
| `glide config list\|get\|set\|paths` | Read or write the layered TOML |
| `glide doctor` | Diagnostics |
| `glide models` | Configured LLM providers |
| `glide tools status\|install` | External tools (repomix) |
| `glide completion <shell>` | Shell completions |

## Teams tier

| Command | What it does |
| --- | --- |
| `glide init [--force]` | Bootstrap `.glide/` |
| `glide build` / `glide index build [--full]` / `show` | Build or inspect the graph |
| `glide who-owns <path> [--top N] [--why]` | Ownership with evidence |
| `glide plan <person> [--role]` | Day-one plan |
| `glide request <permission> [--for]` | Access request |
| `glide friction log <subject>` / `digest` | Friction events and the weekly roll-up |

## Exit codes

`0` ok, `1` general error, `2` usage, `3` config, `4` graph not initialized.
