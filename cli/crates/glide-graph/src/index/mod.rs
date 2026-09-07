use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use glide_common::Config;
use rusqlite::params;
use tracing::{debug, info};

use crate::schema::GraphDb;

pub mod codeowners;
pub mod git_history;
pub mod repomix;
pub mod team_docs;

#[derive(Debug, Clone)]
pub struct IndexReport {
    pub files_scanned: usize,
    pub commits_scanned: usize,
    pub codeowners_rules: usize,
    pub git_history_buckets: usize,
    pub team_doc_mentions: usize,
    pub repomix: repomix::RepomixReport,
}

/// Full-rebuild index. Drops all rows from `paths`/`ownership`/`people`/`teams`,
/// then re-runs the three sources. Incremental indexing is a v1.1 deliverable.
pub fn run_full(db: &mut GraphDb, repo_root: &Path, cfg: &Config) -> Result<IndexReport> {
    let started_at = Utc::now();
    let run_id = db.conn.query_row(
        "INSERT INTO index_runs (started_at, ok) VALUES (?1, 0) RETURNING id",
        params![started_at.to_rfc3339()],
        |r| r.get::<_, i64>(0),
    )?;

    let tx = db.conn.transaction()?;
    tx.execute("DELETE FROM ownership", [])?;
    tx.execute("DELETE FROM paths", [])?;
    tx.execute("DELETE FROM people", [])?;
    tx.execute("DELETE FROM teams", [])?;
    tx.commit()?;

    let mut report = IndexReport {
        files_scanned: 0,
        commits_scanned: 0,
        codeowners_rules: 0,
        git_history_buckets: 0,
        team_doc_mentions: 0,
        repomix: repomix::RepomixReport {
            ran: false,
            output_path: None,
            bytes: 0,
            mode: "not-run".into(),
            skipped_reason: None,
        },
    };

    info!("indexing CODEOWNERS");
    report.codeowners_rules =
        codeowners::ingest(db, repo_root, cfg).with_context(|| "ingesting CODEOWNERS")?;

    info!("indexing git history");
    let git_report =
        git_history::ingest(db, repo_root, cfg).with_context(|| "ingesting git history")?;
    report.commits_scanned = git_report.commits_scanned;
    report.files_scanned = git_report.files_touched;
    report.git_history_buckets = git_report.buckets;

    info!("indexing team docs");
    report.team_doc_mentions =
        team_docs::ingest(db, repo_root, cfg).with_context(|| "ingesting team docs")?;

    info!("packing repo via repomix");
    report.repomix = repomix::ingest(repo_root, cfg).with_context(|| "running repomix")?;

    db.conn.execute(
        "UPDATE index_runs SET finished_at = ?1, files_scanned = ?2,
         commits_scanned = ?3, ok = 1 WHERE id = ?4",
        params![
            Utc::now().to_rfc3339(),
            report.files_scanned as i64,
            report.commits_scanned as i64,
            run_id
        ],
    )?;

    debug!("index complete: {report:?}");
    Ok(report)
}
