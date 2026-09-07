# glide CLI

Single Rust binary for [glide](https://tryglide.dev): keeps your priorities in view while you work (`glide focus`, `glide mcp serve`), and answers ownership questions, drafts day-1 plans, and files access requests from a local SQLite knowledge graph indexed off the repo.

Shape borrows from [NCA CLI](https://nca-cli.com/docs/overview/): flat verb tree, layered TOML config, `--stream {human|ndjson|off}` + `--json`, no Node/JS dependency.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/matthewvilaysack/glide/main/install.sh | sh
# or, once the tap is up:
brew install matthewvilaysack/glide/glide
```

Releases are tag-driven; see [`../docs/releasing.md`](../docs/releasing.md).

## Quick start

```bash
cargo build --release
./target/release/glide init
./target/release/glide index build
./target/release/glide who-owns docs/requirements.md --why
```

## Verbs

| Command | What it does |
|---|---|
| `glide` / `glide console` | Open the block console (see below) |
| `glide init` | Bootstrap `.glide/` in the current repo |
| `glide index build [--full] [--since <ref>]` | Index CODEOWNERS + git history + team docs into SQLite |
| `glide index show [--path <p>] [--json]` | Inspect the graph |
| `glide who-owns <path-or-glob> [--top N] [--why]` | Answer ownership questions |
| `glide plan <person> [--role <role>]` | Generate a day-1 plan |
| `glide request <permission> [--for <person>]` | Draft an access request |
| `glide friction log <event> [--severity 1-5]` | Log a friction event |
| `glide friction digest [--since <duration>]` | Roll friction into a weekly digest |
| `glide focus [set\|done\|capture\|log\|today]` | Keep today's priorities in view; edits the daily note in your vault |
| `glide mcp serve` | Serve the focus verbs to a terminal agent over MCP (stdio) |
| `glide config [get\|set\|list]` | Read/write the layered TOML |
| `glide doctor` | Diagnostics |
| `glide models` | List configured LLM providers |
| `glide completion <shell>` | Emit shell completions |

## Console

Bare `glide` on a terminal opens a block console. Piped or redirected it still
prints help, so nothing scripted changes.

The model is borrowed from Warp. Every turn is an addressable block carrying the
line you typed, a status, and how long it took. The prompt stays pinned at the
bottom, and blocks can be focused, folded, copied, and re-run. What is *not*
borrowed is the PTY: this runs glide's verbs, not your shell, so `ls` and `vim`
are not part of the deal.

Two modes, so one key can mean two things without a pile of modifiers. The
prompt types; blocks navigate.

| Key | Prompt mode | Block mode |
|---|---|---|
| `enter` | run the line | fold or unfold the block |
| `esc` | enter block mode | back to the prompt |
| `^k` | block mode, newest block | — |
| `j` `k` `↑` `↓` | — | move focus |
| `y` | — | copy the block |
| `r` | — | re-run the block |
| `^u` `^d` | scroll | scroll |
| `^c` | quit | quit |

`help`, `clear`, and `exit` are handled by the console itself. Every other line
is parsed as a verb first, and only falls through to the same `route_one_shot`
router `glide -p` uses when it isn't one. That ordering matters: the router
matches on keywords, so `friction log needs staging access` would otherwise
read as a digest.

| Typed in the console | Runs |
|---|---|
| `doctor` | the health checks |
| `index` / `index show` | graph counts |
| `index build` | full rebuild (the only spelling that wipes the graph) |
| `config` / `config list` | the effective TOML |
| `config get <key>` / `config set <key> <value>` | one dotted key |
| `config paths` | the layer precedence |
| `models` | providers and whether the key is set |
| `tools` | repomix detection |
| `friction log <what happened>` | log an event, `--severity` and `--category` accepted |
| `friction digest [days]` | roll it up |
| `who owns <path>`, `plan for <person>`, `i need access to <tool>` | the graph verbs |

Questions and commands share one prompt, which is the part worth borrowing from
Warp. `who owns src/app.tsx` and `doctor` are typed the same way.

`glide --safe` carries into the console: `index build`, `config set`, and
`friction log` answer with a notice saying why instead of writing.
`tools install` is never run from the console at all. It shells out to a global
npm install, and doing that with the event loop frozen and `^c` unreachable is
worse than handing it back to the shell, so the block tells you the command.

The loop is synchronous. A slow verb puts a block on screen showing `running…`
before the work starts, so you can see what it is doing, but the screen is still
frozen until it returns. `doctor` and `tools` take about a second because
repomix detection shells out to `npx`; `index build` takes longer.

A repo with no `.glide/` still opens. The first block tells you to run
`glide init` rather than the console refusing to start.

## Architecture

See `crates/`:

- `glide-common` — config (incl. dotted get/set), errors, paths, stream enum
- `glide-graph`  — SQLite schema + indexer (CODEOWNERS, git history, team docs)
- `glide-core`   — verb implementations (returns data; CLI and console format)
- `glide-llm`    — optional LLM client (feature-gated, off by default)
- `glide-tui`    — the block console: state, key map, renderers
- `glide-cli`    — clap parser, formatters, `main()`

## Config

Layered TOML, merge order lowest → highest:

1. compiled defaults
2. `~/.glide/config.toml`
3. `<repo>/.glide/glide.toml`
4. `<repo>/.glide/config.local.toml`
5. `GLIDE_*` env vars

## Status

Draft scaffold. v1 ships `init`, `index build`, `who-owns`, `doctor`, `config`, `completion`, and the `console` fully working; `plan`, `request`, `friction`, and `models` return draft or stub responses so the surface area is locked. The console renders whatever those verbs return, so they get better without the console changing.

Not in the console yet: interactive form widgets inside a request block (it renders the draft as text, submitting is still `glide request`), LLM routing, and block search.
