//! Prompt templates. Bodies sourced from the existing Claude Code skills in
//! `.claude/skills/{repo-onboarding,who-owns,permissions-request}` — see
//! README for the lineage.

pub const WHO_OWNS: &str = include_str!("who_owns.md");
pub const PLAN: &str = include_str!("plan.md");
pub const REQUEST: &str = include_str!("request.md");
