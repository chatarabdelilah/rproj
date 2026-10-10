use anyhow::{Context, Result, bail};

use crate::config::project_file;
use crate::graph::TestRunner;
use crate::steps::{jest, rojo, toolchain, wally};

pub fn run() -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;
    run_in(&project_dir)
}

pub(super) fn run_in(project_dir: &std::path::Path) -> Result<()> {
    let _guard = crate::background_watch::mutation_guard(Some(project_dir), false)?;
    let project_file = prepare_in(project_dir)?;
    println!("\nWatching for changes - press Ctrl+C to stop.");
    if project_file == "default.project.json" {
        rojo::watch_sourcemap(project_dir)
    } else {
        rojo::watch_sourcemap_from(project_dir, project_file)
    }
}

pub(crate) fn prepare_in(project_dir: &std::path::Path) -> Result<&'static str> {
    crate::interrupt::check()?;
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
    println!("{}", package_summary(project_dir)?);

    // Same idempotent steps whether this is a fresh `git clone` (tools/packages
    // not yet installed locally) or a project already being worked on - both
    // just converge to "everything the manifest asks for is present".
    if project_dir.join("rokit.toml").exists() {
        toolchain::sync_installed_tools(project_dir)?;
    }
    crate::interrupt::check()?;
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

    crate::interrupt::check()?;
    Ok(if jest {
        jest::PROJECT_FILE
    } else {
        "default.project.json"
    })
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

fn package_summary(project_dir: &std::path::Path) -> Result<String> {
    let snapshot = rproj_core::inspect(project_dir)?;
    match snapshot.files.wally.state {
        rproj_core::FileState::Missing => Ok("Packages: no wally.toml".into()),
        rproj_core::FileState::Invalid | rproj_core::FileState::Unreadable => {
            let detail = snapshot
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.path == snapshot.files.wally.path)
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            bail!("cannot read Wally dependency inventory: {detail}")
        }
        rproj_core::FileState::Ready if snapshot.packages.is_empty() => {
            Ok("Packages: none declared in wally.toml".into())
        }
        rproj_core::FileState::Ready => Ok(format!(
            "Packages: {}",
            snapshot
                .packages
                .iter()
                .map(|package| format!(
                    "{} [{}] ({})",
                    package.alias,
                    package.realm.section(),
                    package.declaration
                ))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
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

    #[test]
    fn package_summary_reads_all_wally_realms_without_rewriting_saved_choices() {
        let directory = tempfile::tempdir().unwrap();
        let saved = "packages = ['old-selection']\n";
        std::fs::write(directory.path().join("rproj.toml"), saved).unwrap();
        std::fs::write(directory.path().join("wally.toml"), "[package]\nname = 'owner/game'\nversion = '0.1.0'\nrealm = 'shared'\n[dependencies]\nCustom = 'owner/custom@1'\n[server-dependencies]\nStorage = 'owner/storage@2'\n[dev-dependencies]\nJest = 'roblox/jest@3.20.1'\n").unwrap();
        let summary = package_summary(directory.path()).unwrap();
        assert!(
            summary.contains("Custom [dependencies] (owner/custom@1)"),
            "{summary}"
        );
        assert!(
            summary.contains("Storage [server-dependencies]"),
            "{summary}"
        );
        assert!(summary.contains("Jest [dev-dependencies]"), "{summary}");
        assert!(!summary.contains("old-selection"), "{summary}");
        assert_eq!(
            std::fs::read_to_string(directory.path().join("rproj.toml")).unwrap(),
            saved
        );
    }

    #[test]
    fn malformed_manifest_is_not_reported_as_no_dependencies() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("wally.toml"), "[dependencies").unwrap();
        assert!(
            package_summary(directory.path())
                .unwrap_err()
                .to_string()
                .contains("Wally dependency inventory")
        );
    }
}
