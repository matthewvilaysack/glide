use std::path::PathBuf;

use anyhow::{Context, Result};
use glide_common::errors::GlideError;
use glide_common::paths::find_repo_root;
use glide_common::Config;
use glide_graph::GraphDb;

pub mod completion;
pub mod config;
#[cfg(feature = "console")]
pub mod console;
pub mod doctor;
pub mod focus;
pub mod friction;
pub mod index;
pub mod init;
pub mod mcp;
pub mod models;
pub mod one_shot;
pub mod plan;
pub mod prime;
pub mod request;
pub mod setup;
pub mod sprint;
pub mod tools;
pub mod who_owns;

/// Common context fetched once per command. Resolves repo root, loads layered
/// config, opens the graph DB if it exists.
pub struct CmdCtx {
    pub repo_root: PathBuf,
    pub config: Config,
    pub db_path: PathBuf,
}

/// Open the vault from the layered config. Works outside a git repo, because
/// the vault is the person's, not the repository's.
/// The vault folder itself, for things that live beside the daily notes.
///
/// A sprint note is the user's file in the user's vault, the same as every daily
/// note, so it resolves the same way rather than inventing a second home for state.
pub fn vault_root() -> Result<std::path::PathBuf> {
    Ok(open_vault()?.root)
}

pub fn open_vault() -> Result<glide_vault::Vault> {
    let cwd = std::env::current_dir()?;
    let repo_root = find_repo_root(&cwd);
    let config = Config::load(repo_root.as_deref())?;
    let v = &config.vault;
    Ok(
        glide_vault::Vault::new(&v.path, &v.daily_note_pattern)?.with_sections(
            glide_vault::Sections {
                focus: v.focus_heading.clone(),
                tasks: v.tasks_heading.clone(),
                record: v.record_heading.clone(),
                notes: v.notes_heading.clone(),
            },
        ),
    )
}

impl CmdCtx {
    pub fn discover() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let repo_root = find_repo_root(&cwd).ok_or_else(|| {
            GlideError::Config(format!("not inside a git repo (cwd: {})", cwd.display()))
        })?;
        let config = Config::load(Some(&repo_root))?;
        let db_path = config.resolved_db_path(&repo_root);
        Ok(CmdCtx {
            repo_root,
            config,
            db_path,
        })
    }

    pub fn open_db(&self) -> Result<GraphDb> {
        if !self.db_path.exists() {
            return Err(GlideError::GraphNotInitialized.into());
        }
        glide_graph::open(&self.db_path)
            .with_context(|| format!("opening {}", self.db_path.display()))
    }
}
