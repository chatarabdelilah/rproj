use anyhow::{Context, Result, bail};

use crate::config::project_file;
use crate::graph::TestRunner;
use crate::steps::{git, jest, rojo, toolchain, wally};

pub fn run() -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;
    run_in(&project_dir)
}

pub(super) fn run_in(project_dir: &std::path::Path) -> Result<()> {
    if !project_dir.join("default.project.json").exists() {
        bail!(
            "no default.project.json here - `rproj watch` resumes an existing project, \
             run it from inside one (or `rproj new <name>` to create one)"
        );
    }

    let project = project_file::load_from(project_dir)?;
    let jest =
        project.as_ref().and_then(|graph| graph.test_runner()) == Some(TestRunner::JestRoblox);
    validate_jest_project(project.as_ref(), project_dir.join("wally.toml").exists())?;
    if let Some(project) = &project {
        println!("Packages: {}", project.packages.join(", "));
    }

    // Same idempotent steps whether this is a fresh `git clone` (tools/packages
    // not yet installed locally) or a project already being worked on - both
    // just converge to "everything the manifest asks for is present".
    if project_dir.join("rokit.toml").exists() {
        toolchain::sync_installed_tools(project_dir)?;
    }
    // Both package workflows leave the vendored code out of the repo -
    // Wally's `Packages/` is gitignored, and a submodule's directory is
    // empty until it's fetched - so a fresh clone needs each workflow's
    // own restore step before anything reads those paths. Only the Wally
    // half was here, which made `rproj watch` work on a cloned Wally
    // project and fail on a cloned submodule one.
    git::sync_submodules(project_dir)?;
    // `sync`, never a bare `wally install`: an install rewrites every link
    // file in packages/ without the `export type` lines, so watching used
    // to silently strip the types off every package on each run. See
    // `steps::wally::sync`.
    if project_dir.join("wally.toml").exists() {
        if jest {
            jest::refresh_project(project_dir)?;
            wally::sync_for_project(project_dir, jest::PROJECT_FILE)?;
        } else {
            wally::sync(project_dir)?;
        }
    }

    println!("\nWatching for changes - press Ctrl+C to stop.");
    if jest {
        rojo::watch_sourcemap_from(project_dir, jest::PROJECT_FILE)
    } else {
        rojo::watch_sourcemap(project_dir)
    }
}

fn validate_jest_project(
    project: Option<&crate::graph::ProjectGraph>,
    has_wally: bool,
) -> Result<()> {
    let Some(project) = project.filter(|graph| graph.test_runner() == Some(TestRunner::JestRoblox))
    else {
        return Ok(());
    };
    if !project.testing_is_compatible() {
        bail!("Jest Roblox requires the Wally dependency workflow");
    }
    if !has_wally {
        bail!("Jest Roblox project is missing wally.toml; run `rproj upgrade`");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PackageWorkflow;
    use crate::graph::ProjectGraph;

    #[test]
    fn jest_watch_requires_its_wally_manifest() {
        let mut project = ProjectGraph {
            package_workflow: PackageWorkflow::Wally,
            ..Default::default()
        };
        project.choose("test", Some("jest-roblox"));
        let error = validate_jest_project(Some(&project), false)
            .unwrap_err()
            .to_string();
        assert!(error.contains("missing wally.toml"), "{error}");
    }
}
