# Glide console PRD

The problem statement and solution behind the block console that ships in the `glide` binary.
The design contract for the surface is [`a2ui-cli-design.md`](a2ui-cli-design.md), the implementation notes live in [`../cli/README.md`](../cli/README.md), and the product requirements it serves are in [`requirements.md`](requirements.md).
This doc is the why.

## Problem

A new engineering hire loses their first weeks to interrupt questions.
Who owns this service, how do I get access to that tool, what should I even be doing today.
Each question costs twice: the hire stalls until someone answers, and a senior engineer gets pulled out of their own work to answer it.
The pitch doc calls these the four walls: permissions, teams, tools, friction.

Glide already answers these questions from a private, on-device knowledge graph.
But until now the binary only answered them as one-shot CLI calls, and that shape fights the actual situation the hire is in:

The answer scrolls away.
An ownership answer is only trustworthy with its evidence attached, and evidence in scrollback is gone the moment the next command runs.
The hire re-asks, or worse, half-remembers.

Every question is a cold start.
Onboarding questions come in chains: who owns billing leads to how do I get access to the billing sandbox leads to log that I was blocked on it.
One-shot invocations make the hire re-establish context on every link of that chain.

You have to already know the verb.
`glide who-owns src/billing --why` is the syntax of someone who has read the manual.
A hire on day one does not know the flag exists, which means the tool is least usable for exactly the person it exists for.

Half the surface was invisible.
Diagnostics, the graph inspector, config, friction logging: all real, all shipped, none discoverable from where the hire actually was.

Meanwhile the tool that new engineers increasingly reach for first is a modern terminal, and the best of those proved a specific interaction model: every command and its output live together as an addressable block you can revisit, copy, and re-run.
That model is exactly what a question-with-evidence workflow needs, and we were not using it.

## Who this is for

The new hire, in their first two weeks, in a repo they have just cloned.
Secondarily the engineer running a Phase 0 concierge engagement, who needs to demo the product answering real questions against a real repo without a hosted backend, a browser, or an apology.

## Solution

The glide console: a block-oriented terminal surface over the same knowledge graph, opened by running bare `glide` in a repo.

One prompt takes everything.
`who owns src/App.tsx`, `i need access to pomelo`, and `doctor` are typed the same way into the same line.
Natural questions route to the right verb; typed verbs run directly; the hire never has to know which kind of thing they typed.

Every answer is a block.
A block carries the line as typed, what kind of answer it is, how long it took, whether it succeeded, and the full answer with its evidence.
Blocks can be focused, collapsed, copied, and re-run.
The chain of questions the hire asked this session stays on screen as a chain, which is the difference between a transcript and a scrollback.

Evidence stays attached.
An ownership answer renders each owner with its weight, its source, and the why, and copying the block copies all of it, so the answer can be pasted into a Slack thread intact.

The whole binary is reachable.
Health checks, graph counts, index rebuild, config, model status, tool detection, and friction logging all render as blocks in the same stream as the questions.
Friction logging in particular moves from a verb nobody would find to a one-line habit, which is what the manager digest depends on.

Trust boundaries hold.
Safe mode carries into the console and refuses the three verbs that write, saying why.
Rebuilding the index requires the explicit spelling, because it drops and rewrites the graph.
A repo with no graph yet still opens and tells you how to build one, rather than refusing a hire who just cloned.

## What this is not

It is not a terminal emulator.
It runs glide's verbs, not a shell, and says so rather than half-pretending with a broken passthrough.
The productive version of that refusal is R009: the console becomes the thin layer that lists, launches, and routes to the CLI agent sessions the hire already runs, tmux and Claude Code first, and hands off to tmux for anything interactive.

It is not an LLM chat.
Routing is deterministic keyword matching in this version.
When language-model routing lands it slots in behind the same prompt without the surface changing.

It is not the fillable permission form from the design contract yet.
A request renders as a draft with its channels; submitting is still explicit.
Interactive widgets inside a block are the natural next slice.

## Requirements served

| Requirement | How the console serves it |
|---|---|
| R002 day-1 plan | `plan for <person>` renders the plan as a block the hire keeps on screen |
| R003 ownership answers | leads with the top owner, shows weight and evidence per hit, says plainly when the graph has no owner |
| R004 permission requests | plain sentence in, concrete draft out with channels named, nothing auto-submitted |
| R005 friction | logging becomes a one-liner at the moment of friction, digest is a block in the same stream |
| R006 data handling | everything stays in the repo's own `.glide/`, safe mode refuses writes |

## Success measures

The hire answers their own question without a senior engineer, which is the Phase 0 exit bar, and the console is now the shortest path to that answer.
Worth watching specifically: whether friction events actually get logged once logging is one line in the surface the hire already has open, and whether the demo path can run end to end in the console alone.

## Open questions

Whether the permission block should become the fillable form before or after language-model routing.
Whether the manager digest wants a rendered surface here or stays an emailed artifact.
Whether `ask`, grounded answers with citations over the indexed repo text, is the next verb, since the savings page already promises it and the repomix index the graph builds is most of the input.
Whether agent-session management (R009) lands before or after `ask`, since listing tmux sessions is a small verb and the payoff of one surface over questions, requests, and running agents is the whole thesis.
