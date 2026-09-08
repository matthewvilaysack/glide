//! glide's focus verbs, in memory, for the browser playground at
//! tryglide.net/try. No filesystem, no clock, no network: the same
//! `glide-vault` code the binary ships, driven by argv strings, returning the
//! note text after every command so the page can redraw.

use chrono::NaiveDate;
use glide_vault::{prime_text, Daily};
use wasm_bindgen::prelude::*;

/// The date the seed note is for. Fixed so the demo is the same for everyone.
pub const SEED_DATE: &str = "2026-09-07";
/// The note every session starts from.
pub const SEED_NOTE: &str = include_str!("seed.md");
/// The clock reading every `log` line gets; the browser has no local clock
/// worth trusting and the demo should not change with the time of day.
pub const LOG_STAMP: &str = "10:42";

/// What the CLI prints for `glide focus --help`, trimmed to the verbs the
/// playground supports.
pub const HELP: &str = concat!(
    "Usage: glide focus [COMMAND]\n",
    "\n",
    "Commands:\n",
    "  show     Print the one-line focus strip (default)\n",
    "  set      Make this the current focus; adds it to today's Focus if new\n",
    "  done     Check off the first Focus or Tasks bullet matching the text\n",
    "  capture  Drop a quick note into today's Notes\n",
    "  log      Append a timestamped line to today's Record\n",
    "  today    Print today's note as Markdown\n",
    "\n",
    "Also: glide prime   Print what an agent sees at the start of a session\n",
);

/// One command's result. `note` is the whole note after the command.
#[wasm_bindgen(getter_with_clone)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// What the terminal shows: the CLI's stdout, or its stderr line when
    /// `exit` is non-zero.
    pub stdout: String,
    pub note: String,
    pub exit: i32,
}

/// One visitor's note, kept between commands.
#[wasm_bindgen]
pub struct Session {
    note: Daily,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Session {
        Session { note: seed() }
    }

    /// Back to the seed note.
    pub fn reset(&mut self) {
        self.note = seed();
    }

    /// The current note as Markdown.
    pub fn note(&self) -> String {
        self.note.raw.clone()
    }

    /// Run one command line already split into words, with or without the
    /// leading `glide`.
    pub fn run(&mut self, argv: Vec<String>) -> Outcome {
        execute(&mut self.note, &argv)
    }
}

fn seed() -> Daily {
    let date = NaiveDate::parse_from_str(SEED_DATE, "%Y-%m-%d").expect("seed date is valid");
    Daily::parse(date, SEED_NOTE)
}

/// The command surface of `glide focus` and `glide prime`, matching the CLI's
/// stdout byte for byte on success; on failure the CLI's stderr line comes
/// back in `stdout` with a non-zero exit.
pub fn execute(note: &mut Daily, argv: &[String]) -> Outcome {
    let words: Vec<&str> = argv
        .iter()
        .map(String::as_str)
        .filter(|w| !w.is_empty())
        .collect();
    let words = match words.first() {
        Some(&"glide") => &words[1..],
        _ => &words[..],
    };
    if words.iter().any(|w| w.starts_with('-')) {
        return help(note);
    }

    let result = match words {
        ["focus"] | ["focus", "show"] => Ok(format!("{}\n", note.snapshot().line())),
        ["focus", "today"] | ["today"] => Ok(note.raw.clone()),
        ["prime"] => Ok(prime_text(Some(&note.snapshot()))),
        ["focus", "set", rest @ ..] if !rest.is_empty() => {
            let chosen = note.set_now(&rest.join(" "));
            Ok(report("now", &chosen, note))
        }
        ["focus", "done", rest @ ..] if !rest.is_empty() => match note.mark_done(&rest.join(" ")) {
            Ok(done) => Ok(report("done", &done, note)),
            Err(e) => Err(format!("error: {e}\n")),
        },
        ["focus", "capture", rest @ ..] if !rest.is_empty() => {
            let t = rest.join(" ");
            note.capture(&t);
            Ok(report("captured", &t, note))
        }
        ["focus", "log", rest @ ..] if !rest.is_empty() => {
            let t = rest.join(" ");
            note.log_at(LOG_STAMP, &t);
            Ok(report("logged", &t, note))
        }
        _ => return help(note),
    };

    match result {
        Ok(stdout) => Outcome {
            stdout,
            note: note.raw.clone(),
            exit: 0,
        },
        Err(stdout) => Outcome {
            stdout,
            note: note.raw.clone(),
            exit: 1,
        },
    }
}

