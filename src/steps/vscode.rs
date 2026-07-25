use std::collections::HashSet;
use std::process::Command;

use anyhow::{Context, Result};

use crate::steps::run;

fn installed_extensions() -> HashSet<String> {
    Command::new("code")
        .arg("--list-extensions")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Installs any extension in `extension_ids` that isn't already present.
pub fn ensure_extensions(extension_ids: &[&str]) -> Result<()> {
    let installed = installed_extensions();
    for id in extension_ids {
        if installed.contains(&id.to_lowercase()) {
            println!("check: VS Code extension {id} already installed");
            continue;
        }
        run("code", &["--install-extension", id])
            .with_context(|| format!("failed to install VS Code extension {id}"))?;
    }
    Ok(())
}
