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

## 2. Give the agent the tools

Claude Code, per project or globally in `~/.claude/.mcp.json`:

```json
{
  "mcpServers": {
    "glide": { "command": "glide", "args": ["mcp", "serve"] }
  }
}
```

Warp: Settings, AI, MCP servers, add a server with command `glide` and arguments `mcp serve`.
Any other MCP client takes the same command.

Two Claude Code hooks make the priorities show up without asking.
Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [{ "hooks": [{ "type": "command", "command": "glide focus --quiet" }] }],
    "Stop": [{ "hooks": [{ "type": "command", "command": "glide focus --quiet" }] }]
  }
}
```

The start hook puts today's focus line into the session context; the stop hook prints it again when the agent finishes, so the priority is the last thing on screen.
The MCP server's own instructions ask the agent to log what it did after each task, so the Record section fills in behind you.

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
