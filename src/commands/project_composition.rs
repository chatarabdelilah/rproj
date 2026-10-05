use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::creation::ProjectEdit;
use crate::catalog::wally_packages;
use crate::config::{PackageWorkflow, project_file};
use crate::graph::TestRunner;
use crate::steps::{git, jest, modules, rojo, testez, toolchain, wally};
use crate::ui;

pub fn apply(project_dir: &Path, edit: ProjectEdit) -> Result<()> {
    let _guard = crate::background_watch::mutation_guard(Some(project_dir), false)?;
    let record_path = project_file::path_in(project_dir);
    let current_source = std::fs::read_to_string(&record_path)
        .with_context(|| format!("failed to read {}", record_path.display()))?;
    if current_source != edit.source {
        bail!(
            "rproj.toml changed while the project editor was open; reopen Edit Packages & Capabilities"
        );
    }
    let current = project_file::load_from(project_dir)?.context("rproj.toml disappeared")?;
    if toml::to_string(&current)? != toml::to_string(&edit.original)? {
        bail!("rproj.toml no longer matches the composition that was opened");
    }
    if edit.updated.package_workflow != edit.original.package_workflow {
        bail!("project composition editing does not change the dependency workflow");
    }
    if !edit
        .original
        .packages
        .iter()
        .all(|package| edit.updated.packages.contains(package))
        || !edit
            .original
            .capability_keys()
            .iter()
            .all(|capability| edit.updated.capabilities.contains_key(capability))
    {
        bail!("existing project choices must be preserved; reopen the editor and add choices only");
    }
    if toml::to_string(&edit.original)? == toml::to_string(&edit.updated)? {
        ui::ok("project composition is unchanged");
        return Ok(());
    }
    if !project_dir.join("default.project.json").is_file() {
        bail!("default.project.json disappeared before project changes were applied");
    }
    if edit.updated.package_workflow == PackageWorkflow::Wally
        && !project_dir.join("wally.toml").is_file()
    {
        bail!("Wally project is missing wally.toml; restore it before adding packages");
    }

    project_file::save_to(&edit.updated, project_dir)?;
    ui::ok("updated rproj.toml");

    let saved_source = std::fs::read_to_string(&record_path)
        .with_context(|| format!("failed to read saved {}", record_path.display()))?;
    if let Err(error) = apply_recorded(project_dir, &edit.updated) {
        let current_source = std::fs::read_to_string(&record_path).with_context(|| {
            format!(
                "failed to inspect {} after apply failure",
                record_path.display()
            )
        })?;
        if current_source == saved_source {
            std::fs::write(&record_path, &edit.source).with_context(|| {
                format!(
                    "failed to restore {} after apply failure",
                    record_path.display()
                )
            })?;
            return Err(error.context("project files were not fully updated; restored the original rproj.toml. Review git status/diff for partial changes to default.project.json, test files, rokit.toml, wally.toml, or modules/ before retrying"));
        }
        return Err(error.context(
            "project files were not fully updated; rproj.toml changed again, so it was not restored",
        ));
    }
    Ok(())
}

fn apply_recorded(project_dir: &Path, updated: &crate::graph::ProjectGraph) -> Result<()> {
    let packages: BTreeSet<String> = updated.packages.iter().cloned().collect();
    let runner = updated.test_runner();
    let testez_selected = runner == Some(TestRunner::TestEz);
    let has_server_packages = updated.package_workflow == PackageWorkflow::Wally
        && wally_packages::has_server_realm(&packages);
    rojo::ensure_project_mounts(
        project_dir,
        updated.package_workflow,
        testez_selected,
        has_server_packages,
    )?;

    if let Some(runner) = runner {
        match runner {
            TestRunner::TestEz => {
                testez::ensure_test_tree(project_dir, false)?;
                testez::ensure_tests_luaurc(project_dir)?;
            }
            TestRunner::JestRoblox => jest::ensure_test_tree(project_dir, false)?,
        }
    }

    let tools = updated.tools();
    if !tools.is_empty() {
        toolchain::ensure_rokit_init(project_dir)?;
        toolchain::add_selected_tools(project_dir, &tools)?;
    }

    match updated.package_workflow {
        PackageWorkflow::Wally => {
            wally::merge_selected(project_dir, &updated.packages)?;
        }
        PackageWorkflow::GitSubmodules => {
            let mut cloned = BTreeSet::new();
            for spec in modules::vendorable(&packages) {
                let submodule = spec.submodule.expect("vendorable package has a submodule");
                if cloned.insert(submodule.dir) {
                    git::add_submodule(project_dir, spec.git_repo, submodule.dir)?;
                }
            }
            modules::write_submodules_project(project_dir, &packages)?;
            modules::write_link_files(project_dir, &packages)?;
        }
        PackageWorkflow::None => {}
    }

    super::upgrade::run_in(project_dir, true)?;

    match updated.package_workflow {
        PackageWorkflow::Wally => {
            let project_file = if runner == Some(TestRunner::JestRoblox) {
                jest::PROJECT_FILE
            } else {
                "default.project.json"
            };
            wally::sync_for_project(project_dir, project_file)?;
        }
        PackageWorkflow::GitSubmodules | PackageWorkflow::None => {
            rojo::generate_sourcemap(project_dir)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProjectGraph;

    #[test]
    fn external_record_edits_are_refused_before_project_files_change() {
        let project = tempfile::tempdir().unwrap();
        let graph = ProjectGraph::default();
        project_file::save_to(&graph, project.path()).unwrap();
        let source = std::fs::read_to_string(project_file::path_in(project.path())).unwrap();
        std::fs::write(
            project_file::path_in(project.path()),
            format!("# external edit\n{source}"),
        )
        .unwrap();
        let error = apply(
            project.path(),
            ProjectEdit {
                source,
                original: graph.clone(),
                updated: graph,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("changed while the project editor was open"));
    }

    #[test]
    fn a_post_save_failure_restores_the_exact_original_record() {
        let project = tempfile::tempdir().unwrap();
        let original = ProjectGraph::default();
        project_file::save_to(&original, project.path()).unwrap();
        let path = project_file::path_in(project.path());
        let source = format!(
            "# preserve this exact source on failure\n{}",
            std::fs::read_to_string(&path).unwrap()
        );
        std::fs::write(&path, &source).unwrap();
        std::fs::write(project.path().join("wally.toml"), "").unwrap();
        std::fs::write(project.path().join("default.project.json"), "not json").unwrap();

        let mut updated = original.clone();
        updated.capabilities.insert("lint".into(), "selene".into());
        let error = apply(
            project.path(),
            ProjectEdit {
                source: source.clone(),
                original,
                updated,
            },
        )
        .unwrap_err()
        .to_string();

        assert!(
            error.contains("restored the original rproj.toml"),
            "{error}"
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), source);
    }
}
