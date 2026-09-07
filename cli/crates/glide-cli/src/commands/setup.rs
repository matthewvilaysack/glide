//! `glide setup <agent>`: wire glide into the agent's own hook system in one
//! command, the way beads does. Claude Code gets a SessionStart hook that runs
//! `glide prime --hook-json`; Warp gets the rule text to paste; anything else
//! gets `glide onboard`, the paragraph to drop in its instructions file.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::cli::{GlobalArgs, SetupArgs, SetupTarget};
use crate::output::{dim, ok, use_color, warn};

const HOOK_CMD: &str = "glide prime --hook-json";

pub fn run(globals: &GlobalArgs, args: SetupArgs) -> Result<()> {
    let color = use_color(globals);
    match args.target {
        SetupTarget::Claude => claude(globals, &args, color),
        SetupTarget::Warp => {
            println!("{}", header_line("Warp", color));
            println!("Settings, AI, Rules, add a rule, paste:\n");
            println!("{}", onboard_text());
            println!("\nThen Settings, AI, MCP servers: command `glide`, arguments `mcp serve`, so the agent has the verbs too.");
            Ok(())
        }
    }
}

fn header_line(name: &str, color: bool) -> String {
    ok(&format!("glide setup {}", name.to_lowercase()), color)
}

/// The paragraph any agent can carry in its instructions file.
pub fn onboard_text() -> String {
    "You also keep my daily priorities with glide. At the start of a session run `glide prime` and mention my current focus in one line. \
     When I say what I am working on, run `glide focus set <text>`; when it is done, `glide focus done <text>`. \
     After every task you complete, run `glide focus log <one or two sentences>` without asking. \
     Anything I ask you to remember goes in `glide focus capture <text>`. Never edit my daily note directly."
        .to_string()
}

fn settings_path(global: bool) -> PathBuf {
    if global {
        directories::BaseDirs::new()
            .map(|b| b.home_dir().join(".claude").join("settings.json"))
            .unwrap_or_else(|| PathBuf::from("~/.claude/settings.json"))
    } else {
        PathBuf::from(".claude").join("settings.json")
    }
}

fn claude(globals: &GlobalArgs, args: &SetupArgs, color: bool) -> Result<()> {
    let path = settings_path(args.global);
    let mut root: Value = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&path)?)
            .with_context(|| format!("{} is not valid JSON", path.display()))?
    } else {
        json!({})
    };
    let present = hook_present(&root);

    if args.check {
        if present {
            println!(
                "{} SessionStart hook is installed in {}",
                ok("✓", color),
                path.display()
            );
        } else {
            println!("{} no glide hook in {}", warn("✗", color), path.display());
            println!(
                "{}",
                dim(
                    "run `glide setup claude` (add --global for every project)",
                    color
                )
            );
        }
        return Ok(());
    }
    if globals.safe {
        anyhow::bail!("--safe: refusing to write {}", path.display());
    }
    if args.remove {
        if !present {
            println!("nothing to remove in {}", path.display());
            return Ok(());
        }
        remove_hook(&mut root);
        write(&path, &root)?;
        println!(
            "{} removed the glide hook from {}",
            ok("✓", color),
            path.display()
        );
        return Ok(());
    }
    if present {
        println!("{} already installed in {}", ok("✓", color), path.display());
        return Ok(());
    }
    add_hook(&mut root);
    write(&path, &root)?;
    println!(
        "{} SessionStart hook added to {}",
        ok("✓", color),
        path.display()
    );
    println!("{}", dim("Every new session (and every compaction) now starts with today's focus and the six verbs.", color));
    println!("{}", dim("Optional, for the tools as MCP too: add {\"glide\": {\"command\": \"glide\", \"args\": [\"mcp\", \"serve\"]}} to mcpServers in .mcp.json.", color));
    Ok(())
}

fn hooks_array(root: &mut Value) -> &mut Vec<Value> {
    let hooks = root
        .as_object_mut()
        .expect("settings root is an object")
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let arr = hooks
        .as_object_mut()
        .expect("hooks is an object")
        .entry("SessionStart")
        .or_insert_with(|| json!([]));
    if !arr.is_array() {
        *arr = json!([]);
    }
    arr.as_array_mut().expect("SessionStart is an array")
}

fn hook_present(root: &Value) -> bool {
    root.pointer("/hooks/SessionStart")
        .and_then(Value::as_array)
        .map(|groups| {
            groups.iter().any(|g| {
                g.get("hooks")
                    .and_then(Value::as_array)
                    .map(|hs| {
                        hs.iter()
                            .any(|h| h.get("command").and_then(Value::as_str) == Some(HOOK_CMD))
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn add_hook(root: &mut Value) {
    hooks_array(root).push(json!({
        "matcher": "",
        "hooks": [{ "type": "command", "command": HOOK_CMD, "timeout": 5 }]
    }));
}

fn remove_hook(root: &mut Value) {
    let arr = hooks_array(root);
    for g in arr.iter_mut() {
        if let Some(hs) = g.get_mut("hooks").and_then(Value::as_array_mut) {
            hs.retain(|h| h.get("command").and_then(Value::as_str) != Some(HOOK_CMD));
        }
    }
    arr.retain(|g| {
        g.get("hooks")
            .and_then(Value::as_array)
            .map(|hs| !hs.is_empty())
            .unwrap_or(false)
    });
}

fn write(path: &PathBuf, root: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(root)?;
    text.push('\n');
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_then_remove_leaves_other_hooks_alone() {
        let mut root = json!({"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "other"}]}], "Stop": []}});
        assert!(!hook_present(&root));
        add_hook(&mut root);
        assert!(hook_present(&root));
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 2);
        remove_hook(&mut root);
        assert!(!hook_present(&root));
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
        assert!(root["hooks"]["Stop"].is_array());
    }

    #[test]
    fn add_is_idempotent_by_command() {
        let mut root = json!({});
        add_hook(&mut root);
        assert!(hook_present(&root));
        remove_hook(&mut root);
        add_hook(&mut root);
        assert_eq!(root["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
    }
}
