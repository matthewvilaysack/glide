# Hosted vault MCP: your Obsidian vault in any AI client

Glide's MCP server today speaks stdio, so only clients on the same machine can use it, and it only exposes the daily-note focus tools.
This design turns it into a remote MCP server that claude.ai, the Claude mobile apps, ChatGPT, Cursor, Codex, Gemini and Claude Code can all reach, with read access to the whole vault and a small set of safe writes.
The vault stays on the machine it lives on; a connected app gets only the notes it asks for.
A tunnel gives that machine a public HTTPS address, and OAuth decides who gets through it.

Requirement: R011.

## Architecture

```
AI client (claude.ai, ChatGPT, Cursor, ...)
   │  HTTPS, OAuth bearer token
   ▼
Tailscale Funnel  https://<machine>.<tailnet>.ts.net
   │  forwards to localhost
   ▼
glide mcp serve --http 127.0.0.1:8787
   ├── /mcp                          streamable HTTP MCP endpoint (rmcp)
   ├── /.well-known/oauth-*          OAuth metadata
   ├── /oauth/{register,authorize,token,callback}
   │       └── identity delegated to GitHub, login allowlist
   └── vault tools ──► glide-vault ──► the Obsidian vault on disk
```

### Components

**HTTP transport.**
`glide mcp serve --http <addr>` mounts rmcp's streamable HTTP server (feature `transport-streamable-http-server`) on an axum router.
It binds to 127.0.0.1 only; the tunnel is the one way in from outside.
The stdio mode stays as it is, so the existing Claude Code setup keeps working.

**Vault tools.**
The tool set grows from the daily-note tools to the whole vault, split by risk.

| Tool | Kind | What it does |
|---|---|---|
| `search_notes` | read | Full-text search over note bodies and titles, with an optional folder filter, returning paths and matching lines |
| `read_note` | read | Returns one note's Markdown by vault-relative path |
| `list_notes` | read | Lists notes in a folder, newest first |
| `backlinks` | read | Lists notes that link to a given note via `[[...]]` |
| `today` | read | Today's daily note (existing) |
| `capture` | write | Appends a line to today's daily note (existing) |
| `focus_set`, `focus_done` | write | Existing focus verbs |

There is no tool that edits or deletes an arbitrary note.
Every write lands in today's daily note through the same `glide-vault` writers the CLI uses.
The existing `log` tool is not exposed over HTTP.

**Path guard.**
Every path argument is resolved against the vault root, canonicalized, and rejected if it escapes the root, follows a symlink out, or points into a hidden folder (`.obsidian`, `.trash`, `.git`).
Folders listed in `[mcp.http] exclude` are invisible to every tool: they are skipped by search and listing, and `read_note` on them returns "not found" rather than "forbidden", so the folder's existence doesn't leak either.
Only `.md` files are served.

**OAuth.**
claude.ai and ChatGPT require OAuth 2.1 on remote servers, with dynamic client registration and PKCE.
Glide acts as the authorization server and hands identity to GitHub:

1. The client reads `/.well-known/oauth-protected-resource` and `/.well-known/oauth-authorization-server`.
2. It registers itself at `/oauth/register`.
3. `/oauth/authorize` redirects the user to GitHub sign-in.
4. `/oauth/callback` checks the GitHub login against `[mcp.http] allow`, then issues a code.
5. `/oauth/token` swaps the code (with PKCE) for a short-lived access token and a refresh token.

Tokens are random, stored only as SHA-256 hashes in `~/.glide/mcp-auth.db` (SQLite), and expire (1 hour access, 30 days refresh).
The GitHub OAuth App's client secret lives in the macOS Keychain, not in the config file.
Any request to `/mcp` without a valid token gets 401 with the `WWW-Authenticate` header clients need to start sign-in.

**Tunnel and lifecycle.**
`glide mcp up` is the one command a person runs.
It starts the HTTP server, runs `tailscale funnel --bg 8787`, prints the public URL, and prints the exact setup step for each client.
`glide mcp down` stops both.
`glide mcp status` shows the URL, whether Funnel is on, and the last few requests by GitHub login and tool name.

### Config

```toml
[mcp.http]
addr = "127.0.0.1:8787"
allow = ["your-github-login"]         # GitHub logins allowed through
exclude = ["Personal", "Finance", "Health & Fitness"]
github_client_id = "..."              # secret is in the Keychain
```

## Risks

- **The laptop is the server.**
  When the lid is closed the connector is offline, and clients show a connection error.
  Moving the relay to the Mac mini later is a config change, not a redesign.
- **Private notes.**
  The exclude list is the line between what agents can see and what they can't, so it ships with a conservative default and `glide mcp status` prints it on every start.
- **Prompt injection through notes.**
  A note can contain instructions aimed at whatever model reads it.
  Writes are limited to appending to today's note, so the worst a hijacked agent can do is add lines there.
- **Public URL.**
  Funnel exposes the port to the whole internet.
  Everything except the OAuth metadata and the sign-in routes requires a token, and the token endpoint is rate-limited.
- **Callback URL drift.**
  The GitHub OAuth App's callback must match the Funnel hostname, so a renamed machine or tailnet breaks sign-in until the app is updated.
  `glide mcp up` checks this and says so.

## Phases

1. **Local HTTP, no auth.**
   `glide mcp serve --http` on 127.0.0.1 with the new read tools and the path guard, connected to Claude Code with `claude mcp add --transport http`.
2. **OAuth and Funnel.**
   GitHub sign-in, token storage, `glide mcp up`, then connecting from claude.ai, the Claude phone app, ChatGPT and Cursor.
3. **Publish.**
   Per-client setup guides on tryglide.net and in the docs site, the README section, and listings on the official MCP registry and Smithery.
   Anything public goes out only after the maintainer approves the drafts.

## Test plan

- Unit tests for the path guard: `../`, absolute paths, symlinks out of the vault, `.obsidian/`, and excluded folders are all rejected.
- Unit tests for the OAuth flow: PKCE mismatch, expired code, reused code, expired token, and a GitHub login not on the allowlist are all refused.
- An integration test that runs `glide mcp serve --http` against a temp vault (`GLIDE_VAULT_PATH`) and drives it with an MCP client over HTTP: list tools, search, read, capture.
- End to end by hand, phase 2: connect claude.ai on the web and on the phone, ask a question that needs a note, capture a line, and confirm it shows up in today's note on disk.

## Negative checks

- With no token, `curl` to `/mcp` returns 401, and the response leaks no vault content.
- Signing in with a second GitHub account that is not on the allowlist ends at an error page, and no token is issued.
- Asking any client to read a note under `Personal/` returns not found, and `search_notes` for a phrase that only appears there returns nothing.
- No tool call can change a note other than today's daily note: after a scripted session that tries, `git diff` on a temp vault shows only today's note changed.
