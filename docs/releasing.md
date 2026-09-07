# Releasing glide

One page, repeatable by hand.
The shape is the one Rust CLIs ship with in the open (ripgrep, fd, starship): a tag builds the binaries, a GitHub Release carries them with checksums, and a Homebrew tap and a one-line installer point at the release.
Nothing is signed or notarized; a `curl | sh` or `brew install` is not quarantined by macOS, and the checksums are what an installer verifies.

## Cut a release

```sh
scripts/release.sh 0.2.0
```

That bumps `[workspace.package] version` in `cli/Cargo.toml`, refreshes `Cargo.lock`, runs the tests, commits `Release v0.2.0`, tags `v0.2.0`, and pushes both.
It refuses to run off `main`, with a dirty tree, behind `origin/main`, or onto an existing tag.

The tag starts `.github/workflows/release.yml`, which:

1. Checks the tag matches the crate version, so a mistyped tag cannot publish the wrong number.
2. Builds `glide` with `--locked` for four targets: Apple Silicon, Intel Mac, Linux x86_64, Linux arm64. Each is smoke-run (`--version`, `--help`) on its own runner.
3. Packages `glide-<version>-<target>.tar.gz` with the binary, the CLI README, and both license files, plus a sha256 per archive.
4. Publishes the GitHub Release with generated notes, the four archives, and one `checksums.txt`.
5. Renders `Formula/glide.rb` and pushes it to the Homebrew tap, if the tap is configured (below). Otherwise the job is skipped, not failed.

Watch it with `gh run watch --repo matthewvilaysack/glide`.
A red build never publishes anything: the release job needs every build job green.
If a build fails on a runner or artifact-service hiccup (an upload timeout, say), the tag is already in place, so rerun only what failed: `gh run rerun <run-id> --repo matthewvilaysack/glide --failed`. The publish and tap jobs run once every build is green.

## Patching a released line

Every release comes off `main`, so once `main` moves on, a fix for what people already run needs its own lane.
The lane is lazy: `release/0.1` is cut from `v0.1.0` only the first time a patch is needed, and every merge to it publishes the next `0.1.x`.

```sh
scripts/patch.sh <commit-on-main>        # or: scripts/patch.sh <sha> 0.1
```

1. The fix lands on `main` through a normal PR first, so it is never lost.
2. `patch.sh` checks every commit is on `main`, cuts the release branch from the line's latest tag if it does not exist, cherry-picks the commits with `-x`, and opens a PR against the release branch labelled `patch`. `LINE=0.1` picks a line other than the latest.
3. CI runs on that PR like any other.
4. Merging it runs `.github/workflows/patch-release.yml`, which bumps PATCH of that line (highest existing tag on the line plus one), commits `Release v0.1.2` on the branch, tags it, and calls the Release workflow with that tag.

Rules that keep a patch a patch: only cherry-picked fixes, no features or refactors; at most one live release branch per line; delete the branch once the next minor ships.
If the fix does not apply to `main` because the code there was rewritten, fix on the release branch directly and say so in the PR.

## Installing what shipped

```sh
curl -fsSL https://raw.githubusercontent.com/matthewvilaysack/glide/main/install.sh | sh
```

Detects OS and CPU, downloads the matching archive and `checksums.txt`, refuses on a mismatch, installs to `~/.local/bin`.
`GLIDE_VERSION=0.2.0` pins; `GLIDE_INSTALL_DIR` moves it.
While the repo is private, set `GITHUB_TOKEN`: the installer then fetches the assets through the GitHub API, since a private repo's release assets are not served from the plain download URL even with a token. Homebrew has no such path, so `brew install` waits for the repo to go public.

```sh
brew install matthewvilaysack/glide/glide
```

Once the tap exists.

## The Homebrew tap

`matthewvilaysack/homebrew-glide` is the tap. The release workflow pushes to it over SSH with a deploy key that lives only on that repo (write access) and as the `HOMEBREW_TAP_DEPLOY_KEY` secret on this one; the `HOMEBREW_TAP_REPO` variable names the tap. No personal token is involved. To rotate: `ssh-keygen -t ed25519`, `gh repo deploy-key add --allow-write` on the tap, `gh secret set HOMEBREW_TAP_DEPLOY_KEY` here.

Every release rewrites `Formula/glide.rb` in the tap.
`scripts/homebrew-formula.sh <version> <dir>` renders the same file locally from a release's checksums if a bump ever has to be done by hand.

## The docs site

`www/` is an Astro Starlight site, laid out on Diátaxis (tutorials, how-to guides, reference, explanation), served at https://tryglide.net/docs through a rewrite on the marketing site.
It deploys as the Vercel project `glide-docs`:

```sh
cd www && npm install && npm run build && vercel --prod --yes
```

Deploy it whenever a page under `www/src/content/docs` changes; it is not wired to git yet.

## What CI checks on every pull request

`.github/workflows/ci.yml` runs on Linux and macOS: formatting (`scripts/fmt.sh --check`), clippy with warnings as errors, the test suite, and a `--version` / `--help` smoke of the built binary.
`scripts/fmt.sh` is also what you run locally; it exists because `cargo fmt --all` trips over the `anthropic` module declared behind a feature whose file is not in the tree yet.

## Known gaps

- The repo is private, so `brew install` from the tap cannot download release assets until the repo is public or the formula learns a token. The workflow is ready for the day it flips.
- No Windows build. Add a `x86_64-pc-windows-msvc` row to the matrix and a `.zip` step when someone asks.
- No self-update. `glide` does not check for new versions; the installer and Homebrew are the update path.
- The `anthropic` feature does not compile (missing module). CI builds the default feature set only.
