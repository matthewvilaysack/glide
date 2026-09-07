use anyhow::Result;

use crate::cli::{GlobalArgs, RequestArgs};
use crate::output::{dim, header, print_json, use_color, wants_json};

pub fn run(globals: &GlobalArgs, args: RequestArgs) -> Result<()> {
    let resp = glide_core::request(&args.permission, args.for_person.clone())?;

    if wants_json(globals) {
        return print_json(&resp);
    }
    let color = use_color(globals);
    println!("{} {}", header("access request —", color), resp.permission);
    println!("  {}", resp.draft_message);
    println!();
    println!("{}", dim("channels:", color));
    for c in &resp.channels {
        println!("  - {}", c);
    }
    println!();
    println!("{}", dim(resp.status, color));
    Ok(())
}
