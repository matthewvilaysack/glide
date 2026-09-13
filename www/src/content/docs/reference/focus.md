---
title: Focus
description: The verbs and what each one changes in today's note.
---

`glide focus` is the whole day-to-day surface.
Every verb reads today's note fresh from disk and writes it back whole, so you can edit the same file in Obsidian at the same time.

## The strip

```sh
glide focus
```

```
▶ ship portal tests · focus 1/3 · 4 open · 2 logged
```

Current focus first, then done-over-total for the Focus list, open task count, and Record entries.
Parts that would read zero are left out, so a fresh day shows only `▶ no focus set`.
`--json` prints the same snapshot as data.

## The verbs

| Verb | What it does |
| --- | --- |
| `glide focus set <text>` | Makes this the current focus. Matches an existing Focus bullet by case-insensitive substring, otherwise adds a new one. Moves the `#now` tag. |
| `glide focus done <text>` | Checks off the first Focus or Tasks bullet matching the text and drops the tag if it had it. |
| `glide focus capture <text>` | Appends `- text` to Notes. |
| `glide focus log <text>` | Appends `- HH:MM text` to Record. |
| `glide focus today` | Prints the whole note. |

Matching is forgiving on purpose: `glide focus done portal` finds `ship portal tests`.
When nothing matches, `done` says so and changes nothing.

The four write verbs refuse text that is empty or only whitespace.
`set`, `done`, `capture`, and `log` each exit `1`, name the verb that needs words, and show the shape of the command, leaving the note alone.
It matters most for `done`, where an empty string matches every bullet and would otherwise check off whichever one came first, and for `log`, which would otherwise append a bare timestamp.

## Writing rules

- A missing note is created with just the four sections.
- A section the note lacks is appended at the end the first time something is written to it.
- New bullets go after the last non-blank line of the section, so the blank line before the next heading stays.
- `--safe` turns every write into a refusal that says so.
