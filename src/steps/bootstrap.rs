use std::fs;
use std::path::Path;

use anyhow::Result;

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
        ToolKind::SystemApp { winget_id } => run(
            "winget",
            &[
                "install",
                "--id",
                winget_id,
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements",
            ],
        ),
        _ => Ok(()),
    }
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
