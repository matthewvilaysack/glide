//! `glide prime`: the whole workflow in a few hundred tokens, meant for a
//! SessionStart hook. The shape is borrowed from beads' `bd prime`: CLI plus a
//! hook costs far less context than an MCP tool schema on every request, and
//! SessionStart fires again after compaction, so the agent never forgets.

use anyhow::Result;
use glide_vault::{prime_text, Vault};

use crate::cli::{GlobalArgs, PrimeArgs};
use crate::commands::open_vault;

pub fn run(_globals: &GlobalArgs, args: PrimeArgs) -> Result<()> {
    let text = match open_vault().and_then(|v| Ok(v.load(Vault::today())?)) {
        Ok(note) => prime_text(Some(&note.snapshot())),
        Err(e) => format!(
            "{}\n\n(glide could not open the vault: {e})\n",
            prime_text(None)
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
