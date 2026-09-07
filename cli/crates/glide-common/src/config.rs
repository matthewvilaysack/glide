use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{GlideError, Result};
use crate::paths::{global_dir, workspace_dir};
use crate::stream::Stream;

/// Full glide config. Default-derives compiled defaults; layered with TOML files
/// and `GLIDE_*` env vars in `Config::load`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub graph: GraphConfig,
    pub index: IndexConfig,
    pub llm: LlmConfig,
    pub output: OutputConfig,
    pub permissions: PermissionsConfig,
    pub vault: VaultConfig,
}

/// Where the user's daily notes live. Empty `path` means "not set up".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultConfig {
    /// Folder of Markdown notes, `~` allowed. Usually an Obsidian vault that
    /// iCloud or another folder sync already carries between machines.
    pub path: String,
    /// Relative path of one day's note, with `{date}` for `YYYY-MM-DD`.
    pub daily_note_pattern: String,
    /// The `##` heading that holds the day's priorities.
    pub focus_heading: String,
    /// The `##` heading that holds checkbox tasks.
    pub tasks_heading: String,
    /// The `##` heading `log` appends to.
    pub record_heading: String,
    /// The `##` heading `capture` appends to.
    pub notes_heading: String,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            daily_note_pattern: "Daily Notes/{date}.md".into(),
            focus_heading: "Focus".into(),
            tasks_heading: "Tasks".into(),
            record_heading: "Record".into(),
            notes_heading: "Notes".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    /// Path to the SQLite DB. Workspace-relative if not absolute.
    pub db_path: String,
    pub auto_index_on_command: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            db_path: ".glide/glide.db".into(),
            auto_index_on_command: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IndexConfig {
    pub exclude: Vec<String>,
    pub codeowners_paths: Vec<String>,
    pub team_doc_globs: Vec<String>,
    pub git_history_window_days: u32,
    pub repomix: RepomixConfig,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            exclude: vec![
                "node_modules/**".into(),
                "dist/**".into(),
                "target/**".into(),
            ],
            codeowners_paths: vec![
                ".github/CODEOWNERS".into(),
                "CODEOWNERS".into(),
                "docs/CODEOWNERS".into(),
            ],
            team_doc_globs: vec!["docs/teams/**/*.md".into(), "TEAM.md".into()],
            git_history_window_days: 365,
            repomix: RepomixConfig::default(),
        }
    }
}

/// Repomix integration: `npx repomix --compress -o <output>`.
/// Output is a compact whole-repo snapshot (function/class signatures only)
/// dropped at `<output>` for LLM-context use by `glide plan` and `glide who-owns`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RepomixConfig {
    pub enabled: bool,
    pub output: String,
    pub style: String,
    pub auto_install: bool,
    pub extra_args: Vec<String>,
}

impl Default for RepomixConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            output: ".glide/repomix.xml".into(),
            style: "xml".into(),
            auto_install: true,
            extra_args: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    pub enabled: bool,
    pub default_provider: String,
    #[serde(default)]
    pub anthropic: AnthropicConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_provider: "anthropic".into(),
            anthropic: AnthropicConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnthropicConfig {
    pub api_key_env: String,
    pub model: String,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key_env: "ANTHROPIC_API_KEY".into(),
            model: "claude-haiku-4-5".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    pub default_stream: Stream,
    pub color: String,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            default_stream: Stream::Human,
            color: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PermissionsConfig {
    pub mode: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            mode: "default".into(),
            allow: vec![],
            deny: vec!["shell:rm -rf *".into()],
        }
    }
}

impl Config {
    /// Load layered config: defaults → ~/.glide/config.toml → <repo>/.glide/glide.toml
    /// → <repo>/.glide/config.local.toml → GLIDE_* env vars.
    pub fn load(repo_root: Option<&Path>) -> Result<Config> {
        let mut merged = Config::default();

        if let Ok(global) = global_dir() {
            merge_from_file(&mut merged, &global.join("config.toml"))?;
        }
        if let Some(root) = repo_root {
            let ws = workspace_dir(root);
            merge_from_file(&mut merged, &ws.join("glide.toml"))?;
            merge_from_file(&mut merged, &ws.join("config.local.toml"))?;
        }
        apply_env_overrides(&mut merged);
        Ok(merged)
    }

    /// Resolve the DB path to an absolute path, given the repo root.
    pub fn resolved_db_path(&self, repo_root: &Path) -> PathBuf {
        let p = PathBuf::from(&self.graph.db_path);
        if p.is_absolute() {
            p
        } else {
            repo_root.join(p)
        }
    }

    /// Serialize to a TOML string (for `glide init` to write a starter file).
    pub fn to_toml(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

fn merge_from_file(target: &mut Config, path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(path)?;
    let incoming: Config =
        toml::from_str(&raw).map_err(|e| GlideError::Config(format!("{}: {e}", path.display())))?;
    // Naive merge: incoming wins where set. For v1 the layers carry the whole
    // struct, so this is OK; richer per-field merging can come later.
    *target = incoming;
    Ok(())
}

fn apply_env_overrides(cfg: &mut Config) {
    if let Ok(v) = std::env::var("GLIDE_DB_PATH") {
        cfg.graph.db_path = v;
    }
    if let Ok(v) = std::env::var("GLIDE_VAULT_PATH") {
        cfg.vault.path = v;
    }
    if let Ok(v) = std::env::var("GLIDE_VAULT_DAILY_NOTE_PATTERN") {
        cfg.vault.daily_note_pattern = v;
    }
    if let Ok(v) = std::env::var("GLIDE_DEFAULT_PROVIDER") {
        cfg.llm.default_provider = v;
    }
    if let Ok(v) = std::env::var("GLIDE_MODEL") {
        cfg.llm.anthropic.model = v;
    }
    if let Ok(v) = std::env::var("GLIDE_LLM_ENABLED") {
        cfg.llm.enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
    }
    if let Ok(v) = std::env::var("GLIDE_STREAM") {
        if let Ok(s) = v.parse() {
            cfg.output.default_stream = s;
        }
    }
    if std::env::var("NO_COLOR").is_ok() || std::env::var("GLIDE_NO_COLOR").is_ok() {
        cfg.output.color = "never".into();
    }
}
