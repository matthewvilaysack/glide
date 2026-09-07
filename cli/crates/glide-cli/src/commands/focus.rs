use anyhow::Result;
use glide_vault::Vault;

use crate::cli::{FocusCmd, FocusSub, GlobalArgs};
use crate::commands::open_vault;
use crate::output::{dim, ok, print_json, use_color, wants_json};

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
            let chosen = note.set_now(&text.join(" "));
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "now", &chosen, &note.snapshot())?;
        }
        FocusSub::Done { text } => {
            let done = note.mark_done(&text.join(" "))?;
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "done", &done, &note.snapshot())?;
        }
        FocusSub::Capture { text } => {
            let t = text.join(" ");
            note.capture(&t);
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "captured", &t, &note.snapshot())?;
        }
        FocusSub::Log { text } => {
            let t = text.join(" ");
            note.log(&t);
            write_guard(globals)?;
            note.save()?;
            report(globals, color, "logged", &t, &note.snapshot())?;
        }
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
