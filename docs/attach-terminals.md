# Attaching glide to the terminal you already use

Glide is not a terminal.
It sits under whichever one you run, Warp, iTerm, or a plain tmux, as one binary that two things talk to: the agent in your session over MCP, and the status bar over `glide focus`.
Setting it up is three small edits.

## 1. Tell glide where the notes are

```sh
mkdir -p ~/.glide
cat >> ~/.glide/config.toml <<'TOML'
[vault]
path = "~/Library/Mobile Documents/iCloud~md~obsidian/Documents/zettelkasten"
daily_note_pattern = "Daily Notes/{date}.md"
TOML
glide focus
```

The last line should print `▶ no focus set` or today's focus.
Any folder works; an Obsidian vault on iCloud is the case this was built for.

## 2. Give the agent the workflow

Claude Code, one command:

```sh
glide setup claude --global     # or without --global, for this project only
glide setup claude --check
```

It adds a SessionStart hook that runs `glide prime --hook-json`, so every session (and every compaction, since SessionStart fires again) opens with today's focus, the focus list, and the six verbs in a few hundred tokens.
That is the whole integration; the agent uses the CLI directly from there.
`glide setup claude --remove` takes it out.

Warp: `glide setup warp` prints the rule to paste under Settings, AI, Rules, plus the MCP entry if you want the verbs as tools.
Any other agent: `glide onboard` prints the paragraph for its instructions file.

The MCP server is still there for clients that have no shell, Claude Desktop for one: add `{ "glide": { "command": "glide", "args": ["mcp", "serve"] } }` under `mcpServers`.
Prefer the hook wherever there is a shell; an MCP tool schema rides along on every request, `glide prime` runs once.

## 3. Keep it on screen

tmux, in `~/.tmux.conf`:

```
set -g status-right '#(glide focus 2>/dev/null) '
set -g status-interval 15
```

iTerm2 users on tmux get the same line; without tmux, a shell prompt segment that runs `glide focus` does the job.
Warp shows it through the tmux line as well, or through the agent's replies.

## Day to day

```sh
glide focus set ship portal tests      # what I'm on now
glide focus done portal                 # finished
glide focus capture idea for the demo   # into Notes
glide focus log fixed the flaky build   # into Record with a timestamp
glide focus today                       # the whole note
```

Or say the same things to the agent in the session; it has the same six verbs.
