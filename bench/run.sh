#!/usr/bin/env bash
# Does telling an agent what a team ruled out change what it proposes?
#
# Three arms per scenario, so "any extra context changed the answer" cannot be
# mistaken for "the decision changed the answer":
#   none       nothing injected
#   relevant   the decision that rules out the obvious choice
#   unrelated  decisions of the same shape and size about other things
#
# Each run gets a real repository, because an agent asked what to use in an empty
# directory is answering in a vacuum and will not reach for the same default.
#
# Two endpoints, both read off a FINAL ANSWER line. A headless run has no user to
# say no to it, so it cannot honestly count turns.
#   proposed_ruled_out  did it name the thing the team ruled out
#   token_match         did the name contain the team's own words (crude)
# Full output is kept per run so a blind judge can score the real question,
# which is whether the answer honoured the decision rather than echoed its words.
# The second is the one that moves. An agent reading a repo with Postgres in it
# does not reach for Redis on its own, so the first is near zero in every arm.
#
# usage: bench/run.sh [trials]   (default 5, so 6 scenarios x 3 arms x 5 = 90 runs)
set -uo pipefail
cd "$(dirname "$0")"
TRIALS="${1:-5}"
PAR="${PAR:-6}"
OUT="results.jsonl"
: > "$OUT"

python3 - "$TRIALS" "$PAR" <<'PY'
import json, subprocess, sys, tempfile, os, pathlib, time, random
from concurrent.futures import ThreadPoolExecutor

trials, par = int(sys.argv[1]), int(sys.argv[2])
S = json.load(open("scenarios.json"))
UNRELATED = """Already settled, do not propose otherwise without saying why:
- team: ruled out: hand-rolled date parsing, use the standard library
- team: decided: trunk based development, no long lived branches
- team: ruled out: mocking the database in tests, we run a real one in CI
- team: decided: structured logs as JSON on stdout"""

def ctx_for(arm, decision):
    if arm == "relevant":
        return f"Already settled, do not propose otherwise without saying why:\n- team: {decision}"
    if arm == "unrelated":
        return UNRELATED
    return None

def one(job):
    s, arm, t = job
    d = tempfile.mkdtemp()
    for rel, body in s["files"].items():
        p = pathlib.Path(d, rel); p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body)
    subprocess.run(["git", "init", "-q", "."], cwd=d, capture_output=True)
    cmd = ["claude", "-p", s["question"]]
    ctx = ctx_for(arm, s["decision"])
    if ctx: cmd += ["--append-system-prompt", ctx]
    start = time.time()
    try:
        r = subprocess.run(cmd, cwd=d, capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, timeout=300)
        out = r.stdout
    except subprocess.TimeoutExpired:
        out = ""
    ans = ""
    for line in out.splitlines():
        if "final answer:" in line.lower():
            ans = line.split(":", 1)[1].strip().strip("*` .")
    low = ans.lower()
    return {"scenario": s["id"], "arm": arm, "trial": t, "trap": s["trap"],
            "seconds": round(time.time() - start, 1), "answer": ans,
            "proposed_ruled_out": s["trap"] in low,
            "token_match": any(tok in low for tok in s["preferred"]),
            "empty": ans == "",
            # Kept so a judge can score whether the answer HONOURED the decision.
            # Token matching cannot: one run answered "Procrastinate", which is a
            # Postgres queue using SKIP LOCKED and so honours a decision whose
            # words were "postgres" and "skip locked", and scored as a miss.
            "output": out.strip()}

jobs = [(s, arm, t) for s in S for arm in ("none", "relevant", "unrelated")
        for t in range(1, trials + 1)]
random.Random(0).shuffle(jobs)   # interleave arms so drift hits all of them equally
with ThreadPoolExecutor(max_workers=par) as ex, open("results.jsonl", "w") as f:
    for i, rec in enumerate(ex.map(one, jobs), 1):
        f.write(json.dumps(rec) + "\n"); f.flush()
        if i % 10 == 0: print(f"  {i}/{len(jobs)}", file=sys.stderr)
print(f"wrote {len(jobs)} runs to bench/results.jsonl", file=sys.stderr)
PY
