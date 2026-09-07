use anyhow::Result;

use crate::cli::GlobalArgs;
use crate::commands::CmdCtx;
use crate::output::{dim, header, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let out = glide_core::models(&ctx.config, cfg!(feature = "anthropic"));

    if wants_json(globals) {
        return print_json(&out);
    }
    let color = use_color(globals);
    println!(
        "{} {} (default: {})",
        header("llm:", color),
        if out.enabled { "enabled" } else { "disabled" },
        out.default_provider
    );
    let suffix = dim(" [feature not compiled]", color);
    for p in &out.providers {
        let tail = if p.feature_compiled {
            ""
        } else {
            suffix.as_str()
        };
        println!(
            "  - {} ({}) — key {} {}{}",
            p.id,
            p.model,
            p.api_key_env,
            if p.key_present { "set" } else { "MISSING" },
            tail,
        );
    }
    Ok(())
}
