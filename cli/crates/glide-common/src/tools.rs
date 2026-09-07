//! External tool detection + bootstrap. v1 covers `repomix`.
//!
//! Resolution order, cheapest first:
//!   1. `repomix` on PATH (global npm or brew install)
//!   2. `npx repomix` (uses the cached node_modules; downloads on first run)
//!   3. install via `npm install -g repomix` (only when caller asks)

use std::process::Command;

use serde::Serialize;
use tracing::{debug, info};

use crate::errors::{GlideError, Result};

#[derive(Debug, Clone, Serialize)]
pub enum RepomixCmd {
    /// Use a globally-installed `repomix` binary on PATH.
    Global,
    /// Use `npx repomix` (npm must be on PATH).
    Npx,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepomixStatus {
    pub available: bool,
    pub cmd: Option<RepomixCmd>,
    pub bin_path: Option<String>,
    pub npx_path: Option<String>,
    pub version: Option<String>,
}

/// Probe the system for an available repomix invocation.
pub fn detect_repomix() -> RepomixStatus {
    let bin = which("repomix");
    let npx = which("npx");
    let cmd = if bin.is_some() {
        Some(RepomixCmd::Global)
    } else if npx.is_some() {
        Some(RepomixCmd::Npx)
    } else {
        None
    };

    let version = match &cmd {
        Some(RepomixCmd::Global) => run_capture("repomix", &["--version"]).ok(),
        Some(RepomixCmd::Npx) => run_capture("npx", &["--yes", "repomix", "--version"]).ok(),
        None => None,
    };

    RepomixStatus {
        available: cmd.is_some(),
        cmd,
        bin_path: bin,
        npx_path: npx,
        version,
    }
}

/// Install repomix globally via `npm install -g repomix`. Returns the new
/// status after install.
pub fn install_repomix_global() -> Result<RepomixStatus> {
    if which("npm").is_none() {
        return Err(GlideError::Other(
            "npm not on PATH; install Node.js first (e.g. `brew install node`)".into(),
        ));
    }
    info!("installing repomix via npm...");
    let status = Command::new("npm")
        .args(["install", "-g", "repomix"])
        .status()
        .map_err(|e| GlideError::Other(format!("spawning npm: {e}")))?;
    if !status.success() {
        return Err(GlideError::Other(format!(
            "npm install -g repomix failed (exit {:?})",
            status.code()
        )));
    }
    debug!("repomix installed");
    Ok(detect_repomix())
}

/// Build the command line that should be used to invoke repomix in this env.
/// Returns `(program, [args...])` ready for `Command::new(program).args(args)`.
pub fn repomix_invocation(cmd: &RepomixCmd) -> (String, Vec<String>) {
    match cmd {
        RepomixCmd::Global => ("repomix".into(), vec![]),
        RepomixCmd::Npx => ("npx".into(), vec!["--yes".into(), "repomix".into()]),
    }
}

fn which(bin: &str) -> Option<String> {
    let output = Command::new("which").arg(bin).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| GlideError::Other(format!("spawning {program}: {e}")))?;
    if !output.status.success() {
        return Err(GlideError::Other(format!(
            "{program} {args:?} exited {:?}",
            output.status.code()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
