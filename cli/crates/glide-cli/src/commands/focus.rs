use anyhow::{anyhow, Result};
use glide_vault::Vault;

use crate::cli::{ClearArgs, FocusCmd, FocusSub, GlobalArgs};
use crate::commands::open_vault;
use crate::output::{dim, ok, print_json, use_color, wants_json};

/// The words a write verb was given, refused when there are none.
///
/// An empty argument used to succeed and write a timestamp with nothing after it,
/// and `done` was worse than that: empty text matches every bullet, so it checked
/// off whichever happened to be first. An agent that calls one of these with a
/// variable that came back empty should be told, not quietly obeyed.
fn words(text: &[String], verb: &str, example: &str) -> Result<String> {
    let t = text.join(" ").trim().to_string();
    if t.is_empty() {
        return Err(anyhow!("`glide focus {verb}` needs words: {example}"));
    }
    Ok(t)
}

pub fn run(globals: &GlobalArgs, cmd: FocusCmd) -> Result<()> {
    let vault = open_vault()?;
    let mut note = vault.load(Vault::today())?;
    let color = use_color(globals);
    match cmd.sub.unwrap_or(FocusSub::Show) {
        FocusSub::Show => {
            let snap = note.snapshot();
            if wants_json(globals) {
                return print_json(&snap);
            }
            println!("{}", snap.line());
            if !globals.quiet && !snap.exists {
                eprintln!("{}", dim(&format!("(no note yet at {})", snap.path), color));
            }
        }
        FocusSub::Today => print!("{}", note.raw),
        FocusSub::Set { text } => {
            let t = words(&text, "set", "`glide focus set ship the portal tests`")?;
            let chosen = note.set_now(&t);
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "now", &chosen, &note.snapshot())?;
        }
        FocusSub::Done { text } => {
            let t = words(&text, "done", "`glide focus done portal`")?;
            let done = note.mark_done(&t)?;
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "done", &done, &note.snapshot())?;
        }
        FocusSub::Capture { text } => {
            let t = words(
                &text,
                "capture",
                "`glide focus capture ask about the cache revert`",
            )?;
            note.capture(&t);
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "captured", &t, &note.snapshot())?;
        }
        FocusSub::List => return show_list(globals),
        FocusSub::Clear(args) => return clear(globals, note, args, color),
        FocusSub::Log { text } => {
            let t = words(&text, "log", "`glide focus log fixed the flaky build`")?;
            note.log(&t);
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "logged", &t, &note.snapshot())?;
        }
    }
    Ok(())
}

/// Remove the day's unfinished items, once someone has said so out loud.
///
/// Reports by default and removes only with --confirm, the same shape every other
/// destructive command in this toolchain uses. The thing being deleted is a person's
/// own writing in a file they own, and a command that empties it on a typo is worse
/// than one that asks twice.
fn clear(
    globals: &GlobalArgs,
    mut note: glide_vault::Daily,
    args: ClearArgs,
    color: bool,
) -> Result<()> {
    let open = note.open_items();

    if open.is_empty() {
        if wants_json(globals) {
            return print_json(&serde_json::json!({ "removed": [], "confirmed": args.confirm }));
        }
        println!("nothing open today");
        return Ok(());
    }

    if !args.confirm {
        if wants_json(globals) {
            return print_json(&serde_json::json!({ "wouldRemove": open, "confirmed": false }));
        }
        println!("{} open, nothing removed:", open.len());
        for item in &open {
            println!("  {}", item);
        }
        println!("{}", dim("pass --confirm to remove them", color));
        return Ok(());
    }

    let removed = note.clear_open();
    write_guard(globals)?;
    note.save()?;

    if wants_json(globals) {
        return print_json(&serde_json::json!({ "removed": removed, "confirmed": true }));
    }
    println!("{}", ok(&format!("cleared {}", removed.len()), color));
    for item in &removed {
        println!("  {}", item);
    }
    Ok(())
}

/// `glide show`: the strip, then the Focus list with the current one marked.
pub fn show_list(globals: &GlobalArgs) -> Result<()> {
    let vault = open_vault()?;
    let note = vault.load(Vault::today())?;
    let snap = note.snapshot();
    if wants_json(globals) {
        return print_json(&snap);
    }
    let color = use_color(globals);
    println!("{}", snap.line());
    for item in &snap.focus {
        let mark = if item.done { "[x]" } else { "[ ]" };
        let now = if item.now { "  ← now" } else { "" };
        println!("  {mark} {}{}", item.text, dim(now, color));
    }
    if snap.focus.is_empty() {
        println!(
            "{}",
            dim(
                "  (nothing in the focus list yet; `glide focus set <text>` adds one)",
                color
            )
        );
    }
    Ok(())
}

fn write_guard(globals: &GlobalArgs) -> Result<()> {
    if globals.safe {
        anyhow::bail!("--safe: refusing to write to the vault");
    }
    Ok(())
}

fn report(
    globals: &GlobalArgs,
    color: bool,
    verb: &str,
    what: &str,
    snap: &glide_vault::Snapshot,
) -> Result<()> {
    if wants_json(globals) {
        return print_json(&serde_json::json!({ "action": verb, "text": what, "snapshot": snap }));
    }
    println!("{} {}", ok(&format!("✓ {verb}"), color), what);
    if !globals.quiet {
        println!("{}", dim(&snap.line(), color));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_verb_with_no_words_is_refused() {
        for (verb, args) in [
            ("log", vec![]),
            ("set", vec![String::new()]),
            ("done", vec!["   ".to_string()]),
            ("capture", vec![String::new(), "  ".to_string()]),
        ] {
            let e = words(&args, verb, "`example`").unwrap_err().to_string();
            assert!(e.contains(verb), "{verb}: {e}");
            assert!(e.contains("example"), "{verb} must show how: {e}");
        }
    }

    #[test]
    fn real_words_survive_and_are_trimmed() {
        let t = words(
            &[
                "  ship".to_string(),
                "the".to_string(),
                "tests  ".to_string(),
            ],
            "set",
            "`x`",
        )
        .unwrap();
        assert_eq!(t, "ship the tests");
    }
}
