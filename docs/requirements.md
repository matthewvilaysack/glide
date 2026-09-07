# Glide — MVP requirements

Living requirements for the first design-partner version of Glide. This is written for two audiences: prospects who want to understand what we are promising, and us, so we do not accidentally build a vague "AI onboarding" toy.

Statuses: `planned` -> `in-flight` -> `demo-ready` -> `shipped` -> `retired`.

## Product promise

Glide helps a new engineering hire become useful faster by answering repo- and company-specific questions, filing access requests, identifying the right humans to ask, and giving the manager a weekly read on where onboarding is breaking.

The first version is concierge-backed. We can manually configure and operate parts of the system during Phase 0, but the customer-facing artifacts must look like the real product: chat answers, access requests, manager digest, and repo-committed skills.

## Scope

### In scope for Phase 0

- One design-partner company.
- One new hire or intern.
- One primary repo or monorepo slice.
- One Slack channel or Telegram bot surface.
- Read-only GitHub access.
- Read-only docs access, if granted.
- Read-only issue tracker access, if granted.
- A weekly manager digest generated from the hire's questions, blockers, and first-code activity.
- Customer-specific Claude Code skills committed into the customer's repo.

### Out of scope for Phase 0

- Workspace-wide Slack ingestion.
- DMs, HR systems, payroll, benefits, or personnel review data.
- Production-write permissions.
- Autonomous admin actions.
- Multi-tenant hosted graph backend.
- SSO/RBAC beyond the chosen design-partner access controls.
- SOC2 claims.

## R001 — Deliver repo-committed Claude Code skills

**Status:** demo-ready

Glide must deliver a `.claude/skills/` bundle tailored to the customer's tools, repos, deploy flows, ownership model, and permissions process.

### Acceptance criteria

- At least three skills are customized for the customer: `repo-onboarding`, `who-owns`, and `permissions-request`.
- Each skill includes customer-specific config or clear TODO markers from the scoping questionnaire.
- Skills are committed under the customer's GitHub org and owned by the customer.
- Skills do not require Glide-hosted infrastructure to be readable or useful.

## R002 — Generate a personalized day-1 onboarding plan

**Status:** demo-ready

Given a hire name, role, team, and start date, Glide must produce a first-week plan grounded in the customer's repo, docs, tickets, and permission map.

### Acceptance criteria

- The plan names the hire, team, role, start date, and first-week goals.
- It lists required access with expected owner or approval path.
- It lists 3 candidate first tickets or says why no safe candidate exists.
- It lists who to ask for setup, domain, access, and review questions.
- Every file, ticket, person, and channel is either grounded in provided sources or explicitly marked as missing.

## R003 — Answer ownership questions

**Status:** demo-ready

Glide must answer "who owns X" for services, files, features, and internal tools.

### Acceptance criteria

- The answer leads with the most useful person or channel.
- It shows the evidence source: CODEOWNERS, git history, docs, Slack channel, on-call schedule, or issue tracker.
- If there is no confident owner, Glide says so and routes the hire to their manager or team channel.
- Urgent questions prioritize on-call or escalation path before historical ownership.

## R004 — Help file permission requests

**Status:** demo-ready

Glide must turn "I need access to X" into a concrete request using the customer's permission map.

### Acceptance criteria

- The response identifies the tool, approver, request channel, required justification, and expected ETA.
- Glide asks before submitting anything.
- Production-write or admin-tier requests are not submitted automatically.
- If the requested tool is not in the permission map, Glide asks one clarifying question or routes to the manager.
- The demo path can show a filed request or a pre-filled request draft.

## R005 — Detect and summarize onboarding friction

**Status:** planned

Glide must turn the hire's questions, stalled requests, and first-code activity into a manager-readable weekly digest.

### Acceptance criteria

- The digest includes what the hire accomplished, where they got stuck, and what needs manager action.
- The digest identifies repeated questions or blockers.
- The digest distinguishes access blockers from code blockers and team-context blockers.
- The digest does not include private chat content that is unrelated to onboarding.
- The digest can be generated manually in Phase 0 from captured events.

## R006 — Protect customer data in Phase 0

**Status:** demo-ready

Glide must be able to explain and follow a minimal, concrete data-handling policy for concierge engagements.

### Acceptance criteria

- All granted sources are read-only unless a customer explicitly approves request creation.
- Slack scope is limited to invited channels.
- Secrets and environment files are skipped during indexing.
- Phase 0 customer data stays on the operator machine unless the customer explicitly approves another storage path.
- The customer can request deletion and receive confirmation within 24 hours.

