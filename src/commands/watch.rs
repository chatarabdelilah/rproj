use anyhow::{bail, Context, Result};

use crate::config::ProjectConfig;
use crate::steps::{rojo, toolchain, wally};

pub fn run() -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;

    if !project_dir.join("default.project.json").exists() {
        bail!(
            "no default.project.json here - `rproj watch` resumes an existing project, \
             run it from inside one (or `rproj new <name>` to create one)"
        );
    }

    if let Some(project) = ProjectConfig::load_from(&project_dir)? {
        println!("Packages: {}", project.packages.join(", "));
    }

    // Same idempotent steps whether this is a fresh `git clone` (tools/packages
    // not yet installed locally) or a project already being worked on - both
    // just converge to "everything the manifest asks for is present".
    if project_dir.join("rokit.toml").exists() {
        toolchain::sync_installed_tools(&project_dir)?;
    }
    if project_dir.join("wally.toml").exists() {
        wally::wally_install(&project_dir)?;
    }

    println!("\nWatching for changes - press Ctrl+C to stop.");
    rojo::watch_sourcemap(&project_dir)

}
