use anyhow::Result;
use chrono::{Duration, Utc};
use glide_graph::GraphDb;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct FrictionLogResponse {
    pub id: i64,
    pub occurred_at: String,
    pub category: String,
    pub severity: u8,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrictionDigestResponse {
    pub since: String,
    pub events: Vec<FrictionEvent>,
    pub by_category: Vec<(String, usize)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrictionEvent {
    pub id: i64,
    pub occurred_at: String,
    pub category: String,
    pub severity: u8,
    pub person_handle: Option<String>,
    pub subject: String,
    pub notes: Option<String>,
}

pub fn log_event(
    db: &GraphDb,
    subject: &str,
    category: &str,
    severity: u8,
    person_handle: Option<&str>,
    notes: Option<&str>,
) -> Result<FrictionLogResponse> {
    let occurred = Utc::now().to_rfc3339();
    let id: i64 = db.conn.query_row(
        "INSERT INTO friction_events
         (occurred_at, category, severity, person_handle, subject, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
        params![
            occurred,
            category,
            severity as i64,
            person_handle,
            subject,
            notes
        ],
        |r| r.get::<_, i64>(0),
    )?;
    Ok(FrictionLogResponse {
        id,
        occurred_at: occurred,
        category: category.into(),
        severity,
        subject: subject.into(),
    })
}

/// Roll all events newer than `since` (a duration like `7d`/`24h`) into a
/// digest with a per-category breakdown.
pub fn digest(db: &GraphDb, since_days: i64) -> Result<FrictionDigestResponse> {
    let cutoff = Utc::now() - Duration::days(since_days);
    let cutoff_s = cutoff.to_rfc3339();

    let mut stmt = db.conn.prepare(
        "SELECT id, occurred_at, category, severity, person_handle, subject, notes
         FROM friction_events
         WHERE occurred_at >= ?1
         ORDER BY occurred_at DESC",
    )?;
    let rows = stmt.query_map(params![cutoff_s.clone()], |r| {
        Ok(FrictionEvent {
            id: r.get(0)?,
            occurred_at: r.get(1)?,
            category: r.get(2)?,
            severity: r.get::<_, i64>(3)? as u8,
            person_handle: r.get(4)?,
            subject: r.get(5)?,
            notes: r.get(6)?,
        })
    })?;
    let events: Vec<FrictionEvent> = rows.filter_map(Result::ok).collect();

    let mut counts = std::collections::BTreeMap::new();
    for e in &events {
        *counts.entry(e.category.clone()).or_insert(0usize) += 1;
    }
    let by_category: Vec<(String, usize)> = counts.into_iter().collect();

    Ok(FrictionDigestResponse {
        since: cutoff_s,
        events,
        by_category,
    })
}
