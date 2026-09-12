//! `glide sprint`: the work that outlives today, and what the day draws from.
//!
//! The daily note is deliberately thrown away. That is what makes it useful, and it
//! is also why there has to be somewhere else for the week's work to sit. A sprint
//! is a Markdown note in the same vault, so it opens in Obsidian, diffs in git, and
//! can be edited by hand without this tool's permission, which is the property the
//! whole product is built on.
//!
//! `pull` is the command that matters. Everything else manages a list; `pull` is the
//! join between the list and the day, and it is the reason a sprint is part of glide
//! rather than a separate tool.

use anyhow::{anyhow, Result};
use glide_vault::sprint::{self, Sprint, SprintItem};
use glide_vault::Vault;

use crate::cli::{GlobalArgs, SprintCmd, SprintSub};
use crate::commands::{open_vault, vault_root};
use crate::output::{dim, ok, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, cmd: SprintCmd) -> Result<()> {
    let dir = vault_root()?.join("Sprints");
    let color = use_color(globals);

    match cmd.sub.unwrap_or(SprintSub::Show) {
        SprintSub::Show => show(globals, &dir, color),
        SprintSub::List => list(globals, &dir),
        SprintSub::Start { name } => start(globals, &dir, &name.join(" "), color),
        SprintSub::Add { text } => add(globals, &dir, &text.join(" "), color),
        SprintSub::Done { text } => done(globals, &dir, &text.join(" "), color),
        SprintSub::End => end(globals, &dir, color),
        SprintSub::Pull { text } => pull(globals, &dir, &text.join(" "), color),
    }
}

fn active(dir: &std::path::Path) -> Result<Option<Sprint>> {
    Ok(sprint::list(dir)?.into_iter().find(|s| s.active))
}

/// The active sprint, or a refusal that names the command that fixes it.
fn require_active(dir: &std::path::Path) -> Result<Sprint> {
    active(dir)?.ok_or_else(|| anyhow!("no active sprint; start one with `glide sprint start <name>`"))
}

fn save(s: &Sprint) -> Result<()> {
    let started = chrono::NaiveDate::parse_from_str(&s.started, "%Y-%m-%d").unwrap_or_else(|_| Vault::today());
    if let Some(parent) = s.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&s.path, sprint::render(&s.name, started, s.active, &s.items))?;
    Ok(())
}

fn show(globals: &GlobalArgs, dir: &std::path::Path, color: bool) -> Result<()> {
    match active(dir)? {
        None => {
            if wants_json(globals) {
                return print_json(&serde_json::json!({ "active": null }));
            }
            println!("no active sprint");
            println!("{}", dim("start one with `glide sprint start <name>`", color));
            Ok(())
        }
        Some(s) => {
            if wants_json(globals) {
                return print_json(&serde_json::json!({
                    "active": s.slug, "name": s.name, "started": s.started,
                    "done": s.done_count(), "total": s.items.len(),
                    "open": s.open().iter().map(|i| &i.text).collect::<Vec<_>>(),
                }));
            }
            println!("{}", s.line());
            for item in s.open() {
                println!("  - {}", item.text);
            }
            if s.open().is_empty() && !s.items.is_empty() {
                println!("{}", dim("everything in this sprint is done", color));
            }
            Ok(())
        }
    }
}

fn list(globals: &GlobalArgs, dir: &std::path::Path) -> Result<()> {
    let all = sprint::list(dir)?;
    if wants_json(globals) {
        let rows: Vec<_> = all
            .iter()
            .map(|s| serde_json::json!({ "slug": s.slug, "name": s.name, "active": s.active, "started": s.started, "done": s.done_count(), "total": s.items.len() }))
            .collect();
        return print_json(&rows);
    }
    if all.is_empty() {
        println!("no sprints yet");
        return Ok(());
    }
    for s in all {
        println!("{} {}", if s.active { "*" } else { " " }, s.line());
    }
    Ok(())
}

