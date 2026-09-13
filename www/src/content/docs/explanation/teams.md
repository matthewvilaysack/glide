---
title: Teams
description: What a team gets from glide today, and what is still only a plan.
---

A team already shares a repo, so the team feature ships through the repo.
`glide decide --team <text>` appends a line to `DECISIONS.md` at the root of the repo you are standing in, and once you commit it, git carries it to everyone else on their next pull.
`glide decide --against <text>` records the other half, the thing that got ruled out, which is the half worth having.
[Decisions, and why they are not memory](/docs/explanation/decisions/) makes that argument; this page is about what a team gets.

From then on every teammate's `glide prime` puts up to eight recent decisions into each session it opens, the team's first, under a heading that tells the agent not to propose otherwise without saying why.
Drop `--team` and the decision goes to `Decisions.md` at your vault root and stays yours.
`glide decisions` lists both.
Outside a git repo `--team` is an error that says so.

This works now, it is in the binary, and there is no tier and nothing to buy.

## What it costs and what is not proven

A decision costs about twenty tokens to carry, which is nearly nothing, and the [explanation page](/docs/explanation/decisions/) has the measurements.

One number belongs here rather than there, because it is about teams specifically.
Since the hook was installed, `glide prime` fired in 86 of 86 main sessions and in only 21 of 81 subagent sessions, so a teammate whose agent spawns subagents is not yet reliably passing decisions down to them.

What is not measured is whether any of this removes a turn on a real team.
Nobody has run it with more than one person yet, so there is no saving to quote and this page is not going to invent one.
The mechanism is the whole claim: a decision gets written down once, and every session that runs `glide prime` reads it at the start.

None of it depends on glide.
The file format is specified in [SPEC.md](https://github.com/matthewvilaysack/glide/blob/main/SPEC.md) under CC0, a parser is about thirty lines, and glide is one implementation rather than the owner.
The longer argument for why a team needs this at all is at [tryglide.net/decisions.html](https://tryglide.net/decisions.html).

## Later: the onboarding graph

A person who has kept their own priorities and record for a month has, without trying, written most of what the next hire needs on day one: who owns what, what got stuck and how it got unstuck, which commands finally worked.
Shared decisions are that same accumulating record, read for what was settled instead of what happened.
The verbs that turn it into onboarding are already in the binary, over an on-device SQLite graph:

| Verb | What it does |
| --- | --- |
| `glide init` | Bootstrap `.glide/` in a repo |
| `glide index build` | Index CODEOWNERS, git history, and team docs into an on-device SQLite graph |
| `glide who-owns <path> [--why]` | Answer ownership questions with evidence |
| `glide plan <person>` | Draft a day-one plan |
| `glide request <permission>` | Draft an access request |
| `glide friction log\|digest` | Log ramp friction and roll it into a weekly digest |
| `glide doctor` | Health checks for all of the above |

That part is not sold yet, and it is a later and separate thing rather than a paid version of shared decisions.
If you run a team of five to twenty engineers and are hiring this quarter, [write to hello@tryglide.dev](mailto:hello@tryglide.dev).
