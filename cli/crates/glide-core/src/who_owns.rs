use anyhow::Result;
use glide_graph::{GraphDb, OwnerHit};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WhoOwnsResponse {
    pub query: String,
    pub hits: Vec<OwnerHit>,
    pub include_evidence: bool,
}

/// Resolve ownership of `path` against the local graph. `top` caps results.
/// `include_evidence` is a hint to the formatter — the data is always there.
pub fn who_owns(
    db: &GraphDb,
    path: &str,
    top: usize,
    include_evidence: bool,
) -> Result<WhoOwnsResponse> {
    let hits = db.who_owns(path, top.max(1))?;
    Ok(WhoOwnsResponse {
        query: path.to_string(),
        hits,
        include_evidence,
    })
}
