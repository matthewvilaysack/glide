# Getting Started

How to stand up the first Glide agent against a single repo. Pre-MVP — assume you're the only operator and the codebase you're pointing at is your own.

> **Starting frame:** Glide's agent runtime is built on top of **Hermes Agent** (open-source, MIT-licensed, sibling to Codex / OpenClaw). We're not married to it forever — but it's the fastest way to ship V1 because the skills system, the messaging adapters (Telegram/Slack), and the cron scheduler are already there. We layer Glide's two surfaces on top: the **intern chat** (Telegram/Slack adapter, talks to one person) and the **manager dashboard** (web UI, this React app).

## The two surfaces

| Surface | Who | Where | What it does |
| --- | --- | --- | --- |
| Intern chat | New hire | Telegram / Slack DM | Answers grounded questions about the codebase, files perm tickets, points at the right human |
| Manager view | Hiring manager | This Vite app | Shows where each ramp is stalling — three days stuck on access, same question asked four times across channels |

The Hermes agent is the engine behind both. The intern surface is the chat adapter; the manager surface reads the same agent's event log and renders it.

## What you need before you start

- A VPS (we're starting on Hostinger KVM 2 — 2 vCPU / 8 GB RAM / 100 GB NVMe). Pick a region close to you.
- A GitHub personal access token (classic, repo scope, 7-day rotation) for the repo Glide will read.
- A Telegram account (for the chat adapter) — get one bot token from `@BotFather` per agent.
- An LLM credential. You can drive it with a Codex / ChatGPT subscription for dev. **Production needs an API key** (Anthropic for Claude, OpenAI for GPT). Subscription auth doesn't survive the web dashboard's task runner.
- Your own user ID from Telegram (`@userinfobot`) for the allowlist.

## Treat the agent like a new employee

Every agent gets its **own** credentials. Don't hand it your personal Gmail, your main GitHub PAT, or the team's prod API key. The pattern:

- Dedicated Google Workspace user (e.g. `glide-agent@yourco.com`) — so when it sends an email it's clearly the bot
- Dedicated GitHub PAT scoped to the *single repo* it's onboarding people into
- Dedicated LLM key with a low monthly cap — autonomous agents hallucinate; the cap is your blast-radius limit
- Allowlist of user IDs in the chat adapter — only the operator + the intern can talk to that agent instance

A loose autonomous agent with the team's prod keys is a credential breach waiting to happen. The cap and the allowlist are not nice-to-haves; they are the bedrock.

## Install path

The numbered steps below are the **install** sequence. Each step has a "why" — skip the step only if you understand what you're trading.

### 1. Spin the VPS

- Ubuntu 24.04 LTS
- KVM 2 plan (or local equivalent — 8 GB RAM is the floor, the agent + an embedding index + the web UI hold it in memory)
- Turn on the free malware scanner
- Save the root password into your local password store (`pass` or 1Password) — *not* in a note file in the repo

> Skip daily provider backups for V1. We mirror state to a private GitHub repo via cron (step 6), which is the backup.

### 2. Install Docker Manager on the host

We run the agent in a container, not directly on the host, for one reason: **multi-tenant later**. Each design-partner company eventually gets their own container — same image, separate creds, separate skill set, separate volume. If you install Hermes straight onto the host, you can't add a second tenant without reprovisioning.

```sh
# on the VPS, via the host's web terminal
curl -fsSL https://get.docker.com | sh
```

(If you're on Hostinger, "Docker Manager" is the one-click flavour of the same thing — same outcome.)

### 3. Deploy the Hermes Agent container

In Docker Manager's one-click catalogue, search for `hermes-agent` (not `hermes-web-ui`, not `hermes-workspace` — the **agent** is what we want; the others are surface adapters that ship with it).

Set the admin username + admin password **before** you click deploy. Save both into `.env` immediately. There is no "I forgot my password" flow inside the container.

### 4. Wire the LLM

The first time you `open` the container's terminal, Hermes runs `quick setup`. Pick the LLM there:

- **Dev / personal:** OpenAI Codex via your ChatGPT subscription. Free with the $20/month plan. Note: device-code auth requires you to flip "device code authorization for Codex" on inside your ChatGPT account → Settings → Security, *before* running quick setup. If you skip this, the device code shows up greyed-out red and the install stalls.
- **Production:** Anthropic API key (Claude 4 Sonnet for the cheap path; Claude 4.7 Opus for the manager-dashboard reasoning). The web UI's task runner does not accept subscription auth — only API keys.

For Glide the default is Anthropic; we already pay for it and the prompt-caching story is better for repo-grounded Q&A.

### 5. Wire the chat adapter (Telegram for V1)

- In Telegram, talk to `@BotFather` → `/newbot` → name it after the agent (e.g. `glide-acme-bot` for Acme's intern). Save the bot token.
- Run `quick setup` step "messaging" → space-select Telegram → paste the token.
- **Immediately** add the allowlist: only your user ID + the intern's user ID. Without this, the bot accepts messages from anyone who guesses the username. Get the user ID from `@userinfobot` in Telegram.

> The transcript's source video forgot the allowlist on first install and had to patch it post-hoc. Don't do that — add it before you send the first test message.

### 6. Wire GitHub (for repo access + state backup)

Two separate uses of GitHub here, easy to conflate:

1. **Read access** — the PAT Glide uses to clone and re-index the repo it's onboarding people into. Scope: `repo` (read), single-repo if your GitHub plan allows fine-grained tokens.
2. **State backup** — a separate private repo where Glide commits its own config + skill state once a day via cron. Scope: `repo` on that one backup repo only.

Both tokens go into the container's `.env` (see "Secrets" below). Never the same token for both purposes — the read token rotates frequently, the backup token doesn't.

### 7. Schedule the daily commit

Inside the agent: ask it to set up a cron job that commits its own state to the backup repo daily. Time of day doesn't matter; pick an off-peak hour. The point is that if the VPS dies, you can `docker run` the image elsewhere, `git clone` the backup repo, and you're back.

### 8. Bring up the manager dashboard

This is the React app in `src/`. For V1 it's still mostly the landing page (`App.tsx`) — the manager view is the next build step.

```sh
npm install
npm run dev
```

The dashboard will eventually connect to the agent over its HTTP API (Hermes exposes one on `:8080` by default). For local dev, point it at `http://localhost:8080`; for prod, expose the agent behind a reverse proxy with auth in front.

## Secrets

Everything lives in `.env` on the VPS, inside the container. Never check this file in. The `.gitignore` already excludes it; verify before your first push.

```sh
# .env (illustrative — copy from .env.example, never commit)
HERMES_ADMIN_USER=glide-operator
HERMES_ADMIN_PASS=<from password store>
TELEGRAM_BOT_TOKEN=<from BotFather>
TELEGRAM_ALLOWLIST=<your-user-id>,<intern-user-id>
ANTHROPIC_API_KEY=<scoped to this agent, monthly cap set>
GITHUB_READ_PAT=<repo:read on the codebase being onboarded>
GITHUB_BACKUP_PAT=<repo on the backup repo only>
GLIDE_BACKUP_REPO=yourorg/glide-state-acme
```

The Hermes web terminal edits this file with `nano`. The flow is: paste, `Ctrl+O`, Enter (write), `Ctrl+X` (exit). Don't screenshot the terminal mid-edit — the screenshot key combo on macOS knocks the SSH session out and you'll restart the install.

## First conversation

Once steps 1-7 are done, message your Telegram bot:

> hello, are you there?

You should get a Hermes-style reply. Then onboard the agent to itself — give it the role:

> You're Glide, the onboarding agent for a new engineer joining `<repo-name>`. Your job is to (a) answer questions grounded in the code, the CODEOWNERS, and the team's docs, (b) file permission tickets when the engineer hits a wall, and (c) flag to the manager when the engineer has been stuck on the same blocker for more than a day. Begin by indexing the repo at `<github-url>`. When you're done, post a short ramp plan for day one.

This is the **prompt** that turns a generic Hermes agent into a Glide agent. It will eventually live in `agent/system-prompt.md` once we extract it; for now, paste it once and let the agent persist it via its own memory skill.

## What this stack does *not* do yet

- **Slack adapter** — Telegram only for V1. Slack is the production target (every eng team is already in Slack); Telegram is the dev surface because there's no app-review gate.
- **Manager view** — the React app is the landing page. The actual "where is the ramp breaking" UI doesn't exist yet. Spec lives in Obsidian (`Projects/Onboarding as a Service - MVP Spec.md`).
- **Multi-tenant** — one container = one tenant. The architecture supports adding more (step 2 was the reason), but we haven't wired the per-tenant routing.
- **SSO / RBAC** — the manager dashboard is single-operator. Real customers need auth in front of it before any design partner sees it.

## Troubleshooting (the gotchas from the install path)

- **Codex device code greyed out / red** → ChatGPT account Settings → Security → enable "device code authorization for Codex" → restart `quick setup`.
- **`quick setup` window keyboard-screenshot kicks you out** → delete the container in Docker Manager and start step 3 again. The setup is idempotent; you lose maybe two minutes.
- **Telegram bot replies to strangers** → you skipped the allowlist. Stop the bot, edit `TELEGRAM_ALLOWLIST` in `.env`, restart the container.
- **`nano` edits don't save** → `Ctrl+O` writes, **then** `Ctrl+X` exits. If you `Ctrl+X` first, nano prompts you to save — answer `Y`, then it works.
- **Web dashboard 401s the API key** → you're trying to use Codex subscription auth against the task runner. Swap to an API key.

## Where this goes next

- **Architecture deep-dive:** see [architecture.md](architecture.md) for the diagrams + the "why Hermes, why not OpenClaw" call.
- **Manager view spec:** Obsidian, `Projects/Onboarding as a Service - MVP Spec.md`.
- **Visual direction for the dashboard:** [visual-direction.md](visual-direction.md) and [inspiration.md](inspiration.md).
