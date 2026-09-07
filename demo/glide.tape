# A scripted, reproducible demo of glide against a throwaway vault.
# Render with: vhs demo/glide.tape   (needs vhs, ttyd, ffmpeg)
Output demo.gif
Output demo.mp4
Set Shell zsh
Set FontFamily "Menlo"
Set FontSize 20
Set Width 1180
Set Height 640
Set Padding 28
Set Theme "Catppuccin Mocha"
Set TypingSpeed 45ms
Set Framerate 30
Set PlaybackSpeed 1.0

Env GLIDE_VAULT_PATH "/tmp/notes"
Env PROMPT "%F{green}❯%f "
Env NO_COLOR ""

Hide
Type "setopt interactive_comments" Enter
Type "clear" Enter
Show

Type "glide focus" Sleep 300ms Enter
Sleep 1.6s
Type "glide focus set ship the portal tests" Sleep 300ms Enter
Sleep 1.4s
Type "glide focus set write the roadmap" Sleep 300ms Enter
Sleep 1.4s
Type "glide focus set portal" Sleep 300ms Enter
Sleep 1.6s
Type "# the agent in your terminal logs after each task, without being asked" Enter
Sleep 900ms
Type "glide focus log fixed the flaky build, retry on the artifact step" Sleep 300ms Enter
Sleep 1.6s
Type "glide focus capture ask about the cache revert" Sleep 300ms Enter
Sleep 1.4s
Type "glide focus done portal" Sleep 300ms Enter
Sleep 1.8s
Type "clear" Enter
Type "# one Markdown file, in your own folder" Enter
Sleep 600ms
Type "glide focus today" Sleep 300ms Enter
Sleep 3.2s
Type "clear" Enter
Type "# what an agent sees at the start of every session" Enter
Sleep 600ms
Type "glide prime" Sleep 300ms Enter
Sleep 4s
