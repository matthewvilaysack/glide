use anyhow::Result;
use glide_graph::GraphDb;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlanResponse {
    pub person: String,
    pub role: Option<String>,
    pub sections: Vec<PlanSection>,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanSection {
    pub title: String,
    pub bullets: Vec<String>,
}

/// v1 stub: generates a deterministic day-1 packet skeleton sourced from the
/// graph (top-owned areas, top contributors the new hire should meet). The LLM
/// pass that turns this into prose lives in `glide-llm` and is wired by the CLI
/// crate when `[llm].enabled = true`.
pub fn plan(db: &GraphDb, person: &str, role: Option<String>) -> Result<PlanResponse> {
    let stats = db.stats()?;
    let summary = format!(
        "Graph indexed: {} people, {} teams, {} ownership rows across {} paths.",
        stats.people, stats.teams, stats.ownership_rows, stats.paths
    );

    Ok(PlanResponse {
        person: person.to_string(),
        role: role.clone(),
        sections: vec![
            PlanSection {
                title: "Welcome".into(),
                bullets: vec![format!(
                    "Day-1 packet for {}{}.",
                    person,
                    role.as_deref().map(|r| format!(" ({r})")).unwrap_or_default()
                )],
            },
            PlanSection {
                title: "Repo orientation".into(),
                bullets: vec![
                    summary,
                    "Top owners and recent friction events are available via `glide who-owns` and `glide friction digest`.".into(),
                ],
            },
            PlanSection {
                title: "First-week goals".into(),
                bullets: vec![
                    "Ship a one-line PR (typo fix, doc tweak) before EOW1.".into(),
                    "Pair with each top-owner area for 30 min.".into(),
                ],
            },
        ],
        status: "draft (LLM disabled; enable [llm] in config for prose generation)",
    })
}
