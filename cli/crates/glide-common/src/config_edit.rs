//! Dotted-key reads and writes against the layered config. Lives here rather
//! than in the CLI so the console reaches the same behaviour instead of
//! reimplementing it.

use std::path::Path;

use crate::errors::{GlideError, Result};
use crate::Config;

/// Read a dotted key (`llm.anthropic.model`) off the effective config.
pub fn lookup(cfg: &Config, key: &str) -> Option<String> {
    let v = serde_json::to_value(cfg).ok()?;
    let mut cur = &v;
    for part in key.split('.') {
        cur = cur.get(part)?;
    }
    Some(match cur {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

/// Surgically set a dotted key in a TOML file, preserving everything else.
/// The file must already exist: `glide init` owns creating it.
pub fn set_in_file(path: &Path, key: &str, value: &str) -> Result<()> {
    if !path.exists() {
        return Err(GlideError::Config(format!(
            "no {} — run `glide init` first",
            path.display()
        )));
    }
    let body = std::fs::read_to_string(path)?;
    let mut doc: toml::Value = body
        .parse()
        .map_err(|e| GlideError::Config(format!("parsing {}: {e}", path.display())))?;
    apply_dotted(&mut doc, key, value)?;
    let rendered = toml::to_string_pretty(&doc)
        .map_err(|e| GlideError::Config(format!("rendering toml: {e}")))?;
    std::fs::write(path, rendered)?;
    Ok(())
}

fn apply_dotted(doc: &mut toml::Value, key: &str, value: &str) -> Result<()> {
    let mut parts = key.split('.').collect::<Vec<_>>();
    let leaf = parts
        .pop()
        .ok_or_else(|| GlideError::Config("empty key".into()))?;
    let mut cur = doc;
    for p in parts {
        let table = cur
            .as_table_mut()
            .ok_or_else(|| GlideError::Config(format!("not a table at {p}")))?;
        cur = table
            .entry(p.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    }
    let table = cur
        .as_table_mut()
        .ok_or_else(|| GlideError::Config("not a table at leaf".into()))?;
    table.insert(leaf.to_string(), coerce(value));
    Ok(())
}

/// `true`/`false` become booleans and digits become integers, so a config file
/// round-trips with the types the schema expects.
fn coerce(value: &str) -> toml::Value {
    if let Ok(b) = value.parse::<bool>() {
        return toml::Value::Boolean(b);
    }
    if let Ok(n) = value.parse::<i64>() {
        return toml::Value::Integer(n);
    }
    toml::Value::String(value.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_walks_nested_keys_and_reports_misses() {
        let cfg = Config::default();
        assert!(lookup(&cfg, "llm.anthropic.model").is_some());
        assert_eq!(lookup(&cfg, "llm.nope"), None);
        assert_eq!(lookup(&cfg, "nope"), None);
    }

    #[test]
    fn set_coerces_bools_and_ints_but_leaves_strings_alone() {
        let dir = std::env::temp_dir().join(format!("glide-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.local.toml");
        std::fs::write(&path, "[llm]\nenabled = false\n").unwrap();

        set_in_file(&path, "llm.enabled", "true").unwrap();
        set_in_file(&path, "index.max_commits", "500").unwrap();
        set_in_file(&path, "llm.default_provider", "anthropic").unwrap();

        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("enabled = true"));
        assert!(body.contains("max_commits = 500"));
        assert!(body.contains("default_provider = \"anthropic\""));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn setting_a_missing_file_says_to_init_rather_than_creating_one() {
        let path = std::env::temp_dir().join("glide-does-not-exist-config.toml");
        let err = set_in_file(&path, "llm.enabled", "true").unwrap_err();
        assert!(err.to_string().contains("glide init"));
    }
}