fn start(globals: &GlobalArgs, dir: &std::path::Path, name: &str, color: bool) -> Result<()> {
    if name.trim().is_empty() {
        return Err(anyhow!("a sprint needs a name: `glide sprint start <name>`"));
    }
    // At most one active sprint, so closing the previous one is part of starting the
    // next rather than a step someone has to remember. The note is left behind.
    if let Some(mut previous) = active(dir)? {
        previous.active = false;
        save(&previous)?;
    }
    let slug = sprint::slugify(name);
    let path = dir.join(format!("{}.md", slug));
    let s = Sprint { slug, name: name.trim().to_string(), started: Vault::today().format("%Y-%m-%d").to_string(), active: true, items: Vec::new(), path };
    save(&s)?;
    if wants_json(globals) {
        return print_json(&serde_json::json!({ "started": s.slug, "path": s.path.display().to_string() }));
    }
    println!("{}", ok(&format!("sprint {}", s.name), color));
    println!("{}", dim(&s.path.display().to_string(), color));
    Ok(())
}

fn add(globals: &GlobalArgs, dir: &std::path::Path, text: &str, color: bool) -> Result<()> {
    let mut s = require_active(dir)?;
    s.items.push(SprintItem { text: text.trim().to_string(), done: false });
    save(&s)?;
    if wants_json(globals) {
        return print_json(&serde_json::json!({ "added": text, "sprint": s.slug, "open": s.open().len() }));
    }
    println!("{}", ok(&format!("added {}", text), color));
    Ok(())
}

fn done(globals: &GlobalArgs, dir: &std::path::Path, text: &str, color: bool) -> Result<()> {
    let mut s = require_active(dir)?;
    let needle = text.trim().to_lowercase();
    let hit = s.items.iter_mut().find(|i| !i.done && i.text.to_lowercase().contains(&needle));
    match hit {
        None => Err(anyhow!("no open item in {} matches {:?}", s.name, text)),
        Some(item) => {
            item.done = true;
            let marked = item.text.clone();
            save(&s)?;
            if wants_json(globals) {
                return print_json(&serde_json::json!({ "done": marked, "sprint": s.slug, "remaining": s.open().len() }));
            }
            println!("{}", ok(&format!("done {}", marked), color));
            Ok(())
        }
    }
}

fn end(globals: &GlobalArgs, dir: &std::path::Path, color: bool) -> Result<()> {
    let mut s = require_active(dir)?;
    s.active = false;
    save(&s)?;
    if wants_json(globals) {
        return print_json(&serde_json::json!({ "ended": s.slug, "done": s.done_count(), "total": s.items.len() }));
    }
    println!("{}", ok(&format!("ended {} ({}/{} done)", s.name, s.done_count(), s.items.len()), color));
    Ok(())
}

/// Take an item out of the sprint and make it today's focus.
///
/// The join between the two. Without it a sprint is a second list to keep in sync by
/// hand, which is the thing this product exists to avoid.
fn pull(globals: &GlobalArgs, dir: &std::path::Path, text: &str, color: bool) -> Result<()> {
    let s = require_active(dir)?;
    let needle = text.trim().to_lowercase();
    let item = s
        .open()
        .into_iter()
        .find(|i| needle.is_empty() || i.text.to_lowercase().contains(&needle))
        .ok_or_else(|| anyhow!("no open item in {} matches {:?}", s.name, text))?
        .text
        .clone();

    let vault = open_vault()?;
    let mut note = vault.load(Vault::today())?;
    let chosen = note.set_now(&item);
    note.save()?;

    if wants_json(globals) {
        return print_json(&serde_json::json!({ "pulled": chosen, "sprint": s.slug, "remaining": s.open().len() }));
    }
    println!("{}", ok(&format!("now {}", chosen), color));
    println!("{}", dim(&format!("from {}", s.name), color));
    Ok(())
}
