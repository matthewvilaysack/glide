use anyhow::Result;
use glide_common::paths::{global_dir, workspace_dir};

use crate::cli::{ConfigCmd, ConfigSub, GlobalArgs};
use crate::commands::CmdCtx;
use crate::output::{dim, header, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, cmd: ConfigCmd) -> Result<()> {
    match cmd.sub {
        ConfigSub::List => list(globals),
        ConfigSub::Get { key } => get(globals, &key),
        ConfigSub::Set { key, value } => set(globals, &key, &value),
        ConfigSub::Paths => paths(globals),
    }
}

fn list(globals: &GlobalArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    if wants_json(globals) {
        return print_json(&ctx.config);
    }
    println!("{}", ctx.config.to_toml()?);
    Ok(())
}

fn get(globals: &GlobalArgs, key: &str) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let val = glide_common::config_edit::lookup(&ctx.config, key)
        .ok_or_else(|| anyhow::anyhow!("no such config key: {key}"))?;
    if wants_json(globals) {
        return print_json(&val);
    }
    println!("{val}");
    Ok(())
}

fn set(_globals: &GlobalArgs, key: &str, value: &str) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    // v1: surgical edit of the workspace local file. Full schema-aware set
    // (with type coercion for bool/int/list) is a v1.1 deliverable.
    let local = workspace_dir(&ctx.repo_root).join("config.local.toml");
    glide_common::config_edit::set_in_file(&local, key, value)?;
    println!("set {key} = {value} in {}", local.display());
    Ok(())
}

fn paths(globals: &GlobalArgs) -> Result<()> {
    let color = use_color(globals);
    let global = global_dir().ok();
    let ctx = CmdCtx::discover().ok();
    println!(
        "{}",
        header("config layers (low → high precedence):", color)
    );
    println!("  {} compiled defaults", dim("1.", color));
    if let Some(g) = global {
        println!("  {} {}", dim("2.", color), g.join("config.toml").display());
    }
    if let Some(c) = ctx {
        let ws = workspace_dir(&c.repo_root);
        println!("  {} {}", dim("3.", color), ws.join("glide.toml").display());
        println!(
            "  {} {}",
            dim("4.", color),
            ws.join("config.local.toml").display()
        );
    }
    println!("  {} GLIDE_* env vars", dim("5.", color));
    Ok(())
}
