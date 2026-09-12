# DECISIONS.md

A file format for the decisions a team has already made, written so that any coding agent can read them.

Version 0.1.
This document is the specification.
[glide](https://github.com/matthewvilaysack/glide) is one implementation of it, and the format is not owned by it.

## Why this exists

Every tool that gives an agent memory injects what *happened*.
Session transcripts, compressed command output, retrieved files, embeddings of past work: all of it is recall.
None of them inject what was *decided*, because none of them have a place where a person writes a decision down.

So a team rules out Mongo on a Tuesday, and on Thursday an agent proposes it again to somebody else, and that person spends turns saying no and waiting for the next suggestion.
The cost is not the wasted sentence.
A session re-reads its entire context on every turn, so a rejected proposal costs the turns spent making it, refusing it, and choosing again.

A decision takes about twenty tokens to state.
That is roughly two hundredths of one percent of a single turn, which makes this the cheapest context an agent can be given and the only kind that gets *more* valuable as it accumulates.

## The format

A repository MAY contain a file named `DECISIONS.md` at its root.
Tools SHOULD also read `.glide/decisions.md`, which earlier versions of glide wrote, and SHOULD prefer the root file when both exist.

The file is Markdown, meant to be read and edited by people.
Everything a tool cares about is in the list items.

```markdown
# Decisions

What has been settled, so nothing settled gets proposed again.

- 2026-09-12 ruled out: Mongo, we are on Postgres and the ops story is settled
- 2026-09-12 decided: server-side rendering for the marketing pages
- 2026-09-14 decided: one binary, not a daemon
```

### Entries

An entry is a Markdown list item matching:

```
- [YYYY-MM-DD ]<verb>: <text>
```

- The leading `- ` is required. `* ` SHOULD also be accepted.
- The date is OPTIONAL and ISO 8601 (`YYYY-MM-DD`). When present it is the date the decision was made, not the date the line was edited.
- `<verb>` is `decided` or `ruled out`, followed by a colon and a space.
- `<text>` is free prose to the end of the line, in the words the team would actually use. It is not a key, an identifier, or a schema.

Lines that do not match are not entries.
A parser MUST ignore them rather than fail, because the file is a human document and will accumulate headings, blank lines, and paragraphs.

### The two verbs

`decided` records a choice.
`ruled out` records a rejection.

Both are required, and the second is the one that matters.
Rejections are the half that agents repeat, because a chosen approach is visible in the codebase and a rejected one leaves no trace anywhere.
A format with only `decided` would miss the entire problem.

### Ordering

Entries are append-only and newest last.
A tool that shows a subset SHOULD show the most recent.

Append-only is what makes the file survive git.
Two people adding a decision on two branches conflict at the last line, and the resolution is to keep both, which is a resolution a person can do correctly without thinking about it.

### Scope and precedence

`DECISIONS.md` is the team's, because it is in the repository and git distributes it.

A tool MAY also keep a personal decisions list somewhere outside the repository.
When it does, and the two disagree, the team's entry takes precedence and SHOULD be presented first, because the point of writing it in the shared file was to settle it for everyone.
A tool SHOULD list a decision once when the same text appears in both.

### What is deliberately not in this format

No identifiers, no status field, no supersedes link, no author, no tags, no YAML front matter, no per-decision file.

Architecture Decision Records already exist and already have all of that, and they are written for humans to read in review.
This is the other thing: a few lines an agent reads at the start of every session, cheap enough that nobody thinks about the cost of adding one.
Every field this format does not have is a field somebody has to fill in, and a decision that is annoying to record does not get recorded.

If an entry needs context, it belongs in prose in the same file, above the list.
Parsers ignore it and people read it.

## Using it

An agent harness SHOULD read `DECISIONS.md` once at session start and present the entries to the model, most recent last, with a line telling it these are settled.
The wording that glide uses:

```
Already settled, do not propose otherwise without saying why:
- ruled out: Mongo, we are on Postgres and the ops story is settled
- decided: server-side rendering for the marketing pages
```

The escape hatch is deliberate.
A decision is a default and not a prohibition, and an agent that has a real reason to reopen one should say the reason rather than silently comply.

Reading the file is the whole integration.
There is no server to call, no account, no API key, and no daemon.
A tool that can read a file in the repository it is already working in can implement this completely.

## Why a file and not a service

Because the distribution problem is already solved.

Every team that would want this already runs the thing that copies files to everyone working on a repository, keeps their history, reviews their changes, and handles the conflicts.
A decision committed to the repo reaches every teammate at their next pull, with no directory of who is on the team and nothing to keep running.
That is why it costs the same for two people and for two hundred: the marginal teammate is a clone.

It also means nothing leaves the machine, the file is prose a person can edit or delete, and a team that abandons every tool implementing this keeps everything it wrote.

## Implementing it

The parser is about thirty lines.
That is intentional, and it is the point rather than a limitation: a format is only worth writing to if the next agent someone runs can read it, and a format that is expensive to implement does not get implemented.

To be compatible, a tool needs to:

1. Read `DECISIONS.md` from the repository root, falling back to `.glide/decisions.md`.
2. Parse list items matching the entry grammar, and ignore every line that does not match.
3. Treat a missing file as an empty list, never an error. Most directories are not repositories, and a session has to start either way.
4. Preserve unrecognized content byte for byte when writing. The file belongs to the team, not to the tool.

If you implement this, open an issue on the glide repository and it will be listed here.
A format with one implementation is a file format.
A format with three is a convention, and that is the only version of this worth anyone's time.

## Licence

This specification is published under CC0: copy it, implement it, fork it, or vendor it without asking.
Nothing about the format is proprietary, and the point of writing it down is that other tools read the same file.
