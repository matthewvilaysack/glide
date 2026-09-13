# Working on glide

Glide is a Rust CLI that keeps today's priorities in a Markdown note the person owns, and feeds a coding agent that context at session start through a `SessionStart` hook running `glide prime`.
It also records what a team has already settled into a `DECISIONS.md` that git distributes, which is the part of the product the rest of this file keeps pointing back at.
There is no daemon, no account and no server, and nothing the binary does leaves the machine.

## Which docs are current

`docs/architecture.md` describes an earlier version of this product and does not match the code in `cli/`, so do not plan against it.
`README.md`, `docs/vault-contract.md`, `docs/releasing.md` and `SPEC.md` are the ones that are current.

## Read this before you run any write verb

Every write verb resolves the vault from `~/.glide/config.toml` unless `GLIDE_VAULT_PATH` is set, so running one bare writes to the real Obsidian daily note of whoever is at the keyboard.
That has already cost real notes.
Set a throwaway vault first, every time, including for a one-off check you are sure about:

```sh
export GLIDE_VAULT_PATH="$(mktemp -d)"
glide focus set ship portal tests
```

`glide decide --team` is the exception that variable does not cover.
It resolves the git repository root from the current directory and appends to `<repo>/DECISIONS.md`, which inside this checkout is the project's own decisions file.
Test it from a throwaway `git init` directory instead.

`--safe` refuses the focus write verbs and says so, which is the cheap way to confirm one of those code paths is reached without it touching anything.
It does not cover `glide decide`, which writes either file under `--safe` all the same, so a throwaway repository stays the only way to test `--team`.

## The workspace

The Rust workspace is under `cli/`.
`glide-cli` sits on top and depends on everything else; `glide-common` and `glide-vault` are the leaves and depend on no other glide crate.

| Crate | What it owns | Depends on |
| --- | --- | --- |
| `glide-cli` | The clap surface, one module per verb under `src/commands/`, output formatting, exit codes | everything below |
| `glide-tui` | The block console behind a bare `glide` | common, core, graph, vault |
| `glide-core` | Verb implementations that return structured data: doctor, friction, plan, router, who-owns | common, graph |
| `glide-graph` | The on-device SQLite knowledge graph | common |
| `glide-llm` | Optional LLM client, off by default, `--features anthropic` to enable | common |
| `glide-vault` | The vault contract: the daily note, `decide.rs`, `sprint.rs` | nothing |
| `glide-common` | Config, errors, paths, output stream, shared types | nothing |
| `glide-play` | The same engine compiled to wasm for the browser playground | vault |

`glide-play` is the one crate outside that stack: it is a second consumer of `glide-vault`, not a layer of the binary, which is why a change in the vault crate can break the wasm job.

`glide-core` returns data and `glide-cli` decides how it prints, so a verb that formats its own output is in the wrong crate.
The four focus write verbs refuse empty text rather than obeying it, and an unknown verb exits 2 pointing at its parent rather than guessing.
Keep both when you touch argument handling.

## Build, test, format

Everything below runs from `cli/`, which is what CI does too.

```sh
cd cli
cargo build --release -p glide-cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
../scripts/fmt.sh          # ../scripts/fmt.sh --check in CI
```

Formatting is `scripts/fmt.sh`, not `cargo fmt --all`.
The plain command fails because `glide-llm` declares `mod anthropic` behind a feature whose file does not exist yet, and rustfmt resolves that module before it looks at any flag.
The script formats each crate separately and runs rustfmt directly on `glide-llm/src/provider.rs`, so use it and its `--check` form.

CI runs `../scripts/fmt.sh --check`, clippy with warnings as errors, and the tests on both Linux and macOS, then smoke-runs the built binary.
A second job builds the wasm playground through `scripts/build-play.sh`, so a change in `glide-vault` can break a job you were not looking at.

## The vault contract

Glide reads and edits exactly four `##` sections of today's note: Focus, Tasks, Record, Notes.
Everything else in the file survives byte for byte, including frontmatter, template tags and any other heading, and a missing note is created with those four sections and nothing else.
That is the whole promise the product rests on, so a change that rewrites more of the file than it was asked to is a bug even when the tests pass.
The contract is one page at `docs/vault-contract.md`.

## The decisions format

`SPEC.md` is the normative definition of the `DECISIONS.md` format, published under CC0, and glide is one implementation of it rather than its owner.
The grammar is `- [YYYY-MM-DD ]<decided|ruled out>: <text>`, a line that does not match is ignored rather than an error, and a parser is about thirty lines.
Changing what glide reads or writes means changing the spec in the same commit, because a format with one implementation that drifts is just a file this tool happens to use.
Team decisions are written to `<repo>/DECISIONS.md`, but `team_path_for_read` prefers that file and falls back to `.glide/decisions.md`, which is where earlier versions put it.
That fallback looks like dead code and is not: it is the promise that a repo which already has the old file keeps working and never gets a second one.

## Releases

`scripts/release.sh X.Y.Z` bumps the workspace version, runs the tests, commits, tags and pushes, and the tag is what builds and publishes the binaries.
It refuses to run off `main`, with a dirty tree, behind `origin/main`, or onto a tag that already exists, so a release starts with a clean checkout and not with an argument.
Patches to a shipped line go through `scripts/patch.sh`, and the runbook for both is `docs/releasing.md`.

@DECISIONS.md

That import is the project's own decisions file, carrying what has already been settled here so a session starts knowing it, which is also this repo dogfooding the format it specifies.
