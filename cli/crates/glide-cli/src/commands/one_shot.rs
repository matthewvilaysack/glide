use anyhow::Result;
use glide_core::RouteDecision;

use crate::cli::GlobalArgs;
use crate::commands::CmdCtx;
use crate::output::{dim, header, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, prompt: &str) -> Result<()> {
    let decision = glide_core::route_one_shot(prompt);

    if wants_json(globals) {
        return print_json(&decision);
    }

    let color = use_color(globals);
    match decision {
        RouteDecision::WhoOwns { query } => {
            println!("{} who-owns {}", dim("→", color), query);
            let ctx = CmdCtx::discover()?;
            let db = ctx.open_db()?;
            let resp = glide_core::who_owns(&db, query.trim(), 5, true)?;
            for h in resp.hits {
                println!(
                    "  @{} (weight {:.2}, via {})",
                    h.owner_handle, h.weight, h.source
                );
            }
        }
        RouteDecision::Plan { person } => {
            println!("{} plan for {}", dim("→", color), person);
            let ctx = CmdCtx::discover()?;
            let db = ctx.open_db()?;
            let resp = glide_core::plan(&db, person.trim(), None)?;
            for s in resp.sections {
                println!("\n## {}", s.title);
                for b in s.bullets {
                    println!("- {}", b);
                }
            }
        }
        RouteDecision::Request { permission } => {
            let resp = glide_core::request(&permission, None)?;
            println!("{}", resp.draft_message);
        }
        RouteDecision::FrictionDigest => {
            let ctx = CmdCtx::discover()?;
            let db = ctx.open_db()?;
            let resp = glide_core::digest(&db, 7)?;
            println!(
                "{} {} events in last 7d",
                header("friction:", color),
                resp.events.len()
            );
        }
        RouteDecision::Unrouted { reason } => {
            println!("{}", reason);
        }
    }
    Ok(())
}
