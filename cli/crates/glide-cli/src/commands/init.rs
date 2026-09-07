use anyhow::{bail, Result};
use glide_common::paths::{find_repo_root, workspace_dir};
use glide_common::Config;

use crate::cli::{GlobalArgs, InitArgs};
use crate::output::{ok, use_color};

pub fn run(globals: &GlobalArgs, args: InitArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let repo_root = find_repo_root(&cwd)
        .ok_or_else(|| anyhow::anyhow!("not inside a git repo (cwd: {})", cwd.display()))?;
    let ws = workspace_dir(&repo_root);

    if ws.exists() && !args.force {
        bail!(
            ".glide/ already exists at {}; use --force to overwrite",
            ws.display()
        );
    }
    std::fs::create_dir_all(&ws)?;

    let cfg = Config::default();
    let local_toml = ws.join("config.local.toml");
    if !local_toml.exists() || args.force {
        let body = format!(
            "# glide workspace config. This file is gitignored.\n\
             # See <repo>/.glide/glide.toml for team-shared config.\n\n{}",
            cfg.to_toml()?
        );
        std::fs::write(&local_toml, body)?;
    }

    let db_path = cfg.resolved_db_path(&repo_root);
    glide_graph::init_db(&db_path)?;

    let gitignore = ws.join(".gitignore");
    std::fs::write(&gitignore, "config.local.toml\nglide.db\nglide.db-*\n")?;

    let color = use_color(globals);
    if !globals.quiet {
        println!("{} initialized at {}", ok("✓ glide", color), ws.display());
        println!("  config: {}", local_toml.display());
        println!("  db:     {}", db_path.display());
        println!("\nNext: `glide index build`");
    }
    Ok(())
}
