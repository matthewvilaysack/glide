---
title: A first day with an agent
description: A worked hour, from an empty note to a Record that writes itself.
---

This tutorial walks one real hour.
You need glide [installed](/docs/how-to/install/) and pointed at a notes folder, and Claude Code with the hook from `glide setup claude --global`.
Everything here is copy-paste.

## 1. Start the day

```sh
glide focus
```

```
▶ no focus set
(no note yet at ~/notes/Daily Notes/2026-09-07.md)
```

Nothing exists yet.
Say what the day is about:

```sh
glide focus set ship the portal tests
glide focus set write the roadmap
glide focus set ship the portal tests
```

```
✓ now write the roadmap
▶ write the roadmap · focus 0/2
✓ now ship the portal tests
▶ ship the portal tests · focus 0/2
```

The note now exists with two Focus bullets; the one you named last carries the tag.
Open it in your notes app if you like: it is a normal Markdown file.

## 2. Put it where you type

```
set -g status-right '#(glide focus 2>/dev/null) '
```

in `~/.tmux.conf`, then `tmux source ~/.tmux.conf`.
The strip is now in the corner of every pane.

## 3. Start an agent session

```sh
claude
```

The session opens with today's focus in its context.
Ask it: *what am I on?* It answers with the focus line.
Now give it a task:

> make the flaky portal test pass

When it finishes, look at the note:

```sh
glide focus today
```

```markdown
## Record
- 10:42 Made the portal test deterministic by waiting on the build step instead of a fixed sleep.
```

You did not ask for that line.
The hook's instructions did.

## 4. Capture something mid-conversation

Tell the agent: *remember to ask about the cache revert.*

```markdown
## Notes
- ask about the cache revert
```

## 5. Finish the thing

```sh
glide focus done portal
```

```
✓ done ship the portal tests
▶ no focus set · focus 1/2 · 1 logged
```

The tag came off, the box got checked, and the strip counts it.
Set the next one and keep going.

## What you have at the end of the day

One Markdown file, in your own folder, with what you meant to do, what happened, and what came up, in order, with times.
Friday reads it back.
