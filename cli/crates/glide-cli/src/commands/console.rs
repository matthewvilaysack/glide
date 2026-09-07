use anyhow::Result;

use crate::cli::GlobalArgs;
use crate::commands::CmdCtx;

/// Open the block console. A missing graph is not fatal: the console opens and
/// its first block explains how to build one, which is the more useful answer
/// for someone who just cloned the repo.
pub fn run(globals: &GlobalArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db().ok();

    let repo_label = ctx
        .repo_root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| ctx.repo_root.display().to_string());

    let mut stats_label = match db.as_ref().and_then(|d| d.stats().ok()) {
        Some(s) => format!(
            "{} people · {} paths · {} ownership rows",
            s.people, s.paths, s.ownership_rows
        ),
        None => "no graph".to_string(),
    };
    if globals.safe {
        stats_label.push_str(" · safe");
    }

    let has_graph = db.is_some();
    let engine_ctx = glide_tui::EngineCtx {
        repo_root: ctx.repo_root.clone(),
        db_path: ctx.db_path.clone(),
        config: ctx.config.clone(),
        // Read here, not in glide-tui: that crate has no anthropic feature, so
        // asking it would always answer false.
        anthropic_compiled: cfg!(feature = "anthropic"),
        safe: globals.safe,
    };

    let mut engine = glide_tui::GlideEngine::new(db, engine_ctx);
    let mut app = glide_tui::App::new(repo_label, stats_label, has_graph);
    glide_tui::run_console(&mut app, &mut engine)
}
