#!/usr/bin/env bash
# Open a patch PR: cherry-pick one or more commits that already landed on main
# onto the lazy release branch for a line, creating the branch from the line's
# latest tag if it does not exist yet. Merging the PR publishes the next PATCH.
#   scripts/patch.sh <sha> [<sha>...]          line = the latest released line
#   LINE=0.1 scripts/patch.sh <sha> [<sha>...] a specific line
set -euo pipefail
[ $# -ge 1 ] || { echo "usage: [LINE=X.Y] scripts/patch.sh <commit-on-main> [<commit>...]" >&2; exit 1; }
root="$(cd "$(dirname "$0")/.." && pwd)"; cd "$root"
git fetch -q origin --tags
for sha in "$@"; do
  git merge-base --is-ancestor "$sha" origin/main || { echo "$sha is not on main; land the fix on main first" >&2; exit 1; }
done

if [ -n "${LINE:-}" ]; then line="$LINE"; else
  latest="$(git tag --list 'v*' --sort=-v:refname | head -n1)"; latest="${latest#v}"
  line="${latest%.*}"
fi
branch="release/$line"
if ! git ls-remote --exit-code --heads origin "$branch" >/dev/null; then
  base="$(git tag --list "v$line.*" --sort=-v:refname | head -n1)"
  [ -n "$base" ] || { echo "no released tag on line $line to cut $branch from" >&2; exit 1; }
  echo "cutting $branch from $base"
  git push -q origin "$base^{commit}:refs/heads/$branch"
fi

short="$(git rev-parse --short "$1")"
work="patch/$line-$short"
git checkout -q -B "$work" "origin/$branch"
git cherry-pick -x "$@"
git push -q -u origin "$work"
subject="$(git log -1 --format=%s "$1")"
shorts=""; for sha in "$@"; do shorts="$shorts $(git rev-parse --short "$sha")"; done
body="Cherry-pick of${shorts} from main. Merging publishes the next $line.x patch. No features, no floor bumps."
gh pr create --base "$branch" --head "$work" --label patch \
  --title "Patch $line: $subject" \
  --body "$body"
