use anyhow::{bail, Context, Result};

use crate::config::ProjectConfig;
use crate::steps::{git, rojo, toolchain, wally};

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
    // Both package workflows leave the vendored code out of the repo -
    // Wally's `Packages/` is gitignored, and a submodule's directory is
    // empty until it's fetched - so a fresh clone needs each workflow's
    // own restore step before anything reads those paths. Only the Wally
    // half was here, which made `rproj watch` work on a cloned Wally
    // project and fail on a cloned submodule one.
    git::sync_submodules(&project_dir)?;
    // `sync`, never a bare `wally install`: an install rewrites every link
    // file in packages/ without the `export type` lines, so watching used
    // to silently strip the types off every package on each run. See
    // `steps::wally::sync`.
    if project_dir.join("wally.toml").exists() {
        wally::sync(&project_dir)?;
    }

    println!("\nWatching for changes - press Ctrl+C to stop.");
    rojo::watch_sourcemap(&project_dir)

}
