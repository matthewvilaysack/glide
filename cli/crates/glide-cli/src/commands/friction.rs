use anyhow::Result;

use crate::cli::{FrictionCmd, FrictionDigestArgs, FrictionLogArgs, FrictionSub, GlobalArgs};
use crate::commands::CmdCtx;
use crate::output::{dim, header, ok, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, cmd: FrictionCmd) -> Result<()> {
    match cmd.sub {
        FrictionSub::Log(args) => log_one(globals, args),
        FrictionSub::Digest(args) => digest(globals, args),
    }
}

fn log_one(globals: &GlobalArgs, args: FrictionLogArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db()?;
    let resp = glide_core::log_event(
        &db,
        &args.subject,
        &args.category,
        args.severity,
        args.person.as_deref(),
        args.notes.as_deref(),
    )?;
    if wants_json(globals) {
        return print_json(&resp);
    }
    let color = use_color(globals);
    println!(
        "{} #{} {} (severity {}) — {}",
        ok("✓ logged", color),
        resp.id,
        dim(&resp.category, color),
        resp.severity,
        resp.subject
    );
    Ok(())
}

fn digest(globals: &GlobalArgs, args: FrictionDigestArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db()?;
    let resp = glide_core::digest(&db, args.since_days)?;
    if wants_json(globals) {
        return print_json(&resp);
    }
    let color = use_color(globals);
    println!(
        "{} last {} days",
        header("friction digest —", color),
        args.since_days
    );
    if resp.events.is_empty() {
        println!("  (no events)");
        return Ok(());
    }
    println!();
    println!("{}", dim("by category:", color));
    for (cat, n) in &resp.by_category {
        println!("  {:<20} {}", cat, n);
    }
    println!();
    println!("{}", dim("events:", color));
    for e in &resp.events {
        println!(
            "  [{}] {} sev{} {} — {}",
            e.occurred_at,
            dim(&e.category, color),
            e.severity,
            e.person_handle.as_deref().unwrap_or("-"),
            e.subject
        );
    }
    Ok(())
}
