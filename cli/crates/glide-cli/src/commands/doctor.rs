use anyhow::Result;
use glide_core::CheckState;

use crate::cli::GlobalArgs;
use crate::output::{header, ok, print_json, use_color, wants_json, warn};

pub fn run(globals: &GlobalArgs) -> Result<()> {
    let report = glide_core::doctor(&std::env::current_dir()?)?;

    if wants_json(globals) {
        return print_json(&report);
    }

    let color = use_color(globals);
    println!("{}", header("glide doctor", color));
    for check in &report.checks {
        let glyph = match check.state {
            CheckState::Pass => ok("✓", color),
            CheckState::Fail => warn("✗", color),
        };
        println!("  {} {:<10} {}", glyph, check.name, check.detail);
    }

    let fails = report.failures();
    if fails == 0 {
        println!("\n{}", ok("all checks pass", color));
        Ok(())
    } else {
        anyhow::bail!("{fails} check(s) failed")
    }
}