/// The help text with a usage exit, the note untouched.
fn help(note: &Daily) -> Outcome {
    Outcome {
        stdout: HELP.to_string(),
        note: note.raw.clone(),
        exit: 2,
    }
}

/// `✓ verb what` then the strip, the way `glide focus` reports a write.
fn report(verb: &str, what: &str, note: &Daily) -> String {
    format!("✓ {verb} {what}\n{}\n", note.snapshot().line())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(line: &str) -> Vec<String> {
        line.split(' ').map(String::from).collect()
    }

    #[test]
    fn a_fresh_session_shows_no_focus_and_the_open_tasks() {
        let mut s = Session::new();
        let out = execute(&mut s.note, &args("glide focus"));
        assert_eq!(out.stdout, "▶ no focus set · 2 open\n");
        assert_eq!(out.exit, 0);
        assert_eq!(out.note, SEED_NOTE);
    }

    #[test]
    fn set_reports_the_new_focus_and_tags_it_in_the_note() {
        let mut s = Session::new();
        let out = execute(&mut s.note, &args("glide focus set ship the portal tests"));
        assert_eq!(
            out.stdout,
            "✓ now ship the portal tests\n▶ ship the portal tests · focus 0/1 · 2 open\n"
        );
        assert!(out.note.contains("- [ ] ship the portal tests #now\n"));
    }

    #[test]
    fn log_stamps_the_record_with_the_fixed_clock() {
        let mut s = Session::new();
        let out = execute(&mut s.note, &args("glide focus log fixed the flaky build"));
        assert_eq!(
            out.stdout,
            "✓ logged fixed the flaky build\n▶ no focus set · 2 open · 1 logged\n"
        );
        assert!(out
            .note
            .contains(&format!("- {LOG_STAMP} fixed the flaky build\n")));
    }

    #[test]
    fn capture_appends_to_notes_and_reports_it() {
        let mut s = Session::new();
        let out = execute(
            &mut s.note,
            &args("glide focus capture ask about the cache revert"),
        );
        assert_eq!(
            out.stdout,
            "✓ captured ask about the cache revert\n▶ no focus set · 2 open\n"
        );
        let notes = out.note.split("## Notes\n").nth(1).expect("Notes section");
        assert!(notes.contains("- ask about the cache revert\n"));
    }

    #[test]
    fn done_checks_the_matching_line_and_drops_now() {
        let mut s = Session::new();
        execute(&mut s.note, &args("glide focus set portal"));
        let out = execute(&mut s.note, &args("glide focus done portal"));
        assert_eq!(
            out.stdout,
            "✓ done portal\n▶ no focus set · focus 1/1 · 2 open\n"
        );
        assert!(out.note.contains("- [x] portal\n"));
    }

    #[test]
    fn done_with_no_match_is_an_error_and_leaves_the_note_alone() {
        let mut s = Session::new();
        let out = execute(&mut s.note, &args("glide focus done nothing here"));
        assert_eq!(out.exit, 1);
        assert_eq!(
            out.stdout,
            "error: no line in Focus or Tasks matches \"nothing here\"\n"
        );
        assert_eq!(out.note, SEED_NOTE);
    }

    #[test]
    fn today_prints_the_note_and_prime_prints_the_agent_text() {
        let mut s = Session::new();
        assert_eq!(
            execute(&mut s.note, &args("glide focus today")).stdout,
            SEED_NOTE
        );
        let p = execute(&mut s.note, &args("glide prime")).stdout;
        assert!(p.starts_with(
            "## glide: this person's priorities\n\nToday (2026-09-07): ▶ no focus set · 2 open\n"
        ));
        assert!(!p.contains("No note for today yet"));
    }

    #[test]
    fn unknown_words_print_help_with_a_usage_exit() {
        let mut s = Session::new();
        assert!(HELP.contains("\n  done     Check off"));
        for line in [
            "glide nonsense",
            "glide",
            "glide show",
            "glide focus set",
            "glide focus set --safe x",
            "focus explode",
        ] {
            let out = execute(&mut s.note, &args(line));
            assert_eq!(out.exit, 2, "{line}");
            assert!(out.stdout.starts_with("Usage: glide focus"), "{line}");
            assert_eq!(out.note, SEED_NOTE, "{line}");
        }
    }

    #[test]
    fn reset_restores_the_seed() {
        let mut s = Session::new();
        execute(&mut s.note, &args("glide focus set anything"));
        s.reset();
        assert_eq!(s.note(), SEED_NOTE);
    }
}
