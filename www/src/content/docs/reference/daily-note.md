---
title: The daily note
description: The one-page contract between glide and your notes folder.
---

Glide keeps a person's priorities in a folder of Markdown notes they already own.
This page is the whole contract.

## One file per day

`[vault] path` names the folder and `[vault] daily_note_pattern` names the file for a day, with `{date}` standing for `YYYY-MM-DD`.
The default pattern is `Daily Notes/{date}.md`, which matches Obsidian's daily-notes plugin.

If today's note does not exist, the first write creates it with the four sections below and nothing else.
If it exists, everything outside those four sections is left byte for byte: frontmatter, template tags, and any other heading.

## Four sections

| Section | What it holds | How glide reads it | How glide writes it |
| --- | --- | --- | --- |
| `## Focus` | The day's priorities, in order | Every bullet; `- [ ]`, `- [x]`, and plain `- ` all count | `set` tags one bullet `#now`; `done` checks it off |
| `## Tasks` | Checkbox bullets, sub-headings allowed | Open and done counts | `done` checks off a matching bullet |
| `## Record` | What actually happened | Count of bullets | `log` appends `- HH:MM text` |
| `## Notes` | Quick captures | Count of bullets | `capture` appends `- text` |

The current focus is the one Focus bullet that ends with `#now`.
Setting a new focus moves the tag; finishing the focused bullet drops it.

## Example

```markdown
# 2026-09-07

## Focus
- [ ] ship portal tests #now
- [x] clock in
- write roadmap

## Tasks
### Health
- [ ] run

## Record
- 09:12 stood up, picked portal tests
- 10:40 fixed the flaky build, retry on the artifact step

## Notes
- idea: focus strip in the prompt too
```

That note renders as `▶ ship portal tests · focus 1/3 · 1 open · 2 logged`.

## Concurrency

There is no lock.
Every verb reads the file, changes one thing, and writes it back; the last writer wins, which is the right behaviour for a note you also edit by hand.

## What never goes in

Nothing glide writes carries secrets or file contents from a repo.
The verbs take one line of text; the agent's own instructions are where "no secrets in notes" is enforced.
