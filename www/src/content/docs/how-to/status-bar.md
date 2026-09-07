---
title: Status bar
description: Putting the focus line in tmux, a shell prompt, or Warp.
---

The strip is meant to be seen without looking for it.

## tmux

```
set -g status-right '#(glide focus 2>/dev/null) '
set -g status-interval 15
```

iTerm2 with tmux integration shows the same line.

## zsh prompt

```sh
precmd() { GLIDE_LINE=$(glide focus 2>/dev/null) }
RPROMPT='%F{243}${GLIDE_LINE}%f'
```

## Starship

```toml
[custom.glide]
command = "glide focus"
when = "true"
format = "[$output]($style) "
style = "dim"
```

## Warp

Warp shows the line through the tmux status bar, or through the agent's replies once the [rule](/docs/how-to/warp/) is installed.

## The console

Bare `glide` opens the block console with the strip pinned in its top bar; see [Console](/docs/reference/console/).
