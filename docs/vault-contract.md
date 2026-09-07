# The vault contract

Glide keeps a person's priorities in a folder of Markdown notes they already own.
No database, no account, no server: the folder is usually an Obsidian vault, and iCloud or any folder sync is what carries it between machines.
This doc is the whole contract between glide and that folder.

## One file per day

`[vault] path` names the folder and `[vault] daily_note_pattern` names the file for a day, with `{date}` standing for `YYYY-MM-DD`.
The default pattern is `Daily Notes/{date}.md`, which matches Obsidian's daily-notes plugin.

```toml
# ~/.glide/config.toml
[vault]
path = "~/Library/Mobile Documents/iCloud~md~obsidian/Documents/zettelkasten"
daily_note_pattern = "Daily Notes/{date}.md"
```

`GLIDE_VAULT_PATH` and `GLIDE_VAULT_DAILY_NOTE_PATTERN` override the file for one invocation.
If today's note does not exist, the first write creates it with the four sections below and nothing else.
If it exists, everything outside those four sections is left byte for byte, including frontmatter, template tags, and any other heading.

## Four sections

Glide reads and edits exactly four top-level `##` headings in today's note.
A heading it cannot find is appended at the end of the file the first time something is written to it.

| Section | What it holds | How glide reads it | How glide writes it |
| --- | --- | --- | --- |
| `## Focus` | The day's priorities, in order | Every bullet; `- [ ]`, `- [x]`, and plain `- ` all count | `focus set` tags one bullet `#now` (adding it if new); `focus done` checks it off |
| `## Tasks` | Checkbox bullets, sub-headings allowed | Open and done counts | `focus done` checks off a matching bullet |
| `## Record` | What actually happened | Count of bullets | `focus log` appends `- HH:MM text` |
| `## Notes` | Quick captures | Count of bullets | `focus capture` appends `- text` |

The current focus is the one Focus bullet that ends with `#now`.
Setting a new focus moves the tag; finishing the focused bullet drops it.
Matching for `set` and `done` is a case-insensitive substring, so `glide focus done portal` finds `ship portal tests`.

## The focus line

`glide focus` prints one line meant for a tmux status bar or a shell prompt:

```
▶ ship portal tests · focus 1/3 · 4 open · 2 logged
```

Current focus first, then done-over-total for Focus, open task count, and Record entries.
Parts that would read zero are left out, so a fresh day shows only `▶ no focus set`.
`--json` prints the same snapshot as structured data.

## Over MCP

`glide mcp serve` offers the same operations to whatever agent is running in the terminal, as six tools: `today`, `read_today`, `focus_set`, `focus_done`, `capture`, `log`.
The server's instructions tell the agent to call `today` at the start of a session, `focus_set` when the person says what they are on, `focus_done` when something finishes, and `log` after every task it completes.
Every tool reads the note fresh from disk and writes it back whole, so the person can edit the same file in Obsidian at the same time; the last writer wins, which is the right behaviour for a note.
`--safe` turns every write into a refusal that says so.

## What never goes in the vault

Nothing glide writes carries secrets, tokens, or file contents from the repo.
The tools take one line of text from the agent, and the agent's own contract (the AGENTS.md or rule you give it) is where "no secrets in notes" is enforced.
