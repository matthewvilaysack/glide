//! `glide prime`: the whole workflow in a few hundred tokens, meant for a
//! SessionStart hook. The shape is borrowed from beads' `bd prime`: CLI plus a
//! hook costs far less context than an MCP tool schema on every request, and
//! SessionStart fires again after compaction, so the agent never forgets.

use anyhow::Result;
use glide_vault::{decide, prime_text_with, Vault};

use crate::cli::{GlobalArgs, PrimeArgs};
use crate::commands::open_vault;

/// How many settled things an agent is told about.
///
/// Short on purpose. The value is in the agent actually reading them, and a fact
/// buried in a long list is likelier to be skipped than one in a short one.
const DECISIONS_IN_PRIME: usize = 8;

pub fn run(_globals: &GlobalArgs, args: PrimeArgs) -> Result<()> {
    let text = match open_vault() {
        Ok(vault) => match vault.load(Vault::today()) {
            Ok(note) => {
                // Decisions ride along with the focus. A missing or unreadable note
                // is an empty list rather than a failure: not having decided
                // anything yet must not stop a session from starting.
                let all = decide::load(&vault.root).unwrap_or_default();
                let recent = decide::recent(&all, DECISIONS_IN_PRIME);
                prime_text_with(Some(&note.snapshot()), &recent)
            }
            Err(e) => format!(
                "{}\n\n(glide could not read today's note: {e})\n",
                prime_text_with(None, &[])
            ),
        },
        Err(e) => format!(
            "{}\n\n(glide could not open the vault: {e})\n",
            prime_text_with(None, &[])
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
