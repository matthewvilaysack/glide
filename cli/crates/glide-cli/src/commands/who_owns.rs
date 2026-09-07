use anyhow::Result;

use crate::cli::{GlobalArgs, WhoOwnsArgs};
use crate::commands::CmdCtx;
use crate::output::{dim, header, print_json, use_color, wants_json, warn};

pub fn run(globals: &GlobalArgs, args: WhoOwnsArgs) -> Result<()> {
    let ctx = CmdCtx::discover()?;
    let db = ctx.open_db()?;
    let resp = glide_core::who_owns(&db, &args.query, args.top, args.why)?;

    if wants_json(globals) {
        return print_json(&resp);
    }

    let color = use_color(globals);
    if resp.hits.is_empty() {
        println!(
            "{}",
            warn(&format!("no owners found for: {}", args.query), color)
        );
        println!("{}", dim("(graph empty? run `glide index build`)", color));
        return Ok(());
    }

    println!("{} {}", header("owners of", color), args.query);
    for (i, h) in resp.hits.iter().enumerate() {
        let prefix = format!("  {}.", i + 1);
        let kind = match h.owner_kind {
            glide_graph::OwnerKind::Team => "team",
            glide_graph::OwnerKind::Person => "person",
        };
        println!(
            "{} @{}  {} {} (weight {:.2})",
            prefix,
            h.owner_handle,
            dim(&format!("[{kind}]"), color),
            dim(&format!("via {}", h.source), color),
            h.weight,
        );
        if args.why {
            println!("       {} {}", dim("match :", color), h.matched_pattern,);
            if let Some(ev) = &h.evidence {
                println!("       {} {}", dim("why   :", color), ev);
            }
        }
    }
    Ok(())
}
