---
title: Why plain files
description: Why the note is the database, and what that buys you.
---

Glide has no database, no account, and no server.
Today's plan is a Markdown file in a folder you already keep.
This is a decision, not a shortcut.

**You already sync it.** An Obsidian vault on iCloud, a Dropbox folder, a git repo: whatever carries your notes between machines carries glide's state for free.
There is nothing to host and nothing to trust with your day.

**You can read it without glide.** Open the note in any editor and it makes sense.
If glide disappeared tomorrow, the record would still be yours and still be readable.

**Two writers is fine.** You edit the note in Obsidian; the agent writes to it through the verbs.
Every verb reads the file, changes one thing, and writes it back.
Last writer wins, which is the right rule for a note and the wrong rule for a database, which is why glide is not one.

**The contract is small enough to hold in your head.** Four headings, one tag.
The [daily note reference](/docs/reference/daily-note/) is one page, and everything outside those four sections is untouched byte for byte.

The trade is that glide will never do cross-day queries or multi-user graphs from the note alone.
That is what the [Teams tier](/docs/explanation/teams/) and its on-device SQLite graph are for, and they are separate on purpose.
