#!/usr/bin/env bash
# Cut a release: bump the workspace version, commit, tag, push the tag.
# The Release workflow does the rest. usage: scripts/release.sh 0.2.0
set -euo pipefail
version="${1:?usage: scripts/release.sh X.Y.Z}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version must be X.Y.Z" >&2; exit 1; }
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

[ "$(git branch --show-current)" = "main" ] || { echo "release from main" >&2; exit 1; }
[ -z "$(git status --porcelain)" ] || { echo "working tree must be clean" >&2; exit 1; }
git fetch -q origin main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || { echo "main is not up to date with origin" >&2; exit 1; }
git rev-parse -q --verify "refs/tags/v$version" >/dev/null && { echo "tag v$version already exists" >&2; exit 1; }

current="$(grep -m1 '^version' cli/Cargo.toml | cut -d'"' -f2)"
echo "bumping $current -> $version"
perl -pi -e "s/^version = \"\Q$current\E\"/version = \"$version\"/ && !\$done && (\$done = 1)" cli/Cargo.toml
grep -q "^version = \"$version\"" cli/Cargo.toml || { echo "version bump did not apply" >&2; exit 1; }
(cd cli && cargo update -q --workspace && cargo test -q --workspace)

git add cli/Cargo.toml cli/Cargo.lock
git commit -q -m "Release v$version"
git tag -a "v$version" -m "glide $version"
git push -q origin main "v$version"
echo "pushed v$version; watch: gh run watch --repo matthewvilaysack/glide"
