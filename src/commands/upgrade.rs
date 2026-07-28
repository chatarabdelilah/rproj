//! `rproj upgrade` - bring an existing project's generated files back in
//! line with what this version of rproj would scaffold today.
//!
//! Every `ensure_*` step in the scaffold skips a file that already exists,
//! which is right for `rproj new` (running it twice must not clobber your
//! work) and leaves existing projects stranded: when a scaffold default
//! changes - selene's `mixed_table` waiver for Vide, `files.eol`, the
//! un-deprecated luau-lsp setting names - a project made yesterday keeps
//! the old, broken version forever and nothing says so.
//!
//! Two rules keep this from being destructive:
//!
//! 1. **Only files rproj generates.** `stylua.toml`, `default.project.json`,
//!    `wally.toml`, `rokit.toml` and everything under `src/` are yours -
//!    they're seeded once and then edited by hand, so upgrading them would
//!    throw away real work.
//! 2. **`selene.toml` is merged, not replaced**, and only for the keys
//!    whose correct value follows from the project's composition (`std`
//!    from TestEZ, `mixed_table` from the UI library, `exclude` from the
//!    package workflow). Lint levels you chose yourself are left alone.
//!    See `catalog::tool_settings::merge_toml`.
//!
//! Nothing is written until the list of changes has been shown and
//! confirmed.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use inquire::Confirm;
use serde_json::json;

use crate::catalog::quality_checks::{ci_workflow, render_check};
use crate::catalog::tool_settings::{self, SettingSpec};
use crate::catalog::wally_packages;
use crate::config::{PackageWorkflow, ProjectConfig};
use crate::steps::{gitignore, quality, testez, vscode};
use crate::ui;

/// One file that would change, with the reason and its new contents.
struct Rewrite {
    relative: String,
    contents: String,
    reason: &'static str,
    creating: bool,
}

pub fn run(assume_yes: bool) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;

    if !project_dir.join("default.project.json").exists() {
        bail!(
            "no default.project.json here - `rproj upgrade` updates an existing project, \
             run it from inside one"
        );
    }
    // The package list is what decides most of these files, and guessing it
    // from what's on disk would be guessing.
    let Some(project) = ProjectConfig::load_from(&project_dir)? else {
        bail!(
            "no rproj.toml here - `rproj upgrade` needs the package list it records to know \
             what this project's config should say. Projects scaffolded by `rproj new` have one"
        );
    };

    let packages: BTreeSet<String> = project.packages.iter().cloned().collect();
    let workflow = project.package_workflow;
    let testez_selected = packages.contains("testez");

    let rewrites = plan(&project_dir, &project, &packages, workflow, testez_selected)?;

    if rewrites.is_empty() {
        ui::ok("already up to date - every generated file matches this version of rproj");
    } else {
        println!("\nThese generated files would change:\n");
        for rewrite in &rewrites {
            let verb = if rewrite.creating { "create" } else { "update" };
            println!("  {verb} {}", rewrite.relative);
            ui::detail(rewrite.reason);
        }
        println!(
            "\nNot touched: stylua.toml, default.project.json, wally.toml, rokit.toml, src/.\n\
             Your own selene lint levels are kept; only std, mixed_table and exclude are set.\n"
        );

        if !assume_yes && !Confirm::new("Apply these changes?").with_default(true).prompt()? {
            ui::skip("nothing written");
            return Ok(());
        }

        ui::section(&format!("Upgrading {}", project_dir.display()));
        for rewrite in &rewrites {
            let path = project_dir.join(&rewrite.relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &rewrite.contents)
                .with_context(|| format!("failed to write {}", path.display()))?;
            ui::ok(&format!("wrote {}", rewrite.relative));
        }
    }

    // These merge rather than replace and are no-ops when nothing is
    // missing, so they run either way rather than being planned.
    gitignore::ensure_entries(&project_dir)?;
    quality::ensure_luaurc(&project_dir)?;
    if testez_selected {
        testez::ensure_tests_luaurc(&project_dir)?;
    }
    Ok(())
}

