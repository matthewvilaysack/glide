use anyhow::Result;
use glide_common::tools::{detect_repomix, install_repomix_global};

use crate::cli::{GlobalArgs, ToolsCmd, ToolsSub};
use crate::output::{header, ok, print_json, use_color, wants_json, warn};

pub fn run(globals: &GlobalArgs, cmd: ToolsCmd) -> Result<()> {
    match cmd.sub {
        ToolsSub::Status => status(globals),
        ToolsSub::Install { tool } => install(globals, &tool),
    }
}

fn status(globals: &GlobalArgs) -> Result<()> {
    let s = detect_repomix();
    if wants_json(globals) {
        return print_json(&s);
    }
    let color = use_color(globals);
    println!("{}", header("glide tools", color));
    if s.available {
        println!(
            "  {} repomix — {} {}",
            ok("✓", color),
            s.version.as_deref().unwrap_or("(version unknown)"),
            match s.cmd {
                Some(glide_common::tools::RepomixCmd::Global) => "(global)",
                Some(glide_common::tools::RepomixCmd::Npx) => "(npx)",
                None => "",
            }
        );
        if let Some(p) = s.bin_path {
            println!("    bin: {}", p);
        }
    } else {
        println!("  {} repomix — not available", warn("✗", color));
        println!("    run: `glide tools install repomix`");
    }
    Ok(())
}

fn install(globals: &GlobalArgs, tool: &str) -> Result<()> {
    let color = use_color(globals);
    match tool {
        "repomix" => {
            println!(
                "{}",
                header("installing repomix via `npm install -g repomix`...", color)
            );
            let s = install_repomix_global()?;
            if wants_json(globals) {
                return print_json(&s);
            }
            if s.available {
                println!(
                    "{} repomix {}",
                    ok("✓ installed:", color),
                    s.version.as_deref().unwrap_or("")
                );
            } else {
                println!(
                    "{} install completed but repomix still not detectable",
                    warn("!", color)
                );
            }
            Ok(())
        }
        other => anyhow::bail!("unknown tool: {other} (known: repomix)"),
    }
}
