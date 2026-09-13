---
title: Settle it for the team
description: Record a decision in the repo so git carries it to everyone else.
---

```sh
glide decide --team --against "rewrite the uploader in Go"   # ruled out, for everyone
glide decide --team "uploads stay chunked at 5 MB"           # settled, for everyone
git add DECISIONS.md && git commit -m "rule out the Go uploader rewrite"
git push
```

The first command appends one line to `DECISIONS.md` at the root of the repo you are standing in.
It writes the file and stops there.
The commit is the step people forget: until it is committed and pushed, the decision is only on your machine, and a teammate who pulls gets nothing.
Once it is pushed they pick it up on their next pull, and their `glide prime` puts it in front of their agent at the start of the next session.
Prime carries the eight most recent decisions, so on a long file the older lines stay in the file and out of the session.

If the repo already has a `.glide/decisions.md`, that is the file glide keeps writing to and the one to `git add` instead.
glide never creates a second file.

## When to use --team

Use `--team` when a teammate proposing the opposite would be wrong.
Drop it for how you personally work, which goes to `Decisions.md` at your vault root and stays yours.
A Makefile target you happen to dislike is not a team decision, the uploader chunk size is.
Either way, `--against` is the flag for the thing that got ruled out rather than chosen.

## When two branches both add one

The file is append-only, so two people writing decisions on two branches conflict at the last line.
Keep both lines.
That is the entire resolution.

## Outside a git repo

`--team` is an error there, and says so, because there is no repo to put the file in.
Drop the flag and the decision goes to your own `Decisions.md`.
Reading degrades quietly instead: `glide decisions` and `glide prime` show your personal decisions and say nothing about the missing repo.

## Verify

```sh
glide decisions
git log @{u} -1 -- DECISIONS.md
```

The first lists team decisions ahead of your own.
The second reads the branch as the remote has it, so nothing there (or an older commit than the one you just made) means the decision has not left your machine yet.

Why a team would keep these at all is [Teams](/docs/explanation/teams/).
