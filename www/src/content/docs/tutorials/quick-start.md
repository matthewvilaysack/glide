---
title: Quick start
description: Two minutes from install to a focus line in your status bar.
---

## 1. Tell glide where your notes are

Any folder of Markdown works.
An Obsidian vault on iCloud is the case it was built for, because the sync you already have is the cloud.

```sh
mkdir -p ~/.glide
printf '[vault]\npath = "~/path/to/your/notes"\n' > ~/.glide/config.toml
glide focus
```

The last line prints `▶ no focus set` or today's focus.
If your daily notes are not at `Daily Notes/YYYY-MM-DD.md`, set `daily_note_pattern` too; see [Config](/docs/reference/config/).

## 2. Give your agent the workflow

Claude Code:

```sh
glide setup claude --global
```

That installs a SessionStart hook that runs `glide prime --hook-json`, which puts today's focus and the six verbs into every session, and again after every compaction.
Warp: `glide setup warp` prints the rule to paste.
Anything else: `glide onboard` prints the paragraph for its instructions file.
Details per agent are under [Agents](/docs/explanation/agents/).

## 3. Keep it on screen

tmux, in `~/.tmux.conf`:

```
set -g status-right '#(glide focus 2>/dev/null) '
set -g status-interval 15
```

Without tmux, a prompt segment that runs `glide focus` does the same job; see [Status bar](/docs/how-to/status-bar/).

## Day to day

```sh
glide focus set ship portal tests      # what I'm on now
glide focus done portal                 # finished
glide focus capture idea for the demo   # into Notes
glide focus log fixed the flaky build   # into Record, with a timestamp
glide focus today                       # the whole note
```

Or say the same things to your agent.
It has the same verbs.
