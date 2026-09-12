//! `glide prime`: the whole workflow in a few hundred tokens, meant for a
//! SessionStart hook. The shape is borrowed from beads' `bd prime`: CLI plus a
//! hook costs far less context than an MCP tool schema on every request, and
//! SessionStart fires again after compaction, so the agent never forgets.

use anyhow::Result;
use glide_vault::{decide, prime_text_with, Vault};

use crate::cli::{GlobalArgs, PrimeArgs};
use crate::commands::{open_vault, repo_root};

/// How many settled things an agent is told about.
///
/// Short on purpose. The value is in the agent actually reading them, and a fact
/// buried in a long list is likelier to be skipped than one in a short one.
const DECISIONS_IN_PRIME: usize = 8;

/// The team's settled things, from the repository the person is standing in.
///
/// Outside a repo this is empty, which is the right answer rather than an error:
/// a session must start whether or not there is a repo, and whether or not the
/// team has ever settled anything.
fn team_decisions() -> Vec<decide::Decision> {
    repo_root()
        .ok()
        .and_then(|r| decide::load_team(&r).ok())
        .unwrap_or_default()
}

/// Team first, personal filling whatever room is left.
///
/// The cap exists because an agent actually reads a short list and skims a long
/// one. When both kinds compete for that room the team's wins, since a decision
/// somebody made for everyone is the one worth spending the tokens on.
fn for_prime<'a>(
    team: &'a [decide::Decision],
    personal: &'a [decide::Decision],
) -> Vec<&'a decide::Decision> {
    let t = decide::recent(team, DECISIONS_IN_PRIME);
    let room = DECISIONS_IN_PRIME.saturating_sub(t.len());
    // A fact settled personally and then again for the team is one fact.
    let said: Vec<String> = t.iter().map(|d| decide::key(d)).collect();
    let rest: Vec<&decide::Decision> = personal
        .iter()
        .filter(|d| !said.contains(&decide::key(d)))
        .collect();
    let mut out = t;
    out.extend(rest.iter().rev().take(room).rev().copied());
    out
}

pub fn run(_globals: &GlobalArgs, args: PrimeArgs) -> Result<()> {
    let text = match open_vault() {
        Ok(vault) => match vault.load(Vault::today()) {
            Ok(note) => {
                // Decisions ride along with the focus. A missing or unreadable note
                // is an empty list rather than a failure: not having decided
                // anything yet must not stop a session from starting.
                let team = team_decisions();
                let personal = decide::load(&vault.root).unwrap_or_default();
                let picked = for_prime(&team, &personal);
                prime_text_with(Some(&note.snapshot()), &picked)
            }
            Err(e) => {
                let team = team_decisions();
                format!(
                    "{}\n\n(glide could not read today's note: {e})\n",
                    prime_text_with(None, &for_prime(&team, &[]))
                )
            }
        },
        Err(e) => {
            let team = team_decisions();
            format!(
                "{}\n\n(glide could not open the vault: {e})\n",
                prime_text_with(None, &for_prime(&team, &[]))
            )
        }
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
