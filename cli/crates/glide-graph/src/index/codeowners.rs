use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use glide_common::Config;
use rusqlite::params;
use tracing::debug;

use crate::schema::GraphDb;

/// Parse every configured CODEOWNERS file and upsert rows into `paths` +
/// `ownership` with `source='codeowners'` and `weight=1.0`. Returns the number
/// of rules ingested.
pub fn ingest(db: &mut GraphDb, repo_root: &Path, cfg: &Config) -> Result<usize> {
    let now = Utc::now().to_rfc3339();
    let mut rules = 0;
    for rel in &cfg.index.codeowners_paths {
        let path = repo_root.join(rel);
        if !path.exists() {
            continue;
        }
        let body = std::fs::read_to_string(&path)?;
        for (lineno, line) in body.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let Some(pattern) = parts.next() else {
                continue;
            };
            let owners: Vec<&str> = parts.collect();
            if owners.is_empty() {
                continue;
            }
            let normalized = normalize_pattern(pattern);
            let path_id = upsert_path(db, &normalized)?;
            for owner_raw in owners {
                let owner = owner_raw.trim_start_matches('@');
                let (kind, owner_id) = if owner.contains('/') {
                    let id = upsert_team(db, owner, "codeowners")?;
                    ("team", id)
                } else {
                    let id = upsert_person(db, owner)?;
                    ("person", id)
                };
                db.conn.execute(
                    "INSERT OR REPLACE INTO ownership
                     (path_id, owner_id, owner_kind, source, weight, evidence, observed_at)
                     VALUES (?1, ?2, ?3, 'codeowners', 1.0, ?4, ?5)",
                    params![
                        path_id,
                        owner_id,
                        kind,
                        format!("{}:{}", rel, lineno + 1),
                        now
                    ],
                )?;
                rules += 1;
            }
        }
        debug!("ingested {rules} CODEOWNERS rules from {}", rel);
    }
    Ok(rules)
}

/// CODEOWNERS uses gitignore-style globs, slightly. Normalize:
///   `*`         → `**/*`        (any file anywhere)
///   `*.tsx`     → `**/*.tsx`
///   `/foo/`     → `foo/**`
///   `src/`      → `src/**`
///   `src/api/`  → `src/api/**`
fn normalize_pattern(raw: &str) -> String {
    let mut p = raw.to_string();
    if p.starts_with('/') {
        p.remove(0);
    }
    if p == "*" {
        return "**/*".into();
    }
    if !p.contains('/') && p.starts_with("*.") {
        return format!("**/{}", p);
    }
    if p.ends_with('/') {
        return format!("{}**", p);
    }
    p
}

pub(crate) fn upsert_path(db: &GraphDb, pattern: &str) -> Result<i64> {
    db.conn.execute(
        "INSERT OR IGNORE INTO paths (pattern, kind) VALUES (?1, 'glob')",
        params![pattern],
    )?;
    Ok(db.conn.query_row(
        "SELECT id FROM paths WHERE pattern = ?1",
        params![pattern],
        |r| r.get::<_, i64>(0),
    )?)
}

pub(crate) fn upsert_person(db: &GraphDb, handle: &str) -> Result<i64> {
    let now = Utc::now().to_rfc3339();
    db.conn.execute(
        "INSERT INTO people (handle, first_seen, last_seen) VALUES (?1, ?2, ?2)
         ON CONFLICT(handle) DO UPDATE SET last_seen = excluded.last_seen",
        params![handle, now],
    )?;
    Ok(db.conn.query_row(
        "SELECT id FROM people WHERE handle = ?1",
        params![handle],
        |r| r.get::<_, i64>(0),
    )?)
}

pub(crate) fn upsert_team(db: &GraphDb, slug: &str, source: &str) -> Result<i64> {
    db.conn.execute(
        "INSERT OR IGNORE INTO teams (slug, source) VALUES (?1, ?2)",
        params![slug, source],
    )?;
    Ok(db
        .conn
        .query_row("SELECT id FROM teams WHERE slug = ?1", params![slug], |r| {
            r.get::<_, i64>(0)
        })?)
}
