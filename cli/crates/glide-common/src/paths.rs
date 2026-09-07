use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::errors::{GlideError, Result};

/// Resolve the global config directory (`~/.glide/`). Creates it if missing.
pub fn global_dir() -> Result<PathBuf> {
    let base = BaseDirs::new().ok_or_else(|| GlideError::Config("no home directory".into()))?;
    let dir = base.home_dir().join(".glide");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Walk up from `start` looking for a `.git` directory (or worktree gitfile).
/// Returns the repo root or None.
pub fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start.to_path_buf());
    while let Some(p) = cur {
        if p.join(".git").exists() {
            return Some(p);
        }
        cur = p.parent().map(Path::to_path_buf);
    }
    None
}

/// Resolve `<repo>/.glide/` for a given repo root.
pub fn workspace_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".glide")
}
