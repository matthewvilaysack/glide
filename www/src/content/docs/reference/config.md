---
title: Config
description: The layered TOML config and every key glide reads.
---

Glide reads config in layers, later ones winning:

1. Compiled defaults
2. `~/.glide/config.toml` (yours, every project)
3. `<repo>/.glide/glide.toml` (a project's, committed)
4. `<repo>/.glide/config.local.toml` (a project's, ignored by git)
5. `GLIDE_*` environment variables

`glide config list` prints the effective result; `glide config paths` prints where each layer lives; `glide config set <key> <value>` writes the project-local layer.

## `[vault]`

Where your daily notes are.
This is the only section you need for the focus features.

```toml
[vault]
path = "~/Library/Mobile Documents/iCloud~md~obsidian/Documents/vault"
daily_note_pattern = "Daily Notes/{date}.md"
```

| Key | Default | Meaning |
| --- | --- | --- |
| `path` | empty | Folder of Markdown notes. `~` is expanded. Empty means "not set up" and every focus verb says so. |
| `daily_note_pattern` | `Daily Notes/{date}.md` | Relative path of one day's note; `{date}` becomes `YYYY-MM-DD`. |

| `focus_heading` | `Focus` | The `##` heading that holds the day's priorities. Set it to whatever your template calls that list (`Checklist`, `Today`). |
| `tasks_heading` | `Tasks` | The `##` heading with checkbox tasks. |
| `record_heading` | `Record` | Where `log` appends. |
| `notes_heading` | `Notes` | Where `capture` appends. |

If today's note lacks the focus heading, the first `focus set` appends that section at the end of the file rather than guessing.

Environment overrides: `GLIDE_VAULT_PATH`, `GLIDE_VAULT_DAILY_NOTE_PATTERN`.

## `[graph]`, `[index]`, `[llm]`, `[permissions]`

These belong to the [Teams tier](/docs/explanation/teams/): the on-device SQLite graph of ownership and permissions, what the indexer reads, and which model answers questions.
`glide init` writes a starter file with every key and a comment on each.

## Output

| Flag | Effect |
| --- | --- |
| `--json` | Structured output for every verb |
| `--stream human\|ndjson\|off` | Streaming mode |
| `--no-color` | No ANSI, also honoured via `NO_COLOR` |
| `--quiet` / `-q` | Only the result |
| `--safe` / `-s` | Refuse every write and say why; the vault included |
