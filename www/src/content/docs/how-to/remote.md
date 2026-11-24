---
title: Use your vault from any AI app
description: Put your Obsidian vault behind one URL that claude.ai, ChatGPT, Cursor, Codex and Claude Code can all use.
---

`glide mcp up` turns the machine your vault lives on into a remote MCP server.
Your vault stays on that machine; an app gets only the notes it asks for, and those go to its AI provider.
Tailscale Funnel gives it a public HTTPS address, and GitHub sign-in decides who gets through.

What an AI app can do once it's connected:

- search, list and read any note outside the folders you hide
- find the notes that link to a note
- read today's daily note, capture a line into it, and set or check off your focus

It can't edit, move or delete any other note.

## Before you start

You need three things, once.

**Tailscale with Funnel.**
Install Tailscale, run `tailscale up`, and in the admin console turn on MagicDNS and HTTPS certificates, then allow Funnel for this machine.

**A GitHub OAuth App.**
At github.com/settings/developers, create an OAuth App.
Set the callback URL to `https://<machine>.<tailnet>.ts.net/oauth/callback`; `glide mcp up` prints the exact value.
Copy the client id, then store the client secret in the Keychain:

```sh
security add-generic-password -s glide-mcp-github -a <client-id> -w
```

It prompts for the secret, so it never ends up in your shell history.

**Config.**
In `~/.glide/config.toml`:

```toml
[mcp.http]
allow = ["your-github-login"]
github_client_id = "<client-id>"
exclude = ["Personal", "Finance", "Health & Fitness"]
```

`allow` is the list of GitHub logins that can sign in.
`exclude` is the list of folders no app can see; a hidden note reads as "not found", even to search.

## Start it

```sh
glide mcp up
```

It prints the URL to connect to and the step for each app below.
Ctrl-C stops the server and turns Funnel off.
`glide mcp status` shows the URL, what's hidden, and the last requests by login and tool.

## Connect an app

Every app gets the same URL: `https://<machine>.<tailnet>.ts.net/mcp`.
The first time, the app opens a GitHub sign-in page.

**claude.ai, Claude Desktop and the Claude phone apps.**
Settings, Connectors, Add custom connector, then paste the URL.
Adding it once on claude.ai makes it available in the desktop and phone apps too.

**ChatGPT.**
Turn on developer mode under Settings, Apps, Advanced settings, then create an app with the URL.

**Cursor.**
Add it to `~/.cursor/mcp.json`:

```json
{ "mcpServers": { "glide": { "url": "https://<machine>.<tailnet>.ts.net/mcp" } } }
```

**Claude Code.**

```sh
claude mcp add --transport http glide https://<machine>.<tailnet>.ts.net/mcp
```

**Codex.**

```sh
codex mcp add glide --url https://<machine>.<tailnet>.ts.net/mcp
```

Any other app that supports remote MCP servers over streamable HTTP with OAuth works with the same URL.

## When it doesn't work

- **The app can't reach the server.** The machine is asleep or `glide mcp up` isn't running. A laptop with the lid closed is offline.
- **Sign-in ends with "isn't allowed to open this vault".** Add that GitHub login to `allow`.
- **GitHub says the redirect URI is wrong.** The OAuth App's callback URL has to match the one `glide mcp up` prints, including the machine name.
- **`tailscale funnel` fails.** Funnel isn't allowed for this machine yet in the Tailscale admin console.
