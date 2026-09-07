use anyhow::Result;

use crate::cli::{GlobalArgs, PlanArgs};
use crate::commands::CmdCtx;
use crate::output::{header, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, args: PlanArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db()?;
    let resp = glide_core::plan(&db, &args.person, args.role.clone())?;

    if wants_json(globals) {
        if let Some(out) = args.out {
            std::fs::write(&out, serde_json::to_string_pretty(&resp)?)?;
            return Ok(());
        }
        return print_json(&resp);
    }

    let color = use_color(globals);
    let mut buf = String::new();
    buf.push_str(&format!(
        "# Day-1 plan — {}{}\n\n",
        resp.person,
        resp.role
            .as_deref()
            .map(|r| format!(" ({r})"))
            .unwrap_or_default()
    ));
    buf.push_str(&format!("> {}\n\n", resp.status));
    for s in &resp.sections {
        buf.push_str(&format!("## {}\n\n", s.title));
        for b in &s.bullets {
            buf.push_str(&format!("- {}\n", b));
        }
        buf.push('\n');
    }
    if let Some(out) = args.out {
        std::fs::write(&out, &buf)?;
        if !globals.quiet {
            println!("{} {}", header("wrote plan to", color), out.display());
        }
    } else {
        print!("{buf}");
    }
    Ok(())
}
