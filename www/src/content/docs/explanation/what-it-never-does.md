---
title: What glide never does
description: The lines the product will not cross, so you can rely on them.
---

- **Never leaves your machine.** No telemetry, no account, no network in the focus features. The vault is never copied or uploaded, including in the one opt-in exception below, where an app you connect gets only the notes it asks for.
- **Never edits outside the four sections.** Focus, Tasks, Record, Notes. Frontmatter, template tags, and every other heading survive byte for byte.
- **Never runs in the background.** No daemon, no watcher, no lock file. Every verb starts, does one thing, and exits. The exception is `glide mcp up`, which you start yourself and which stops when you stop it.
- **Never becomes a project tool.** No boards, no sprints, no integrations beyond MCP and a hook. The moment a tool competes with Linear on surface area it loses; that lesson was paid for by others.
- **Never writes when told not to.** `--safe` turns every write into a refusal that says so, in the CLI, the console, and the MCP server alike.
- **Never asks you to move.** Your terminal, your notes app, your folder sync stay what they are. Glide adds one binary.

If any of these change, it will be a major version and a page like this will say why.

## What changed in 0.7

People wanted their notes in the AI apps they already use on the web and on their phones, not only in the terminal.
`glide mcp up` makes that possible without sending the vault anywhere: it serves the vault from your own machine, through Tailscale Funnel, to apps that sign in as a GitHub login you allow.
It is off unless you run it, and it stops when you stop it.
Apps can search and read notes outside the folders you hide, and the only writes go to today's note through the same verbs as everywhere else.
The details are in [Use your vault from any AI app](/docs/how-to/remote/).
