use std::path::Path;

use anyhow::Result;
use glide_common::paths::{find_repo_root, workspace_dir};
use glide_common::tools::{detect_repomix, RepomixCmd};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CheckState {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub state: CheckState,
    pub detail: String,
}

impl Check {
    fn pass(name: &str, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            state: CheckState::Pass,
            detail: detail.into(),
        }
    }
    fn fail(name: &str, detail: impl Into<String>) -> Self {
        Check {
            name: name.into(),
            state: CheckState::Fail,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub checks: Vec<Check>,
}

impl DoctorReport {
    pub fn failures(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.state == CheckState::Fail)
            .count()
    }
}

/// Walk the health checks from `cwd`. Never returns Err for a failing check —
/// a failed check is data, so both the CLI and the console can render it.
pub fn doctor(cwd: &Path) -> Result<DoctorReport> {
    let mut checks = Vec::new();

    match find_repo_root(cwd) {
        None => checks.push(Check::fail("git", "not inside a git repo")),
        Some(root) => {
            checks.push(Check::pass("git", format!("repo at {}", root.display())));

            let ws = workspace_dir(&root);
            if ws.exists() {
                checks.push(Check::pass("workspace", ".glide/ present"));
            } else {
                checks.push(Check::fail(
                    "workspace",
                    ".glide/ missing — run `glide init`",
                ));
            }

            match glide_common::Config::load(Some(&root)) {
                Err(e) => checks.push(Check::fail("config", format!("invalid: {e}"))),
                Ok(cfg) => {
                    checks.push(Check::pass("config", "valid"));
                    checks.extend(graph_checks(&cfg.resolved_db_path(&root)));
                }
            }
        }
    }

    checks.push(repomix_check());
    Ok(DoctorReport { checks })
}

fn graph_checks(db_path: &Path) -> Vec<Check> {
    if !db_path.exists() {
        return vec![Check::fail(
            "graph",
            format!("db missing at {}", db_path.display()),
        )];
    }
    let db = match glide_graph::open(db_path) {
        Ok(db) => db,
        Err(e) => return vec![Check::fail("graph", format!("unreadable: {e}"))],
    };
    let stats = match db.stats() {
        Ok(s) => s,
        Err(e) => return vec![Check::fail("graph", format!("unreadable: {e}"))],
    };

    let mut out = vec![Check::pass(
        "graph",
        format!(
            "{} owners across {} paths",
            stats.ownership_rows, stats.paths
        ),
    )];
    out.push(match (stats.last_index_at, stats.last_index_ok) {
        (Some(t), Some(true)) => Check::pass("index", format!("last run {t}")),
        (Some(t), Some(false)) => Check::fail("index", format!("last run FAILED at {t}")),
        _ => Check::fail("index", "never indexed — run `glide index build`"),
    });
    out
}

fn repomix_check() -> Check {
    let repomix = detect_repomix();
    if !repomix.available {
        return Check::fail("repomix", "not on PATH — run `glide tools install repomix`");
    }
    let mode = match repomix.cmd {
        Some(RepomixCmd::Global) => "global",
        Some(RepomixCmd::Npx) => "npx",
        None => "?",
    };
    Check::pass(
        "repomix",
        format!(
            "{} ({mode})",
            repomix.version.as_deref().unwrap_or("version unknown")
        ),
    )
}
