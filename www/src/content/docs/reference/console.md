---
title: Console
description: The block console, keys, and what it runs.
---

Bare `glide` on a terminal opens a block console.
Piped or redirected it prints help instead, so scripts don't change.

Every turn is an addressable block carrying the line you typed, a status, and how long it took.
The prompt stays pinned at the bottom, the focus strip sits in the top bar, and blocks can be focused, folded, copied, and re-run.
It runs glide's verbs, not your shell: `ls` and `vim` are not part of the deal.

## Keys

| Key | Prompt mode | Block mode |
| --- | --- | --- |
| `enter` | run the line | fold or unfold the block |
| `esc` | enter block mode | back to the prompt |
| `^k` | block mode, newest block | |
| `j` `k` `↑` `↓` | | move focus |
| `y` | | copy the block |
| `r` | | re-run the block |
| `^u` `^d` | scroll | scroll |
| `^c` | quit | quit |

## Verbs in the console

| Typed | Runs |
| --- | --- |
| `focus` | the strip and the Focus list |
| `focus set\|done\|capture\|log <text>` | the same as the CLI |
| `focus today` | the whole note as a block |
| `doctor` | health checks |
| `index` / `index show` | graph counts (Teams tier) |
| `config` / `config list` | the effective TOML |
| `help`, `clear`, `exit` | handled by the console itself |

Anything else is treated as a question and routed to the Teams-tier verbs (`who owns <path>`, `plan for <person>`, `i need access to <tool>`).
