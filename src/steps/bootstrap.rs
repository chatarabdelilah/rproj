use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::catalog::tool_catalog::{ToolEntry, ToolKind};
use crate::steps::{capture, probe, run};
use crate::ui;

pub fn is_installed(entry: &ToolEntry) -> bool {
    match entry.kind {
        ToolKind::SystemApp { winget_id } => {
            probe("winget", &["list", "--id", winget_id, "-e"])
        }
        _ => false,
    }
}

pub fn install(entry: &ToolEntry) -> Result<()> {
    match entry.kind {
        ToolKind::SystemApp { winget_id } => install_winget(winget_id),
        _ => Ok(()),
    }
}

/// Runs `winget install` with output captured (not just inherited) so a
/// hash-mismatch failure - a known, ongoing upstream winget-pkgs issue for
/// installers like Roblox's that self-update behind a static download URL,
/// leaving the pinned manifest hash stale - can be called out with an
/// actionable message instead of a bare exit-code error.
fn install_winget(winget_id: &str) -> Result<()> {
    let args = [
        "install",
        "--id",
        winget_id,
        "-e",
        "--accept-source-agreements",
        "--accept-package-agreements",
    ];
    let output = capture("winget", &args, None)?;
    if !output.success || ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if output.success {
        return Ok(());
    }

    // A hash mismatch is a known, ongoing upstream winget-pkgs issue, not
    // anything the user did. Keep the headline short and put the recovery
    // options in `detail` so they're available without dominating the run.
    if output.combined().contains("Installer hash does not match") {
        bail!("winget's pinned installer hash is stale (an upstream winget-pkgs issue, not rproj)");
    }
    bail!("winget install failed");
}

/// Recovery options for the hash-mismatch case, printed by the caller
/// alongside its warning so the failure is actionable without every other
/// install path carrying the same wall of text.
pub const WINGET_HASH_HELP: &str = "A vendor updated their installer behind a static URL faster than winget's manifest.\n\
     - Try again in a few days; the winget-pkgs bots usually catch up\n\
     - Or install it directly (for Studio: https://www.roblox.com/create)\n\
     - Or, as admin, `winget settings --enable InstallerHashOverride` once, then retry";

/// Ensures Rokit itself is present. Rokit isn't on winget - its own docs
/// specify `cargo install rokit --locked` then `rokit self-install`.
pub fn ensure_rokit() -> Result<()> {
    if probe("rokit", &["--version"]) {
        ui::ok("rokit already installed");
        return Ok(());
    }
    run("cargo", &["install", "rokit", "--locked"])?;
    run("rokit", &["self-install"])
}

pub fn ensure_projects_folder(root: &Path) -> Result<()> {
    if root.exists() {
        ui::ok(&format!("{} already exists", root.display()));
        return Ok(());
    }
    fs::create_dir_all(root)?;
    ui::ok(&format!("created {}", root.display()));
    Ok(())
}
