import json, collections
rows=[json.loads(l) for l in open("results.jsonl")]
arms=("none","relevant","unrelated")
print(f"{len(rows)} runs, {len([r for r in rows if r['empty']])} with no parseable answer\n")

print("=== agrees with what the team settled on ===")
for arm in arms:
    rs=[r for r in rows if r["arm"]==arm]
    n=sum(1 for r in rs if r["agrees_with_team"])
    print(f"  {arm:10s} {n:3d}/{len(rs):3d}  {100*n/len(rs):5.1f}%   {'#'*int(30*n/len(rs))}")

print("\n=== proposed the thing the team ruled out ===")
for arm in arms:
    rs=[r for r in rows if r["arm"]==arm]
    n=sum(1 for r in rs if r["proposed_ruled_out"])
    print(f"  {arm:10s} {n:3d}/{len(rs):3d}  {100*n/len(rs):5.1f}%")

print("\n=== per scenario: agrees with the team (none -> relevant) ===")
print(f"  {'scenario':10s} {'none':>7s} {'relevant':>9s} {'unrelated':>10s}   what the team settled on")
S={s['id']:s for s in json.load(open('scenarios.json'))}
for sc in S:
    cell={}
    for arm in arms:
        rs=[r for r in rows if r["scenario"]==sc and r["arm"]==arm]
        cell[arm]=sum(1 for r in rs if r["agrees_with_team"])/max(1,len(rs))
    print(f"  {sc:10s} {cell['none']*100:6.0f}% {cell['relevant']*100:8.0f}% {cell['unrelated']*100:9.0f}%   {'/'.join(S[sc]['preferred'][:2])}")

print("\n=== what it actually answered, by scenario and arm ===")
for sc in S:
    print(f"  {sc}:")
    for arm in arms:
        a=collections.Counter(r["answer"] for r in rows if r["scenario"]==sc and r["arm"]==arm)
        print(f"     {arm:10s} " + ", ".join(f"{k or '(none)'} x{v}" for k,v in a.most_common(4)))
