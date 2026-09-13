//! `glide decide`: what has been settled.
//!
//! The cheapest turn to remove is the one spent proposing something that was ruled
//! out last week. Twenty tokens to say it once; a turn costs a hundred thousand.
//!
//! A decision is either one person's or the team's. The team's goes in the repo,
//! which means git distributes it: one person rules something out, commits, and
//! every teammate's agent knows by their next pull. That is the whole sharing
//! mechanism, and it costs no server, no account and no sync.

use anyhow::{anyhow, Result};
use glide_vault::{decide, Vault};

use crate::cli::{DecideArgs, GlobalArgs};
use crate::commands::{open_vault, repo_root, vault_root};
use crate::output::{dim, ok, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, args: DecideArgs) -> Result<()> {
    let text = args.text.join(" ");
    if text.trim().is_empty() {
        return Err(anyhow!(
            "a decision needs words: `glide decide <what was settled>`, or `--against` for what was ruled out"
        ));
    }

    let (d, where_written) = if args.team {
        // Only `--team` needs a repo, so say that rather than blaming `decide`,
        // which works anywhere.
        let root = repo_root().map_err(|_| {
            anyhow!(
                "`--team` writes the decision into the repository so git can carry it to your teammates, and {} is not inside one.\n\
                 help: cd into the repo this decision is about, or drop `--team` to settle it just for yourself.",
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "this directory".into())
            )
        })?;
        let d = decide::add_team(&root, &text, args.against, Vault::today())?;
        (d, decide::team_path_for_read(&root))
    } else {
        let root = vault_root()?;
        let _ = open_vault()?;
        let d = decide::add(&root, &text, args.against, Vault::today())?;
        (d, decide::note_path(&root))
    };
    let color = use_color(globals);

    if wants_json(globals) {
        return print_json(&serde_json::json!({
            "recorded": d.text, "against": d.against, "on": d.on,
            "team": args.team,
            "note": where_written.display().to_string(),
        }));
    }
    println!("{}", ok(&d.line(), color));
    println!("{}", dim(&where_written.display().to_string(), color));
    if args.team {
        println!(
            "{}",
            dim("commit it and your teammates' agents know too", color)
        );
    }
    Ok(())
}

pub fn list(globals: &GlobalArgs) -> Result<()> {
    let team = match repo_root() {
        Ok(r) => decide::load_team(&r)?,
        Err(_) => Vec::new(),
    };
    let personal = decide::load(&vault_root()?)?;
    let all = decide::merge(team, personal);

    if wants_json(globals) {
        let rows: Vec<_> = all
            .iter()
            .map(|d| {
                serde_json::json!({
                    "text": d.text,
                    "against": d.against,
                    "on": d.on,
                    "team": d.scope == decide::Scope::Team,
                })
            })
            .collect();
        return print_json(&rows);
    }
    if all.is_empty() {
        println!("nothing settled yet");
        println!(
            "{}",
            dim(
                "record one with `glide decide <text>`, or `--team` to settle it for everyone",
                use_color(globals)
            )
        );
        return Ok(());
    }
    for d in &all {
        println!("  {}  {}", d.on, d.line());
    }
    Ok(())
}
