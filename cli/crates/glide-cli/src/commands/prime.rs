//! `glide prime`: the whole workflow in a few hundred tokens, meant for a
//! SessionStart hook. The shape is borrowed from beads' `bd prime`: CLI plus a
//! hook costs far less context than an MCP tool schema on every request, and
//! SessionStart fires again after compaction, so the agent never forgets.

use anyhow::Result;
use glide_vault::{Snapshot, Vault};

use crate::cli::{GlobalArgs, PrimeArgs};
use crate::commands::open_vault;

pub fn run(_globals: &GlobalArgs, args: PrimeArgs) -> Result<()> {
    let text = match open_vault().and_then(|v| Ok(v.load(Vault::today())?)) {
        Ok(note) => render(Some(&note.snapshot())),
        Err(e) => format!(
            "{}\n\n(glide could not open the vault: {e})\n",
            render(None)
        ),
    };
    if args.hook_json {
        let envelope = serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": text,
            }
        });
        println!("{}", serde_json::to_string(&envelope)?);
    } else {
        print!("{text}");
    }
    Ok(())
}

/// The text an agent needs at the start of a session, and nothing else.
pub fn render(snap: Option<&Snapshot>) -> String {
    let mut s = String::new();
    s.push_str("## glide: this person's priorities\n\n");
    match snap {
        Some(snap) => {
            s.push_str(&format!("Today ({}): {}\n", snap.date, snap.line()));
            if let Some(now) = &snap.now {
                s.push_str(&format!("Current focus: {now}\n"));
            }
            if !snap.focus.is_empty() {
                s.push_str("Focus list:\n");
                for item in &snap.focus {
                    let mark = if item.done { "[x]" } else { "[ ]" };
                    let now = if item.now { " (now)" } else { "" };
                    s.push_str(&format!("- {mark} {}{now}\n", item.text));
                }
            }
            if !snap.exists {
                s.push_str("(No note for today yet; the first write creates it.)\n");
            }
        }
        None => s.push_str("Today: unknown, the vault is not set up.\n"),
    }
    s.push_str(
        "\nWorkflow: mention the current focus in one line at the start. When the person says what they are on, run `glide focus set <text>`. \
         When something finishes, `glide focus done <text>`. After every task you complete, `glide focus log <one or two sentences>`, without being asked. \
         Anything they say to remember: `glide focus capture <text>`. Read the whole note with `glide focus today`. Add `--json` for structured output. \
         Never edit the daily note by hand; these verbs are the only writers.\n",
    );
    s
}
