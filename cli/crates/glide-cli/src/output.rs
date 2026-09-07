use std::io::IsTerminal;

use owo_colors::OwoColorize;
use serde::Serialize;

use crate::cli::GlobalArgs;

/// Whether the human formatter should emit ANSI color.
pub fn use_color(globals: &GlobalArgs) -> bool {
    if globals.no_color || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

/// Decide whether to render JSON. `--json` always wins; `--stream ndjson` also
/// implies structured output for our verbs (one event = the whole result).
pub fn wants_json(globals: &GlobalArgs) -> bool {
    if globals.json {
        return true;
    }
    matches!(globals.stream, Some(glide_common::Stream::Ndjson))
}

pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    let s = if wants_pretty() {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };
    println!("{s}");
    Ok(())
}

fn wants_pretty() -> bool {
    std::io::stdout().is_terminal()
}

pub fn header(s: &str, color: bool) -> String {
    if color {
        format!("{}", s.bold())
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str, color: bool) -> String {
    if color {
        format!("{}", s.dimmed())
    } else {
        s.to_string()
    }
}

pub fn ok(s: &str, color: bool) -> String {
    if color {
        format!("{}", s.green())
    } else {
        s.to_string()
    }
}

pub fn warn(s: &str, color: bool) -> String {
    if color {
        format!("{}", s.yellow())
    } else {
        s.to_string()
    }
}
