use anyhow::Result;
use globset::Glob;
use rusqlite::params;
use serde::Serialize;

use crate::schema::GraphDb;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnerKind {
    Person,
    Team,
}

impl OwnerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OwnerKind::Person => "person",
            OwnerKind::Team => "team",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        if s == "team" {
            OwnerKind::Team
        } else {
            OwnerKind::Person
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OwnerHit {
    pub owner_handle: String,
    pub owner_kind: OwnerKind,
    pub weight: f64,
    pub source: String,
    pub evidence: Option<String>,
    pub matched_pattern: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphStats {
    pub people: i64,
    pub teams: i64,
    pub paths: i64,
    pub ownership_rows: i64,
    pub friction_events: i64,
    pub last_index_at: Option<String>,
    pub last_index_ok: Option<bool>,
}

impl GraphDb {
    /// Resolve `path` against every `paths.pattern` (treated as a glob).
    /// Returns up to `top` owners, ranked by weight, with evidence when
    /// available.
    pub fn who_owns(&self, path: &str, top: usize) -> Result<Vec<OwnerHit>> {
        // Pull all (pattern, path_id) so we can match in Rust.
        let mut stmt = self.conn.prepare("SELECT id, pattern, kind FROM paths")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;

        let mut matching_path_ids = Vec::new();
        for row in rows {
            let (id, pattern, kind) = row?;
            let matched = match kind.as_str() {
                "exact" => pattern == path,
                _ => Glob::new(&pattern)
                    .ok()
                    .map(|g| g.compile_matcher().is_match(path))
                    .unwrap_or(false),
            };
            if matched {
                matching_path_ids.push((id, pattern));
            }
        }

        if matching_path_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut hits: Vec<OwnerHit> = Vec::new();
        for (path_id, pattern) in matching_path_ids {
            let mut stmt = self.conn.prepare(
                "SELECT owner_id, owner_kind, source, weight, evidence
                 FROM ownership
                 WHERE path_id = ?1
                 ORDER BY weight DESC",
            )?;
            let owners = stmt.query_map(params![path_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, f64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })?;
            for o in owners {
                let (owner_id, owner_kind_s, source, weight, evidence) = o?;
                let owner_kind = OwnerKind::from_str(&owner_kind_s);
                let handle = self.resolve_owner_handle(owner_id, owner_kind)?;
                hits.push(OwnerHit {
                    owner_handle: handle,
                    owner_kind,
                    weight,
                    source,
                    evidence,
                    matched_pattern: pattern.clone(),
                });
            }
        }

        // Coalesce duplicates (same handle + source), keep max weight.
        hits.sort_by(|a, b| {
            (b.weight)
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut seen = std::collections::HashSet::new();
        hits.retain(|h| seen.insert((h.owner_handle.clone(), h.source.clone())));

        Ok(hits.into_iter().take(top).collect())
    }

    fn resolve_owner_handle(&self, owner_id: i64, kind: OwnerKind) -> Result<String> {
        let q = match kind {
            OwnerKind::Person => "SELECT handle FROM people WHERE id = ?1",
            OwnerKind::Team => "SELECT slug FROM teams WHERE id = ?1",
        };
        Ok(self
            .conn
            .query_row(q, params![owner_id], |r| r.get::<_, String>(0))?)
    }

    pub fn stats(&self) -> Result<GraphStats> {
        let people: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM people", [], |r| r.get(0))?;
        let teams: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM teams", [], |r| r.get(0))?;
        let paths: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM paths", [], |r| r.get(0))?;
        let ownership_rows: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM ownership", [], |r| r.get(0))?;
        let friction_events: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM friction_events", [], |r| r.get(0))?;

        let last: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT finished_at, ok FROM index_runs
                 WHERE finished_at IS NOT NULL
                 ORDER BY id DESC LIMIT 1",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .ok();

        Ok(GraphStats {
            people,
            teams,
            paths,
            ownership_rows,
            friction_events,
            last_index_at: last.as_ref().map(|(t, _)| t.clone()),
            last_index_ok: last.map(|(_, ok)| ok != 0),
        })
    }
}
