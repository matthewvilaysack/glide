//! Terminal lifecycle and the event loop. The only module that talks to a real
//! terminal, and the only one without tests.

use std::io::{self, Stdout, Write};

use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use crate::app::{App, Effect};
use crate::engine::Engine;
use crate::input::action_for;

/// Restores the terminal on the way out, including on an early return.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Run the console until the user quits.
///
/// The terminal is restored twice over: by the guard on any return path, and by
/// a panic hook. Leaving someone in a raw alt-screen with no echo is the worst
/// way a TUI can fail, and it is cheap to make impossible.
pub fn run_console(app: &mut App, engine: &mut dyn Engine) -> Result<()> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    let result = event_loop(&mut terminal, app, engine);

    let _ = std::panic::take_hook();
    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    engine: &mut dyn Engine,
) -> Result<()> {
    loop {
        terminal.draw(|f| crate::render::draw(f, app))?;

        let Event::Key(key) = event::read()? else {
            continue;
        };
        // Windows reports press and release; only act on the press.
        if key.kind != KeyEventKind::Press {
            continue;
        }

        let Some(action) = action_for(key, app.mode) else {
            continue;
        };

        match app.update(action) {
            Some(Effect::Copy(text)) => {
                if let Err(e) = copy_to_clipboard(&text) {
                    app.status_msg = Some(format!("copy failed: {e}"));
                }
            }
            Some(Effect::Run(input)) => {
                // Draw the running block before doing the work. `index build`
                // takes seconds, and this loop is synchronous: the screen still
                // freezes, but it freezes showing what it is doing.
                terminal.draw(|f| crate::render::draw(f, app))?;
                let started = std::time::Instant::now();
                let body = engine.run(&input);
                app.finish_submit(body, started.elapsed().as_millis());
                if let Some(label) = engine.stats_label() {
                    app.stats_label = label;
                }
            }
            None => {}
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Shell out rather than take a clipboard dependency. Three commands cover
/// macOS, Wayland, and X11; anything else gets told plainly that copy failed.
fn copy_to_clipboard(text: &str) -> Result<()> {
    use std::process::{Command, Stdio};

    let candidates: [(&str, &[&str]); 3] = [
        ("pbcopy", &[]),
        ("wl-copy", &[]),
        ("xclip", &["-selection", "clipboard"]),
    ];

    for (bin, args) in candidates {
        let Ok(mut child) = Command::new(bin)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(text.as_bytes())?;
        }
        if child.wait()?.success() {
            return Ok(());
        }
    }
    anyhow::bail!("no clipboard command available")
}
