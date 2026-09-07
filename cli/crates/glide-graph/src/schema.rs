use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct GraphDb {
    pub conn: Connection,
}

const MIGRATION_0001: &str = include_str!("../migrations/0001_initial.sql");

/// Open (and migrate) the SQLite DB at `path`. Creates parent dirs if missing.
pub fn open(path: &Path) -> Result<GraphDb> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating db parent dir {}", parent.display()))?;
    }
    let conn =
        Connection::open(path).with_context(|| format!("opening sqlite at {}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA synchronous = NORMAL;",
    )?;
    migrate(&conn)?;
    Ok(GraphDb { conn })
}

/// Equivalent to `open` but lifts the parent-dir creation + always runs
/// migrations. Used by `glide init`.
pub fn init_db(path: &Path) -> Result<()> {
    let _ = open(path)?;
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(MIGRATION_0001)?;
        conn.pragma_update(None, "user_version", 1)?;
    }
    Ok(())
}
