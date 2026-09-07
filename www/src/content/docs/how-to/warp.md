---
title: Warp
description: One global rule, and optionally the MCP server.
---

Warp only loads a folder's rules when you are working inside that folder, and your work happens in your repos, not in your notes.
So the setup is one global rule that follows you everywhere.

```sh
glide setup warp
```

prints the rule.
Open Warp, Settings, AI, Rules, add a rule, and paste it:

```text
You also keep my daily priorities with glide. At the start of a session run `glide prime` and mention my current focus in one line.
When I say what I am working on, run `glide focus set <text>`; when it is done, `glide focus done <text>`.
After every task you complete, run `glide focus log <one or two sentences>` without asking.
Anything I ask you to remember goes in `glide focus capture <text>`. Never edit my daily note directly.
```

For the verbs as tools: Settings, AI, MCP servers, command `glide`, arguments `mcp serve`.

The focus line reaches Warp through the tmux status bar, or through the agent's replies.
