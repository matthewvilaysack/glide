---
title: Decisions
description: The verbs that record what is settled, and where the two files live.
---

`glide decide` writes down something that has been settled, so nothing settled gets proposed again.
[`glide prime`](/docs/reference/cli/) then hands those lines to the agent at the start of every session, next to the focus strip.

## The verbs

| Verb | What it does |
| --- | --- |
| `glide decide <text>` | Records a choice as `decided: text` in your own `Decisions.md`. |
| `glide decide --against <text>` | Records a rejection instead, as `ruled out: text`. |
| `glide decide --team <text>` | Writes into the repo's `DECISIONS.md` rather than the vault, so git carries it to every teammate. Combines with `--against`. |
| `glide decisions` | Lists everything, the team's first and then your own. |
| `--json` | Structured output on either verb. `decisions` prints an array of `{ "against", "on", "team", "text" }`; `decide` prints the single entry it wrote, as `{ "against", "on", "team", "recorded", "note" }` with `note` the path of the file it appended to. |

Outside a git repository `--team` is an error that says so and suggests dropping the flag.
Reading degrades quietly there instead: `glide decisions` shows your own and nothing else.

## Where the files live

| File | Holds | Written by |
| --- | --- | --- |
| `Decisions.md` at the root of your vault | your own | `glide decide` |
| `DECISIONS.md` at the root of the repo | the team's | `glide decide --team` |

The team's file is read first and shown first, because the point of putting a line in the shared file was to settle it for everyone.
When the same line appears in both, it is listed once.

A repo that already has `.glide/decisions.md` from an earlier version keeps using that file, and glide never creates a second one.
When both it and a root `DECISIONS.md` exist, the root file is preferred.

`--team` writes the file and stops there, so committing and pushing it is yours to do: [Settle it for the team](/docs/how-to/team-decisions/) walks through that.

## What prime injects

Up to eight recent decisions, under this heading:

```
Already settled, do not propose otherwise without saying why:
- team: ruled out: a hosted sync service; git already distributes files to everyone on the repo
- decided: postgres over sqlite, replication matters
```

Team entries carry a `team: ` prefix and come first.
Eight is a cap on what an agent sees at session start, not on what the files hold; `glide decisions` still lists all of them.

## The format

Both files are plain Markdown you can read, edit, or delete by hand.
The grammar is normative and lives under CC0 in [SPEC.md](https://github.com/matthewvilaysack/glide/blob/main/SPEC.md).
Lines that do not match it are ignored rather than treated as an error, so headings and prose above the list are safe.
Glide is one implementation of that format, not its owner.