## R007 — Show the product without a live customer integration

**Status:** demo-ready

Glide must have a credible mocked packet that shows the real shape of the product before the first design partner grants access.

### Acceptance criteria

- The packet includes an onboarding plan, chat transcript, manager digest, and skill examples.
- The fake company, names, tools, and tickets are clearly marked as fake.
- The demo script can be run in 15 minutes.
- The artifacts map cleanly to the requirements in this document.

## R008 — Define design-partner success criteria

**Status:** planned

Before starting a Phase 0 engagement, Glide must agree with the customer on what "worked" means.

### Acceptance criteria

- The customer names the hire, manager, repo, and start date.
- The customer names 1-2 measurable outcomes.
- The scoping questionnaire captures the top five week-1 permissions.
- The engagement has a weekly check-in slot.
- The customer explicitly lists off-limits repos, channels, docs, or data types.

## R009: Be the thin layer for CLI agent management

**Status:** planned

Engineers increasingly ramp by driving CLI coding agents, and a new hire's real working set is a handful of agent sessions: Claude Code in a tmux pane, another agent on a second worktree, a long-running index or build.
The glide console must manage those sessions rather than making the hire switch surfaces to find them.

Thin layer means exactly that.
Glide lists, launches, and routes to sessions, and hands off to tmux for anything interactive.
It never embeds a terminal, wraps the shell, or grows a daemon.
The console already refuses to be a terminal emulator; this requirement is the productive version of that refusal.

### Acceptance criteria

- The console lists running agent sessions as blocks: name, what it is running, state (working, waiting for input, idle), and age. tmux sessions and Claude Code sessions are the first two sources.
- A session can be started from the prompt into tmux by name.
- Attach quits the console and execs `tmux attach`, rather than pretending to embed the session.
- A prompt can be sent to a named session without attaching.
- No tmux on PATH, or no sessions running, renders as a block that says so and what to do, never an error.
- Glide stays a single binary with no daemon. Session state is read from tmux and the agents' own on-disk state at ask time, nothing is polled or mirrored.
- Safe mode refuses launch and send, and still allows listing.

## R010: Keep the person's priorities in view while they work

**Status:** shipped (focus line and MCP tools)

The people glide is for spend the day in a terminal driving agents, and the thing that slips is not the code, it is the plan: what today was supposed to be about, what got done, what came up.
Glide keeps that in a Markdown daily note the person already owns (an Obsidian vault on iCloud is the reference case) and puts it on screen where they type.

### Acceptance criteria

- `glide focus` prints a one-line strip (current focus, focus done-over-total, open tasks, entries logged) from today's note, suitable for a tmux status bar, and `--json` returns the same as data.
- `focus set`, `done`, `capture`, and `log` edit only the four contract sections of the note (`docs/vault-contract.md`); everything else in the file survives byte for byte, and a missing note is created with just those sections.
- `glide mcp serve` offers the same operations as MCP tools over stdio, with server instructions that ask the agent to read the focus at session start and log after each task.
- Works outside a git repo; the vault is the person's, not the repository's.
- `--safe` refuses every vault write and says why. An unset vault path says how to set it.
- No daemon, no network, no lock file: every operation reads the note fresh and writes it back whole.

## Demo traceability

| Requirement | Show it with |
| --- | --- |
| R001 | `.claude/skills/repo-onboarding/`, `.claude/skills/who-owns/`, `.claude/skills/permissions-request/` |
| R002 | `docs/day-1-packet/onboarding-priya.md` |
| R003 | `docs/day-1-packet/sample-chat-transcript.md` |
| R004 | `docs/day-1-packet/sample-chat-transcript.md` and `.claude/skills/permissions-request/SKILL.md` |
| R005 | `docs/day-1-packet/sample-manager-digest.md` |
| R006 | `docs/privacy.md` |
| R007 | `docs/showcase.md` |
| R008 | `docs/scoping-questionnaire.md` |
| R009 | `docs/console-prd.md` and the console section of `cli/README.md` |
| R010 | `docs/vault-contract.md`, `docs/attach-terminals.md`, `glide focus` against a temp vault |

## Phase 0 exit bar

Glide is ready to ask for an LOI when all of these are true:

- The hire gets a useful answer to at least 80% of onboarding questions without needing a senior engineer first.
- The first-week access bundle is requested on day one.
- The manager can name at least one blocker earlier than they would have without Glide.
- The customer says the skill bundle is specific enough to keep in their repo.
- We can explain exactly what data we read, where it lives, and how to delete it.
