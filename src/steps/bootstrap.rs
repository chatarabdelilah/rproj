use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::catalog::tool_catalog::{ToolEntry, ToolKind};
use crate::steps::{probe, run};

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
    println!("\n> winget {}", args.join(" "));
    let output = Command::new("winget")
        .args(args)
        .output()
        .context("failed to spawn `winget`")?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if output.status.success() {
        return Ok(());
    }

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if combined.contains("Installer hash does not match") {
        bail!(
            "winget install --id {winget_id} failed: installer hash does not match.\n\
             This is a known upstream winget-pkgs issue, not a bug in rproj - it happens \
             when a vendor's installer self-updates behind a static download URL (Roblox's \
             installers are a common example) faster than the winget manifest's pinned \
             hash gets refreshed. Options:\n\
             - Re-run `rproj setup` later; winget-pkgs bots usually catch up within days\n\
             - Install it directly for now: https://www.roblox.com/create\n\
             - Or, as an admin, run `winget settings --enable InstallerHashOverride` once, \
             then retry with the integrity check bypassed (only if you trust the source)"
        );
    }
    bail!("`winget {}` exited with {}", args.join(" "), output.status);
}

/// Ensures Rokit itself is present. Rokit isn't on winget - its own docs
/// specify `cargo install rokit --locked` then `rokit self-install`.
pub fn ensure_rokit() -> Result<()> {
    if probe("rokit", &["--version"]) {
        println!("check: rokit already installed");
        return Ok(());
    }
    run("cargo", &["install", "rokit", "--locked"])?;
    run("rokit", &["self-install"])
}

pub fn ensure_projects_folder(root: &Path) -> Result<()> {
    if root.exists() {
        println!("check: {} already exists", root.display());
        return Ok(());
    }
    fs::create_dir_all(root)?;
    println!("created {}", root.display());
    Ok(())
}
