# glide

Your team already decided this. Your agent doesn't know.

Every tool that gives a coding agent memory injects what *happened*: session transcripts, compressed output, retrieved files.
None of them inject what was *decided*, because none of them have a place where a person writes a decision down.
So the team rules out Mongo on a Tuesday, and on Thursday an agent proposes it again, to somebody else, and that person spends turns saying no.

```sh
glide decide --team --against mongo, we are on postgres and the ops story is settled
git commit -am "settle the datastore"
```

That is the whole mechanism.
The decision is a line in `DECISIONS.md` in your repository, so git carries it: every teammate's agent opens its next session already knowing, with no server, no account and nothing to sync.
It costs the same for two people or two hundred, because the marginal teammate is a clone.

The file format is [specified separately](SPEC.md) and published under CC0.
It is not owned by this tool, and a decision is only worth writing if whatever agent the next person runs can read it too.

Glide is also the thing that keeps today's plan where you type, which is where the decisions come from.
One binary under whatever terminal you already use (Warp, iTerm, tmux), writing to a Markdown note you own in a folder you already sync.

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

   That installs a SessionStart hook running `glide prime --hook-json`, which puts today's focus and the verbs into every session (and again after every compaction) for a few hundred tokens, no MCP schema overhead. `glide setup claude --check` and `--remove` do what they say. Warp: `glide setup warp` prints the rule to paste. Anything else: `glide onboard` prints the paragraph for its instructions file. The MCP server (`glide mcp serve`) is there for clients without a shell, Claude Desktop for one.

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
glide focus clear                       # what is still open today; --confirm removes it
glide                                   # the block console, with the focus strip in its top bar
glide prime                             # what an agent sees at session start
```

`clear` reports before it removes anything, and takes `--confirm` to actually do it.
Finished items stay, because they are the day's record of what happened and the thing being cleared is what did not.
A bullet with no checkbox stays too: that is prose you wrote in a list rather than a task you left open.

### What is already settled

The expensive thing an agent does is propose something you ruled out last week, hear no, and pick again.
That is turns, and a turn re-reads the whole conversation.

```sh
glide decide postgres over sqlite, the replication story matters
glide decide --against mongo, schema churn already bit us
glide decisions                         # everything settled so far
```

Both kinds ride along in `glide prime`, so a session opens already knowing them.
One decision costs about twenty tokens to carry and removes the turns that would have re-litigated it.

Every other tool in this category injects what happened: session memory, compressed command output, retrieved facts.
None of them inject what you decided, because none of them have a place where you write a decision down.

### Settling it for everyone

A decision that binds the team belongs to the repository rather than to you.

```sh
glide decide --team --against mongo, we are on postgres and the ops story is settled
git add DECISIONS.md && git commit -m "settle the datastore"
```

That writes `<repo>/DECISIONS.md`, so git carries it.
One person rules something out, commits, and every teammate's agent knows at their next pull.
There is no server, no account and nothing to sync, because the team already has a thing that distributes files to everyone working on the repo, and this is a file.
It is also why it costs the same whether there are two of you or two hundred.

`glide decisions` reads the team's first and then your own, and `glide prime` does the same, so a decision made for everyone is the one an agent sees first.
Settle something personally and then again for the team and it is listed once.
Outside a repository `--team` is an error, while reading degrades quietly to your own decisions, because a session has to start whether or not there is a repo.

The file is append-only prose, so two people adding a decision on two branches conflict at the last line and the resolution is to keep both.
That is the whole merge story and it does not need tooling.

The file is deliberately not named after this tool, and the format is [written down](SPEC.md) so other tools can read and write it.
The parser is about thirty lines.
If you implement it somewhere else, open an issue and it gets listed in the spec: a format with one implementation is a file format, and the version worth anyone's time is the one with three.

### Work that outlives today

The daily note is thrown away, which is what makes it useful, and it is also why the week's work needs somewhere else to sit.

```sh
glide sprint start LEG test pipeline    # begin one; any sprint already open is closed
glide sprint add close the fold         # put work in it
glide sprint                            # what is left, and how far along
glide sprint pull fold                  # take an item and make it today's focus
glide sprint done fold                  # check it off in the sprint
glide sprint list                       # every sprint, active first
```

`pull` is the one that matters: it is the join between the list and the day, without which a sprint is a second list to keep in sync by hand.

A sprint is a Markdown note in the same vault, under `Sprints/`, so it opens in Obsidian, diffs in git, and can be edited by hand without asking this tool.
At most one is active at a time, which is a limit rather than a missing feature: a tool that let three sprints be current would hand the choice back to you at the moment it is meant to make it.

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
