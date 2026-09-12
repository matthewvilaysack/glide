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
pub mod decide;
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

/// What to say when a repository command is run outside one.
///
/// Glide has two halves and they have different requirements. The daily note lives
/// in a vault and works from anywhere; the graph is built from a repository and
/// cannot exist without one. Saying only "not inside a git repo" states a fact and
/// leaves someone to guess whether they typed the wrong command or are standing in
/// the wrong place, when the answer is usually the second and the commands that
/// would have worked are one line away.
pub fn not_in_repo(cwd: &std::path::Path) -> String {
    format!(
        "`{}` reads a repository, and {} is not inside one.\n\
         help: cd into a repo first. The daily-note commands (glide, focus, sprint, prime, show) work anywhere.",
        std::env::args().nth(1).unwrap_or_else(|| "this command".into()),
        cwd.display()
    )
}

impl CmdCtx {
    pub fn discover() -> Result<Self> {
        let cwd = std::env::current_dir()?;
        let repo_root =
            find_repo_root(&cwd).ok_or_else(|| GlideError::NotInRepo(not_in_repo(&cwd)))?;
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

#[cfg(test)]
mod tests {
    use super::not_in_repo;
    use std::path::Path;

    #[test]
    fn the_refusal_names_what_works_instead() {
        // A refusal that only states the problem costs a second guess. This one has
        // to carry the commands that would have worked from where they are standing.
        let msg = not_in_repo(Path::new("/Users/someone"));
        assert!(msg.contains("/Users/someone"));
        assert!(msg.contains("cd into a repo"));
        assert!(msg.contains("focus"), "names a command that works anywhere");
        assert!(msg.contains("prime"));
    }
}
