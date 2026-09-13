---
title: Why decisions are not memory
description: The argument for storing what was settled, not just what happened.
---

Every tool that gives a coding agent memory injects a record of what happened: session transcripts, a compressed summary of earlier turns, files and diffs pulled back by retrieval.
That is the shape of the whole category, because capturing history is what these tools are built to do.
None of them injects what was decided, and that is structural rather than an oversight: there is nowhere in any of them that a person writes a conclusion down.

So a team rules an approach out on Tuesday and on Thursday an agent proposes it again to somebody else.
The agent is not misbehaving, because nothing it can see says the question was ever settled.

The waste shows up as turns, and a turn is expensive because a session re-reads its whole context on every one.
Across 227 real sessions here the median session ran 28 assistant turns at about 100,282 tokens re-read per call, so three wasted turns is three full re-reads.
A proposal that was already rejected costs the turn spent making it, the turn spent refusing it, and the turn spent choosing again.
One decision on disk is 82 bytes, about 20 tokens, about 0.02% of one turn.
Storing the conclusion is close to free.
Re-deriving it is not.

[`glide decide --against <text>`](/docs/reference/decisions/) is the half that matters.
What a team chose is already legible without any help: it is in the codebase, the schema, the dependency list, the shape of the tests.
What a team ruled out leaves no trace anywhere, which is exactly why it comes back around.
The first time a team rejects something it is work; every time after that it is waste.

A team's decisions go in a file at the root of the repo rather than into a service, because git already solves distribution.
A commit carries the file to everyone who pulls, which costs the same for two people or two hundred, and there is nothing to deploy and nothing to keep running.
Why a plain file is enough at all is argued in [why plain files](/docs/explanation/plain-files/); this is the part that only matters once there is more than one of you.

What is measured here is only the cost side.
The 82 bytes and the 20 tokens are real, and so are the 28 turns and the 100,282 tokens per call, but nobody has run this with more than one person, so the effect on a real team is not measured and is not claimed.
It is also worth saying plainly that a decisions file with four lines in it from install week helps nobody.
Whatever value is here comes from months of a team actually writing things down, and that has not happened yet.

The longer version of this argument is at [tryglide.net/decisions](https://tryglide.net/decisions.html).
