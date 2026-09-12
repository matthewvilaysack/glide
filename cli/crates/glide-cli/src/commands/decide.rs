//! `glide decide`: what has been settled.
//!
//! The cheapest turn to remove is the one spent proposing something that was ruled
//! out last week. Twenty tokens to say it once; a turn costs a hundred thousand.

use anyhow::{anyhow, Result};
use glide_vault::{decide, Vault};

use crate::cli::{DecideArgs, GlobalArgs};
use crate::commands::{open_vault, vault_root};
use crate::output::{dim, ok, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, args: DecideArgs) -> Result<()> {
    let text = args.text.join(" ");
    if text.trim().is_empty() {
        return Err(anyhow!(
            "a decision needs words: `glide decide <what was settled>`, or `--against` for what was ruled out"
        ));
    }
    let root = vault_root()?;
    let _ = open_vault()?;
    let d = decide::add(&root, &text, args.against, Vault::today())?;
    let color = use_color(globals);

    if wants_json(globals) {
        return print_json(&serde_json::json!({
            "recorded": d.text, "against": d.against, "on": d.on,
            "note": decide::note_path(&root).display().to_string(),
        }));
    }
    println!("{}", ok(&d.line(), color));
    println!(
        "{}",
        dim(&decide::note_path(&root).display().to_string(), color)
    );
    Ok(())
}

pub fn list(globals: &GlobalArgs) -> Result<()> {
    let root = vault_root()?;
    let all = decide::load(&root)?;
    if wants_json(globals) {
        let rows: Vec<_> = all
            .iter()
            .map(|d| serde_json::json!({ "text": d.text, "against": d.against, "on": d.on }))
            .collect();
        return print_json(&rows);
    }
    if all.is_empty() {
        println!("nothing settled yet");
        println!(
            "{}",
            dim("record one with `glide decide <text>`", use_color(globals))
        );
        return Ok(());
    }
    for d in &all {
        println!("  {}  {}", d.on, d.line());
    }
    Ok(())
}
