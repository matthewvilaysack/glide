# Does telling an agent what your team ruled out change what it proposes?

This is the experiment behind the claim, and it is meant to be re-run rather than believed.
One command, about six minutes, and the raw per-run output is written to `results.jsonl` so anyone can score it differently.

```sh
bench/run.sh 5      # 6 scenarios x 3 arms x 5 trials = 90 runs
python3 bench/report.py
```

## What is being measured

Six realistic engineering questions, each asked inside a small but real repository: a FastAPI service on Postgres that needs caching, a Go service that needs a database layer, a React app that needs state management, and so on.
Every question has an obvious default answer that a competent agent will reach for, and a decision that the team has already made and that points somewhere else.

Three arms, and the third one is the point:

| Arm | What the agent is given |
| --- | --- |
| `none` | nothing |
| `relevant` | the team's decision about this exact question, injected the way `glide prime` injects it |
| `unrelated` | four decisions of the same shape and size about other things |

Without the `unrelated` arm the experiment cannot tell "the decision changed the answer" from "any extra text in the system prompt changed the answer".
That is the difference between a result and a demo.

## What is not being measured, and why

Turns.

The argument for this product is that a rejected proposal costs turns: the agent suggests something, a person says no, and the agent picks again, with the whole context re-read each time.
A headless run has nobody to say no to it, so it cannot produce that number honestly.
Simulating the user who says no would make the simulated user the experiment.

So the endpoint here is what the agent proposes on its first pass, which is upstream of the turns argument rather than a substitute for it.
If the decision does not change what gets proposed, nothing downstream matters.
If it does, the turns claim is still unproven and is labelled that way everywhere it appears.

## Scoring

Two scores per run, from the `FINAL ANSWER:` line and the reasoning above it.

`proposed_ruled_out` is a string match against the thing the decision rules out.

`honours_decision` is judged, not matched, because matching cannot do this job.
One run answered "Procrastinate" to a decision whose words were "the queue is already in Postgres and we use SKIP LOCKED".
Procrastinate is a Postgres queue that claims jobs with `SELECT ... FOR UPDATE SKIP LOCKED`, so that answer honours the decision completely while sharing none of its vocabulary, and a token match scores it as a miss.
The judges see the full recommendation with no indication of which arm produced it, and a second pass audits them for scoring the same recommendation the same way twice.

## The result

90 runs, 2026-09-12, Claude Code headless on the default model. Raw output in `results.jsonl`, judged verdicts in `judged.json`.

| Arm | Honoured the team's decision |
| --- | --- |
| `none` | 15/30, 50.0% |
| `relevant` | 25/30, 83.3% |
| `unrelated` | 15/30, 50.0% |

The `unrelated` arm came out identical to `none`, to the run, which is the part of this that matters most.
Four decisions of the same shape and size about other subjects moved nothing, so what moved the answer was the content of the decision and not the presence of context.

The aggregate hides the shape, so here it is per scenario:

| Scenario | none | relevant | unrelated | |
| --- | --- | --- | --- | --- |
| cache | 100% | 100% | 100% | the repo already implied it |
| jobs | 100% | 100% | 100% | the repo already implied it |
| search | 100% | 100% | 100% | the repo already implied it |
| orm | 0% | 100% | 0% | the decision was decisive |
| deploy | 0% | 100% | 0% | the decision was decisive |
| state | 0% | 0% | 0% | the decision did not take |

Three of the six questions were already settled by the repository itself.
Asked what to use for caching inside a FastAPI service with Postgres in `requirements.txt`, the agent recommends Postgres every time, with or without being told.
On those, writing the decision down bought nothing, and that is worth knowing: a decision that only restates what the code already says is a decision not worth recording.

On the three where the repository does not determine the answer, the split is 0/15 without the decision, 10/15 with it, and 0/15 with unrelated context.
`orm` and `deploy` went from 0/5 to 5/5, deterministically.
Both arms without the decision picked defensible answers (pgx, Google Cloud Run) that simply were not this team's answer (sqlc, a systemd unit on the existing VM).

## What this killed

The premise this experiment was built to test was that an uninformed agent walks into choices the team has already ruled out.

It does not. Across all 90 runs, in every arm, the agent recommended the ruled-out technology exactly **zero** times.
Given a repository to read, it does not reach for Redis, Celery, Elasticsearch, Kubernetes, Redux or GORM on its own.

So the value here is not that a decision file stops bad suggestions, because there were none to stop.
It is that an agent without the decision gives a *generic defensible* answer, and an agent with it gives *your team's* answer.
Google Cloud Run and a systemd unit are both reasonable. Only one of them is what this team runs.

## The scenario that failed

`state` scored 0% in every arm, including with its own decision injected, and it is a flaw in the scenario rather than a finding about the product.

The question asks what to use for *client* state. The decision says Redux is ruled out because the state is server state and TanStack Query owns it.
Those are two different questions, and the agent answered the one it was asked: keep TanStack Query for server data, add Zustand for the client bits.
That is a good answer to a badly posed pair, and it is reported here rather than quietly fixed, because rewriting a scenario after seeing it fail is how a benchmark stops meaning anything.

## Known limits

Six scenarios is small, one of the six was malformed, and three of the remaining five turned out to be questions the repository already answered.
The load-bearing part of this result rests on two scenarios.
That is thin, and the honest reading is that it establishes the mechanism works rather than how much it is worth.

The scenarios were written by the same person who wrote the decisions, which is the standard way to rig a benchmark like this.
The guards are that the repositories are realistic rather than staged, that the `unrelated` arm controls for the mere presence of context, and that every raw output ships in `results.jsonl` so a reader can disagree with the scoring without re-running anything.
The stronger version of this experiment uses scenarios contributed by people who did not write the tool, and that is an open invitation.

One model, one harness, one day.
Nothing here says anything about other models, and the numbers should be expected to move.
