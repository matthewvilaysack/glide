use anyhow::Result;
use serde::Serialize;

use crate::cli::{GlobalArgs, IndexBuildArgs, IndexCmd, IndexShowArgs, IndexSub};
use crate::commands::CmdCtx;
use crate::output::{dim, header, ok, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, cmd: IndexCmd) -> Result<()> {
    match cmd.sub {
        IndexSub::Build(args) => build(globals, args),
        IndexSub::Show(args) => show(globals, args),
    }
}

#[derive(Serialize)]
struct BuildResult {
    files_scanned: usize,
    commits_scanned: usize,
    codeowners_rules: usize,
    git_history_buckets: usize,
    team_doc_mentions: usize,
    repomix_ran: bool,
    repomix_mode: String,
    repomix_path: Option<String>,
    repomix_bytes: u64,
    repomix_skipped: Option<String>,
    db_path: String,
}

fn build(globals: &GlobalArgs, _args: IndexBuildArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    glide_graph::init_db(&ctx.db_path)?;
    let mut db = glide_graph::open(&ctx.db_path)?;

    let report = glide_graph::index::run_full(&mut db, &ctx.repo_root, &ctx.config)?;

    let out = BuildResult {
        files_scanned: report.files_scanned,
        commits_scanned: report.commits_scanned,
        codeowners_rules: report.codeowners_rules,
        git_history_buckets: report.git_history_buckets,
        team_doc_mentions: report.team_doc_mentions,
        repomix_ran: report.repomix.ran,
        repomix_mode: report.repomix.mode.clone(),
        repomix_path: report
            .repomix
            .output_path
            .as_ref()
            .map(|p| p.display().to_string()),
        repomix_bytes: report.repomix.bytes,
        repomix_skipped: report.repomix.skipped_reason.clone(),
        db_path: ctx.db_path.display().to_string(),
    };

    if wants_json(globals) {
        return print_json(&out);
    }

    let color = use_color(globals);
    println!("{}", header("glide index — full rebuild", color));
    println!(
        "  {} {}",
        dim("codeowners rules :", color),
        ok(&out.codeowners_rules.to_string(), color)
    );
    println!(
        "  {} {}",
        dim("git buckets      :", color),
        ok(&out.git_history_buckets.to_string(), color)
    );
    println!(
        "  {} {}",
        dim("team-doc mentions:", color),
        ok(&out.team_doc_mentions.to_string(), color)
    );
    println!(
        "  {} {} commits across {} files",
        dim("scanned          :", color),
        out.commits_scanned,
        out.files_scanned
    );
    if out.repomix_ran {
        let kb = out.repomix_bytes as f64 / 1024.0;
        println!(
            "  {} {} → {} ({:.1} KB, via {})",
            dim("repomix         :", color),
            ok("ok", color),
            out.repomix_path.as_deref().unwrap_or("(unknown)"),
            kb,
            out.repomix_mode
        );
    } else if let Some(why) = &out.repomix_skipped {
        println!("  {} skipped — {}", dim("repomix         :", color), why);
    }
    println!("  {} {}", dim("db               :", color), out.db_path);
    Ok(())
}

fn show(globals: &GlobalArgs, _args: IndexShowArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db()?;
    let stats = db.stats()?;
    if wants_json(globals) {
        return print_json(&stats);
    }
    let color = use_color(globals);
    println!("{}", header("glide graph", color));
    println!("  people          : {}", stats.people);
    println!("  teams           : {}", stats.teams);
    println!("  paths           : {}", stats.paths);
    println!("  ownership rows  : {}", stats.ownership_rows);
    println!("  friction events : {}", stats.friction_events);
    match (stats.last_index_at.as_deref(), stats.last_index_ok) {
        (Some(t), Some(ok_)) => println!(
            "  last indexed    : {} ({})",
            t,
            if ok_ { "ok" } else { "FAILED" }
        ),
        _ => println!("  last indexed    : never (run `glide index build`)"),
    }
    Ok(())
}
