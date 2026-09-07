use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use glide_common::Config;
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use rusqlite::params;
use walkdir::WalkDir;

use super::codeowners::{upsert_path, upsert_team};
use crate::schema::GraphDb;

/// Walk `[index].team_doc_globs`, look for inline patterns like:
///   `@glide/billing owns src/billing/**`
///   `Owner: @glide/api  ->  src/api/`
/// Upsert ownership rows with source='team-doc' and weight=0.5 (heuristic;
/// CODEOWNERS still wins). Returns mention count.
pub fn ingest(db: &mut GraphDb, repo_root: &Path, cfg: &Config) -> Result<usize> {
    let mut builder = GlobSetBuilder::new();
    for pat in &cfg.index.team_doc_globs {
        if let Ok(g) = Glob::new(pat) {
            builder.add(g);
        }
    }
    let set: GlobSet = builder.build()?;

    // Match "@team/slug" followed within ~40 chars by a path-like token.
    let re = Regex::new(
        r"@([a-zA-Z0-9._-]+/[a-zA-Z0-9._-]+)[^\n]{0,40}?(?:owns|->|owns:|:)\s*([A-Za-z0-9_./*-]+)",
    )?;

    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for entry in WalkDir::new(repo_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let abs = entry.path();
        let rel = match abs.strip_prefix(repo_root) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !set.is_match(rel) {
            continue;
        }
        let body = match std::fs::read_to_string(abs) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for cap in re.captures_iter(&body) {
            let team_slug = &cap[1];
            let path_pattern = &cap[2];
            let path_id = upsert_path(db, path_pattern)?;
            let team_id = upsert_team(db, team_slug, "team-doc")?;
            db.conn.execute(
                "INSERT OR REPLACE INTO ownership
                 (path_id, owner_id, owner_kind, source, weight, evidence, observed_at)
                 VALUES (?1, ?2, 'team', 'team-doc', 0.5, ?3, ?4)",
                params![path_id, team_id, format!("{}", rel.display()), now],
            )?;
            count += 1;
        }
    }
    Ok(count)
}
