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

use anyhow::{Context, Result, bail};
use inquire::Confirm;
use serde_json::json;

use crate::catalog::quality_checks::{ci_workflow, render_check};
use crate::catalog::tool_settings::{self, SettingKind, SettingSpec};
use crate::catalog::wally_packages;
use crate::config::{PackageWorkflow, project_file};
use crate::graph::{ProjectGraph, TestRunner};
use crate::steps::{gitignore, jest, quality, testez, vscode};
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
    run_in(&project_dir, assume_yes)
}

pub(super) fn run_in(project_dir: &Path, assume_yes: bool) -> Result<()> {
    if !project_dir.join("default.project.json").exists() {
        bail!(
            "no default.project.json here - `rproj upgrade` updates an existing project, \
             run it from inside one"
        );
    }
    // The package list is what decides most of these files, and guessing it
    // from what's on disk would be guessing.
    let Some(project) = project_file::load_from(project_dir)? else {
        bail!(
            "no rproj.toml here - `rproj upgrade` needs the package list it records to know \
             what this project's config should say. Projects scaffolded by `rproj new` have one"
        );
    };

    let packages: BTreeSet<String> = project.packages.iter().cloned().collect();
    let workflow = project.package_workflow;
    let runner = project.test_runner();
    if !project.testing_is_compatible() {
        bail!(
            "Jest Roblox requires the Wally dependency workflow; repair rproj.toml before upgrading"
        );
    }
    let rewrites = plan(project_dir, &project, &packages, workflow, runner)?;

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

        crate::diagnostics::event("prompt", "Apply upgrade changes?");
        let apply = assume_yes
            || Confirm::new("Apply these changes?")
                .with_default(true)
                .prompt()?;
        crate::diagnostics::event(
            "choice.upgrade",
            format!("apply={apply}; assumed={assume_yes}"),
        );
        if !apply {
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
    gitignore::ensure_entries(project_dir)?;
    quality::ensure_luaurc(project_dir)?;
    if runner == Some(TestRunner::TestEz) {
        testez::ensure_tests_luaurc(project_dir)?;
    }
    Ok(())
}

fn plan(
    project_dir: &Path,
    project: &ProjectGraph,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    runner: Option<TestRunner>,
) -> Result<Vec<Rewrite>> {
    let mut rewrites = Vec::new();

    // **Upgrade re-derives from the graph.** Every rewrite below is gated on
    // the project's own plan, so an upgrade cannot restore a file this
    // project never wanted: a project that declined CI does not silently
    // acquire a workflow because a newer rproj generates one, and a file
    // dropped at the summary stays dropped. Before the graph existed there
    // was nothing to ask - upgrade rewrote whatever it could render.
    let planned = project.maintenance_plan();
    let wants = |key: &str| planned.iter().any(|p| p.key == key);

    let testez_selected = runner == Some(TestRunner::TestEz);
    if wants("selene.toml")
        && let Some(contents) = selene_config(project_dir, packages, workflow, testez_selected)?
    {
        push(
            &mut rewrites,
            project_dir,
            "selene.toml",
            contents,
            "std, mixed_table and exclude follow this project's packages and workflow",
        )?;
    }

    if wants(".vscode/settings.json") {
        let settings = vscode::merged_settings(project_dir, &vscode::project_settings(workflow))?;
        push(
            &mut rewrites,
            project_dir,
            ".vscode/settings.json",
            settings,
            "editor settings rproj manages; your other keys are kept",
        )?;
    }

    // The gate script names the tools this project pinned, so it has to be
    // rebuilt from the same list - not from what the machine has today.
    if wants(".lute/check.luau")
        && let Some(contents) = render_check(&project.tools(), wants("tests"))
    {
        push(
            &mut rewrites,
            project_dir,
            ".lute/check.luau",
            contents,
            "the generated quality gate",
        )?;

        if wants(".github/workflows/ci.yml") {
            let has_server_packages =
                workflow == PackageWorkflow::Wally && wally_packages::has_server_realm(packages);
            push(
                &mut rewrites,
                project_dir,
                ".github/workflows/ci.yml",
                ci_workflow(workflow, has_server_packages, runner),
                "the generated CI workflow",
            )?;
        }
    }

    if wants("testez.yml") {
        push(
            &mut rewrites,
            project_dir,
            "testez.yml",
            testez::TESTEZ_STD.to_string(),
            "selene's TestEZ standard library",
        )?;
    }
    if wants("testez-companion.toml") {
        push(
            &mut rewrites,
            project_dir,
            "testez-companion.toml",
            testez::companion_config(),
            "TestEZ Companion's test roots",
        )?;
    }

    if wants("jest.project.json") {
        let source = project_dir.join("default.project.json");
        let production: serde_json::Value = serde_json::from_str(&fs::read_to_string(&source)?)
            .with_context(|| format!("failed to parse {}", source.display()))?;
        push(
            &mut rewrites,
            project_dir,
            jest::PROJECT_FILE,
            jest::project_contents(&production)?,
            "the generated test-only Rojo project",
        )?;
    }
    if wants("jest.config.json") {
        push(
            &mut rewrites,
            project_dir,
            jest::CONFIG_FILE,
            jest::merged_config(project_dir)?,
            "Jest runner paths managed by rproj; other options are kept",
        )?;
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
        return Ok(
            tool_settings::default_toml("selene", &overrides(packages, testez_selected)).map(
                |config| {
                    let exclude = vendored_exclude(workflow);
                    tool_settings::insert_top_level(&config, &exclude)
                },
            ),
        );
    };

    let tool = tool_settings::find("selene").context("selene missing from the catalog")?;
    let managed: Vec<(&SettingSpec, serde_json::Value)> = overrides(packages, testez_selected)
        .into_iter()
        .filter_map(|(key, value)| {
            tool.settings
                .iter()
                .find(|s| s.key == key)
                .map(|s| (s, json!(value)))
        })
        .collect();

    let mut updated = tool_settings::merge_toml(&existing, &managed);
    let required = vendored_excludes(workflow);
    if !required.is_empty() {
        let parsed: toml::Value = toml::from_str(&existing)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let mut excludes: Vec<String> = parsed
            .get("exclude")
            .map(|value| {
                value
                    .as_array()
                    .context("selene.toml `exclude` must be an array")?
                    .iter()
                    .map(|entry| {
                        entry
                            .as_str()
                            .map(str::to_string)
                            .context("selene.toml `exclude` entries must be strings")
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        for path in required {
            if !excludes.iter().any(|entry| entry == path) {
                excludes.push((*path).to_string());
            }
        }
        let exclude = SettingSpec {
            key: "exclude",
            description: "",
            section: None,
            kind: SettingKind::Bool { default: false },
        };
        updated = tool_settings::merge_toml(&updated, &[(&exclude, json!(excludes))]);
    }
    Ok((updated != existing).then_some(updated))
}

fn overrides(
    packages: &BTreeSet<String>,
    testez_selected: bool,
) -> Vec<(&'static str, &'static str)> {
    let mut overrides = vec![(
        "std",
        if testez_selected {
            "roblox+testez"
        } else {
            "roblox"
        },
    )];
    if wally_packages::allows_mixed_tables(packages) {
        overrides.push(("mixed_table", "allow"));
    }
    overrides
}

/// The selene `exclude` for whatever this project vendors. Empty when it
/// vendors nothing, so no dead key lands in the config.
fn vendored_excludes(workflow: PackageWorkflow) -> &'static [&'static str] {
    match workflow {
        PackageWorkflow::Wally => &["Packages/**", "ServerPackages/**", "DevPackages/**"],
        PackageWorkflow::GitSubmodules => &["modules/submodules/**"],
        PackageWorkflow::None => &[],
    }
}

fn vendored_exclude(workflow: PackageWorkflow) -> String {
    let values = vendored_excludes(workflow)
        .iter()
        .map(|path| format!(r#""{path}""#))
        .collect::<Vec<_>>()
        .join(", ");
    format!("exclude = [{values}]")
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
