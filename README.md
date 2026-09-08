# glide

Keep your priorities in view while you work in the terminal.

Glide is one binary that sits under whatever terminal you already use (Warp, iTerm, tmux) and keeps today's plan where you type.
Your priorities live in a Markdown note you own, in a folder you already sync.
The agent you run in the terminal (Claude Code, Warp AI, any MCP client) gets the same six verbs you do, so the status writes itself while the work happens instead of on Friday from a blank page.

```
▶ ship portal tests · focus 1/3 · 4 open · 2 logged
```

That line is `glide focus`. It goes in your tmux status bar or your prompt, and it changes as you and your agent work.

Try it without installing anything at [tryglide.net/try](https://tryglide.net/try): the same engine, running in your browser.

![glide in a terminal: focus set, the agent logs, done, the note, and what an agent sees at session start](demo/demo.gif)

The recording is scripted (`demo/glide.tape`, rendered with [VHS](https://github.com/charmbracelet/vhs)) against a throwaway vault, so it is the real binary every time.

## Install

```sh
brew install matthewvilaysack/glide/glide
```

or

```sh
curl -fsSL https://raw.githubusercontent.com/matthewvilaysack/glide/main/install.sh | sh
```

Prebuilt for Apple Silicon, Intel Mac, and Linux x86_64 and arm64, with checksums the installer verifies.
From source: `cargo install --git https://github.com/matthewvilaysack/glide glide-cli`.

## Two minutes to set up

1. Tell glide where your notes are. Any folder of Markdown works; an Obsidian vault on iCloud is the case it was built for.

   ```sh
   mkdir -p ~/.glide
   printf '[vault]\npath = "~/path/to/your/notes"\n' > ~/.glide/config.toml
   glide focus
   ```

2. Give your agent the workflow. One command for Claude Code:

   ```sh
   glide setup claude --global
   ```

   That installs a SessionStart hook running `glide prime --hook-json`, which puts today's focus and the six verbs into every session (and again after every compaction) for a few hundred tokens, no MCP schema overhead. `glide setup claude --check` and `--remove` do what they say. Warp: `glide setup warp` prints the rule to paste. Anything else: `glide onboard` prints the paragraph for its instructions file. The MCP server (`glide mcp serve`) is there for clients without a shell, Claude Desktop for one.

3. Keep it on screen. In `~/.tmux.conf`:

   ```
   set -g status-right '#(glide focus 2>/dev/null) '
   ```

The docs are at [tryglide.net/docs](https://tryglide.net/docs/): a quick start, a worked first day with an agent, one how-to per terminal and agent, and the reference for every verb and the daily note contract.

## Day to day

```sh
glide focus set ship portal tests      # what I'm on now
glide focus done portal                 # finished
glide focus capture idea for the demo   # into Notes
glide focus log fixed the flaky build   # into Record, with a timestamp
glide focus today                       # the whole note
glide                                   # the block console, with the focus strip in its top bar
glide prime                             # what an agent sees at session start
```

Say the same things to your agent instead and it calls the same verbs.
Its server instructions ask it to read your focus at the start of a session and log what it did after every task, so the Record section fills in behind you.

## What it touches, and what it never does

Glide reads and edits exactly four `##` sections of today's note: Focus, Tasks, Record, Notes.
Everything else in the file survives byte for byte, and a missing note is created with just those sections.
No daemon, no account, no server, no lock file.
Nothing leaves your machine; the folder sync you already have is the cloud.
`--safe` turns every write into a refusal that says so.
The contract is one page: [`docs/vault-contract.md`](docs/vault-contract.md).

## Where this is going

The same notes compound.
A person who has kept their own priorities and record for a month has, without trying, written most of what the next hire on their team needs on day one: who owns what, what got stuck and how it got unstuck, which commands finally worked.
The team tier turns that record into onboarding, and the on-device knowledge graph the binary already carries (ownership, permissions, friction) is where it lands.
The product requirements are in [`docs/requirements.md`](docs/requirements.md). The pitch, as it stands, is at [tryglide.net/pitch.html](https://tryglide.net/pitch.html), and how a release happens is at [tryglide.net/how-it-ships.html](https://tryglide.net/how-it-ships.html).

## Building and releasing

Rust workspace under [`cli/`](cli/), `cargo build --release -p glide-cli`.
CI runs formatting, clippy with warnings as errors, and the tests on Linux and macOS.
Releases are tag-driven with a patch lane for shipped lines; the runbook is [`docs/releasing.md`](docs/releasing.md).

MIT or Apache-2.0, your choice.
