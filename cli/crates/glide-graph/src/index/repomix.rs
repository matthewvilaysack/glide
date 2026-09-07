use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use glide_common::tools::{detect_repomix, install_repomix_global, repomix_invocation, RepomixCmd};
use glide_common::Config;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RepomixReport {
    pub ran: bool,
    pub output_path: Option<PathBuf>,
    pub bytes: u64,
    pub mode: String,
    pub skipped_reason: Option<String>,
}

/// Run `repomix --compress -o <output>` against the repo root. Returns a
/// report. When `[index.repomix].auto_install` is true and repomix isn't on
/// PATH or available via npx, attempts `npm install -g repomix` first.
pub fn ingest(repo_root: &Path, cfg: &Config) -> Result<RepomixReport> {
    let rcfg = &cfg.index.repomix;
    if !rcfg.enabled {
        return Ok(RepomixReport {
            ran: false,
            output_path: None,
            bytes: 0,
            mode: "disabled".into(),
            skipped_reason: Some("[index.repomix].enabled = false".into()),
        });
    }

    let mut status = detect_repomix();
    if !status.available && rcfg.auto_install {
        info!("repomix not found; installing via `npm install -g repomix`");
        match install_repomix_global() {
            Ok(s) => status = s,
            Err(e) => warn!("auto-install failed: {e}"),
        }
    }
    let Some(cmd) = status.cmd else {
        return Ok(RepomixReport {
            ran: false,
            output_path: None,
            bytes: 0,
            mode: "missing".into(),
            skipped_reason: Some(
                "repomix not on PATH and npx unavailable; run `glide tools install repomix`".into(),
            ),
        });
    };

    let out_rel = PathBuf::from(&rcfg.output);
    let out_abs = if out_rel.is_absolute() {
        out_rel.clone()
    } else {
        repo_root.join(&out_rel)
    };
    if let Some(parent) = out_abs.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }

    let (program, mut args) = repomix_invocation(&cmd);
    args.push("--compress".into());
    args.push("--style".into());
    args.push(rcfg.style.clone());
    args.push("-o".into());
    args.push(out_abs.display().to_string());
    args.extend(rcfg.extra_args.iter().cloned());

    info!("running {} {}", program, args.join(" "));
    let output = Command::new(&program)
        .args(&args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| anyhow!("spawning {program}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "repomix exited {:?}: {}",
            output.status.code(),
            stderr.trim()
        ));
    }

    let bytes = std::fs::metadata(&out_abs).map(|m| m.len()).unwrap_or(0);
    let mode = match cmd {
        RepomixCmd::Global => "global".to_string(),
        RepomixCmd::Npx => "npx".to_string(),
    };

    Ok(RepomixReport {
        ran: true,
        output_path: Some(out_abs),
        bytes,
        mode,
        skipped_reason: None,
    })
}
