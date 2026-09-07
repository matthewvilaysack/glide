use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use chrono::{Duration, Utc};
use git2::{DiffOptions, Repository, Sort};
use glide_common::Config;
use rusqlite::params;
use tracing::warn;

use super::codeowners::{upsert_path, upsert_person};
use crate::schema::GraphDb;

pub struct GitReport {
    pub commits_scanned: usize,
    pub files_touched: usize,
    pub buckets: usize,
}

/// Walk `git log --since=<window>` and bucket commits by (file, author_email).
/// Weight = author_commits / total_commits_for_file. Source = 'git-history'.
pub fn ingest(db: &mut GraphDb, repo_root: &Path, cfg: &Config) -> Result<GitReport> {
    let repo = match Repository::discover(repo_root) {
        Ok(r) => r,
        Err(e) => {
            warn!("no git repo at {}: {e}", repo_root.display());
            return Ok(GitReport {
                commits_scanned: 0,
                files_touched: 0,
                buckets: 0,
            });
        }
    };

    let cutoff = Utc::now() - Duration::days(cfg.index.git_history_window_days as i64);
    let cutoff_secs = cutoff.timestamp();

    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TIME)?;
    if revwalk.push_head().is_err() {
        return Ok(GitReport {
            commits_scanned: 0,
            files_touched: 0,
            buckets: 0,
        });
    }

    // (file_path, author_email) -> commit_count
    let mut bucket: HashMap<(String, String), u32> = HashMap::new();
    let mut file_totals: HashMap<String, u32> = HashMap::new();
    let mut commits_scanned = 0usize;

    for oid in revwalk {
        let Ok(oid) = oid else { continue };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        if commit.time().seconds() < cutoff_secs {
            continue;
        }
        commits_scanned += 1;

        let author_email = commit.author().email().unwrap_or("(unknown)").to_string();

        let tree = commit.tree()?;
        // Compare to first parent (skip merges + initial commit edge case).
        let parent_tree = commit
            .parent_count()
            .eq(&1)
            .then(|| commit.parent(0).ok().and_then(|p| p.tree().ok()))
            .flatten();

        let diff = repo.diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&tree),
            Some(&mut DiffOptions::new()),
        )?;

        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                    let path_str = path.to_string_lossy().to_string();
                    if is_excluded(&path_str, &cfg.index.exclude) {
                        return true;
                    }
                    *bucket
                        .entry((path_str.clone(), author_email.clone()))
                        .or_insert(0) += 1;
                    *file_totals.entry(path_str).or_insert(0) += 1;
                }
                true
            },
            None,
            None,
            None,
        )?;
    }

    let now = Utc::now().to_rfc3339();
    let mut written = 0usize;
    for ((file, author), count) in &bucket {
        let total = *file_totals.get(file).unwrap_or(&1) as f64;
        let weight = (*count as f64) / total;
        let path_id = upsert_path(db, file)?;
        let person_id = upsert_person(db, author)?;
        db.conn.execute(
            "INSERT OR REPLACE INTO ownership
             (path_id, owner_id, owner_kind, source, weight, evidence, observed_at)
             VALUES (?1, ?2, 'person', 'git-history', ?3, ?4, ?5)",
            params![
                path_id,
                person_id,
                weight,
                format!(
                    "{count} of {} commits in last {}d",
                    total as u32, cfg.index.git_history_window_days
                ),
                now
            ],
        )?;
        written += 1;
    }

    Ok(GitReport {
        commits_scanned,
        files_touched: file_totals.len(),
        buckets: written,
    })
}

fn is_excluded(path: &str, patterns: &[String]) -> bool {
    use globset::Glob;
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            if g.compile_matcher().is_match(path) {
                return true;
            }
        }
    }
    false
}