fn plan(
    project_dir: &Path,
    project: &ProjectConfig,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    testez_selected: bool,
) -> Result<Vec<Rewrite>> {
    let mut rewrites = Vec::new();

    if let Some(contents) = selene_config(project_dir, packages, workflow, testez_selected)? {
        push(&mut rewrites, project_dir, "selene.toml", contents, "std, mixed_table and exclude follow this project's packages and workflow")?;
    }

    let settings = vscode::merged_settings(project_dir, &vscode::project_settings(workflow))?;
    push(&mut rewrites, project_dir, ".vscode/settings.json", settings, "editor settings rproj manages; your other keys are kept")?;

    // The gate script names the tools this project pinned, so it has to be
    // rebuilt from the same list - not from what the machine has today.
    if let Some(contents) = render_check(&project.tools_at_creation, testez_selected) {
        push(&mut rewrites, project_dir, ".lute/check.luau", contents, "the generated quality gate")?;

        let has_server_packages =
            workflow == PackageWorkflow::Wally && wally_packages::has_server_realm(packages);
        push(&mut rewrites, project_dir, ".github/workflows/ci.yml", ci_workflow(workflow, has_server_packages), "the generated CI workflow")?;
    }

    if testez_selected {
        push(&mut rewrites, project_dir, "testez.yml", testez::TESTEZ_STD.to_string(), "selene's TestEZ standard library")?;
        push(&mut rewrites, project_dir, "testez-companion.toml", testez::companion_config(), "TestEZ Companion's test roots")?;
    }

    Ok(rewrites)
}

/// `selene.toml` with only the composition-derived keys updated, or `None`
/// if it already says the right things.
fn selene_config(
    project_dir: &Path,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    testez_selected: bool,
) -> Result<Option<String>> {
    let path = project_dir.join("selene.toml");
    let Ok(existing) = fs::read_to_string(&path) else {
        // No selene.toml at all: fall through to the full scaffolded file.
        return Ok(tool_settings::default_toml("selene", &overrides(packages, testez_selected))
            .map(|config| tool_settings::insert_top_level(&config, vendored_exclude(workflow))));
    };

    let tool = tool_settings::find("selene").context("selene missing from the catalog")?;
    let managed: Vec<(&SettingSpec, serde_json::Value)> = overrides(packages, testez_selected)
        .into_iter()
        .filter_map(|(key, value)| {
            tool.settings.iter().find(|s| s.key == key).map(|s| (s, json!(value)))
        })
        .collect();

    let mut updated = tool_settings::merge_toml(&existing, &managed);
    // `exclude` is not a catalog setting - `merge_toml` preserves it if it's
    // there and can't add it if it isn't.
    if !existing.lines().any(|line| line.trim_start().starts_with("exclude")) {
        updated = tool_settings::insert_top_level(&updated, vendored_exclude(workflow));
    }
    Ok((updated != existing).then_some(updated))
}

fn overrides(packages: &BTreeSet<String>, testez_selected: bool) -> Vec<(&'static str, &'static str)> {
    let mut overrides = vec![("std", if testez_selected { "roblox+testez" } else { "roblox" })];
    if wally_packages::allows_mixed_tables(packages) {
        overrides.push(("mixed_table", "allow"));
    }
    overrides
}

fn vendored_exclude(workflow: PackageWorkflow) -> &'static str {
    match workflow {
        PackageWorkflow::Wally => r#"exclude = ["Packages/**", "ServerPackages/**"]"#,
        PackageWorkflow::GitSubmodules => r#"exclude = ["modules/submodules/**"]"#,
    }
}

/// Adds a rewrite only when the file's contents would actually change.
fn push(
    rewrites: &mut Vec<Rewrite>,
    project_dir: &Path,
    relative: &str,
    contents: String,
    reason: &'static str,
) -> Result<()> {
    let path: PathBuf = project_dir.join(relative);
    let current = fs::read_to_string(&path).ok();
    if current.as_deref() == Some(contents.as_str()) {
        return Ok(());
    }
    rewrites.push(Rewrite {
        relative: relative.to_string(),
        contents,
        reason,
        creating: current.is_none(),
    });
    Ok(())
}
