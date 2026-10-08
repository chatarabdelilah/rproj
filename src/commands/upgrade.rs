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
//! 1. **Only files rproj generates.** `stylua.toml`,
//!    `wally.toml`, `rokit.toml` and everything under `src/` are yours -
//!    they're seeded once and then edited by hand, so upgrading them would
//!    throw away real work.
//!    The default Rojo document is preserved except for Jest's devPackages mount.
//! 2. **`selene.toml` is merged, not replaced**, and only for the keys
//!    whose correct value follows from the project's composition (`std`
//!    from TestEZ, `mixed_table` from the UI library, `exclude` from the
//!    package workflow). Lint levels you chose yourself are left alone.
//!    See `catalog::tool_settings::checked_toml_merge`.
//!
//! Nothing is written until the list of changes has been shown and
//! confirmed.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, ErrorKind, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use serde_json::json;

use crate::catalog::quality_checks::{ci_workflow, render_check};
use crate::catalog::tool_settings::{self, SettingKind, SettingSpec};
use crate::catalog::wally_packages;
use crate::config::PackageWorkflow;
use crate::graph::{ProjectGraph, TestRunner};
use crate::steps::{gitignore, jest, quality, testez, vscode};
use crate::ui;

mod diff;
mod review;

pub(super) enum ReviewOutcome {
    Apply(PreparedUpgrade),
    Cancelled { home: bool },
    UpToDate(PreparedUpgrade),
}

pub(super) struct PreparedUpgrade {
    project_dir: PathBuf,
    plan: UpgradePlan,
}

impl PreparedUpgrade {
    pub(super) fn execute(self) -> Result<()> {
        self.summary();
        if self.plan.rewrites.is_empty() {
            self.plan.verify(&self.project_dir)?;
            if self.plan.skipped.is_empty() {
                ui::ok("already up to date - no upgrade changes needed");
            } else {
                ui::skip("no applicable upgrade changes; skipped files were left unchanged");
            }
            Ok(())
        } else {
            self.plan.apply(&self.project_dir)
        }
    }

    fn summary(&self) {
        for note in &self.plan.skipped {
            ui::skip(note);
        }
        if self.plan.rewrites.is_empty() {
            return;
        }
        println!("\nThese generated files would change:\n");
        for rewrite in &self.plan.rewrites {
            let verb = if rewrite.creating { "create" } else { "update" };
            println!("  {verb} {}", rewrite.relative);
            ui::detail(rewrite.reason);
        }
        println!(
            "\nNot touched: stylua.toml, wally.toml, rokit.toml, src/.\n\
             Jest projects add only the devPackages mount to default.project.json.\n\
             Existing test imports must use ReplicatedStorage.devPackages.\n\
             Your own selene lint levels are kept; only std, mixed_table and exclude are set.\n"
        );
    }
}

/// One file that would change, with the reason and its new contents.
struct Rewrite {
    relative: String,
    contents: String,
    reason: &'static str,
    creating: bool,
}

#[derive(Default)]
struct UpgradePlan {
    rewrites: Vec<Rewrite>,
    originals: BTreeMap<String, Option<String>>,
    skipped: Vec<&'static str>,
}

impl UpgradePlan {
    fn read(&mut self, project_dir: &Path, relative: &str) -> Result<Option<String>> {
        if let Some(original) = self.originals.get(relative) {
            return Ok(original.clone());
        }
        let original = read_optional(&project_dir.join(relative))?;
        self.originals.insert(relative.to_owned(), original.clone());
        Ok(original)
    }

    fn verify(&self, project_dir: &Path) -> Result<()> {
        // Check every target and input before the first write, so a conflict in
        // a later file cannot leave the earlier files upgraded.
        for (relative, original) in &self.originals {
            let path = project_dir.join(relative);
            ensure!(
                read_optional(&path)? == *original,
                "{} changed while upgrade was being reviewed. Nothing written; re-run `rproj upgrade` to review the current files.",
                path.display()
            );
        }
        Ok(())
    }

    fn apply(&self, project_dir: &Path) -> Result<()> {
        let _guard = crate::background_watch::mutation_guard(Some(project_dir), false)?;
        self.verify(project_dir)?;
        let staged: Vec<_> = self
            .rewrites
            .iter()
            .map(|rewrite| {
                let path = project_dir.join(&rewrite.relative);
                let existing = self.originals[&rewrite.relative].is_some();
                crate::file_replace::stage(&path, rewrite.contents.as_bytes(), existing)
                    .with_context(|| {
                        format!(
                            "failed to prepare {}. No upgrade targets replaced.",
                            path.display()
                        )
                    })
            })
            .collect::<Result<_>>()?;
        // Staging can take time. Recheck the whole plan before any replacement.
        self.verify(project_dir)?;
        ui::section(&format!("Upgrading {}", project_dir.display()));
        for (saved, (rewrite, pending)) in self.rewrites.iter().zip(staged).enumerate() {
            let path = project_dir.join(&rewrite.relative);
            pending.persist(&path).map_err(|error| error.error).with_context(|| {
                format!("failed to save {}; {saved} earlier upgrade files were updated. Fix the error and re-run `rproj upgrade`.", path.display())
            })?;
            ui::ok(&format!("wrote {}", rewrite.relative));
        }
        Ok(())
    }
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn require_released_wpt(manifest: Option<&str>) -> Result<()> {
    let install = "From this project directory run `rokit add wally-package-types`, verify rokit.toml pins the official release at 1.7.0 or newer, then run `rproj upgrade`. Nothing written.";
    let update = "From this project directory run `rokit update wally-package-types`, verify the official stable version is 1.7.0 or newer, then run `rproj upgrade`. Nothing written.";
    let Some(manifest) = manifest else {
        bail!(
            "Wally CI requires the official wally-package-types 1.7.0 or newer in rokit.toml. {install}"
        );
    };
    let parsed: toml::Value = toml::from_str(manifest)
        .context("failed to parse rokit.toml before Wally CI upgrade. Repair the manifest and re-run `rproj upgrade`. Nothing written.")?;
    let Some(spec) = parsed
        .get("tools")
        .and_then(|tools| tools.get("wally-package-types"))
    else {
        bail!("Wally CI requires wally-package-types in rokit.toml. {install}");
    };
    let version = spec
        .as_str()
        .and_then(|spec| spec.rsplit_once('@'))
        .filter(|(source, _)| source.eq_ignore_ascii_case("JohnnyMorganz/wally-package-types"))
        .and_then(|(_, version)| stable_version(version));
    ensure!(
        version.is_some_and(|version| version >= [1, 7, 0]),
        "Wally CI requires the official stable wally-package-types 1.7.0 or newer; rokit.toml has an older or unverifiable pin. {update}"
    );
    Ok(())
}

fn stable_version(version: &str) -> Option<[u64; 3]> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let version = if let Some((version, metadata)) = version.split_once('+') {
        if !metadata.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
        }) {
            return None;
        }
        version
    } else {
        version
    };
    if !version.split('.').all(|part| {
        !part.is_empty()
            && (part.len() == 1 || !part.starts_with('0'))
            && part.bytes().all(|ch| ch.is_ascii_digit())
    }) {
        return None;
    }
    let parts = version
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()
        .ok()?;
    parts.try_into().ok()
}

pub fn run(assume_yes: bool) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;
    run_in(&project_dir, assume_yes)
}

pub(super) fn run_in(project_dir: &Path, assume_yes: bool) -> Result<()> {
    let prepared = prepare(project_dir)?;
    if assume_yes || prepared.plan.rewrites.is_empty() {
        if !prepared.plan.rewrites.is_empty() {
            record_choice(true, true);
        }
        return prepared.execute();
    }
    ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        "Upgrade review requires an interactive terminal; use `rproj upgrade --yes` to apply without the viewer. Nothing written."
    );
    let decision = {
        let mut terminal = crate::tui::TerminalSession::enter()?;
        review::run(&mut terminal, &prepared.project_dir, &prepared.plan)?
    };
    match decision {
        review::Decision::Apply => {
            record_choice(true, false);
            prepared.execute()
        }
        review::Decision::Cancel { .. } => {
            record_choice(false, false);
            prepared.summary();
            ui::skip("nothing written");
            Ok(())
        }
    }
}

pub(super) fn open_in(
    terminal: &mut crate::tui::TerminalSession,
    project_dir: &Path,
) -> Result<ReviewOutcome> {
    crate::interrupt::check()?;
    let context = super::projects::ProjectContext::load(project_dir.to_owned());
    if let Err(reason) = context.availability(super::projects::ProjectAction::Upgrade) {
        bail!("{}: {reason}", project_dir.display());
    }
    crate::diagnostics::event(
        "project.action",
        format!("Upgrade: {}", project_dir.display()),
    );
    let prepared = prepare(project_dir)?;
    if prepared.plan.rewrites.is_empty() {
        prepared.plan.verify(project_dir)?;
        return Ok(ReviewOutcome::UpToDate(prepared));
    }
    match review::run(terminal, project_dir, &prepared.plan)? {
        review::Decision::Apply => {
            record_choice(true, false);
            Ok(ReviewOutcome::Apply(prepared))
        }
        review::Decision::Cancel { home } => {
            record_choice(false, false);
            Ok(ReviewOutcome::Cancelled { home })
        }
    }
}

fn record_choice(apply: bool, assumed: bool) {
    crate::diagnostics::event(
        "choice.upgrade",
        format!("apply={apply}; assumed={assumed}"),
    );
}

fn prepare(project_dir: &Path) -> Result<PreparedUpgrade> {
    let mut upgrade = UpgradePlan::default();
    if upgrade.read(project_dir, "default.project.json")?.is_none() {
        bail!(
            "no default.project.json here - `rproj upgrade` updates an existing project, \
             run it from inside one"
        );
    }
    // The package list is what decides most of these files, and guessing it
    // from what's on disk would be guessing.
    let Some(record) = upgrade.read(project_dir, "rproj.toml")? else {
        bail!(
            "no rproj.toml here - `rproj upgrade` needs the package list it records to know \
             what this project's config should say. Projects scaffolded by `rproj new` have one"
        );
    };
    let project: ProjectGraph = toml::from_str(&record).with_context(|| {
        format!(
            "failed to parse {}",
            project_dir.join("rproj.toml").display()
        )
    })?;

    let packages: BTreeSet<String> = project.packages.iter().cloned().collect();
    let workflow = project.package_workflow;
    let runner = project.test_runner();
    if !project.testing_is_compatible() {
        bail!(
            "Jest Roblox requires the Wally dependency workflow; repair rproj.toml before upgrading"
        );
    }
    let upgrade = plan(project_dir, &project, &packages, workflow, runner, upgrade)?;
    Ok(PreparedUpgrade {
        project_dir: project_dir.to_owned(),
        plan: upgrade,
    })
}

fn plan(
    project_dir: &Path,
    project: &ProjectGraph,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    runner: Option<TestRunner>,
    mut upgrade: UpgradePlan,
) -> Result<UpgradePlan> {
    // **Upgrade re-derives from the graph.** Every rewrite below is gated on
    // the project's own plan, so an upgrade cannot restore a file this
    // project never wanted: a project that declined CI does not silently
    // acquire a workflow because a newer rproj generates one, and a file
    // dropped at the summary stays dropped. Before the graph existed there
    // was nothing to ask - upgrade rewrote whatever it could render.
    let planned = project.maintenance_plan();
    let wants = |key: &str| planned.iter().any(|p| p.key == key);

    if workflow == PackageWorkflow::Wally && wants(".github/workflows/ci.yml") {
        // The tool pin is an upgrade input even though upgrade never rewrites it.
        let manifest = upgrade.read(project_dir, "rokit.toml")?;
        require_released_wpt(manifest.as_deref())?;
    }

    let testez_selected = runner == Some(TestRunner::TestEz);
    if wants("selene.toml") {
        let original = upgrade.read(project_dir, "selene.toml")?;
        let contents = selene_config(original.as_deref(), packages, workflow, testez_selected)
            .with_context(|| {
                format!(
                    "cannot upgrade {}; edit the file manually and re-run `rproj upgrade`",
                    project_dir.join("selene.toml").display()
                )
            })?;
        push(
            &mut upgrade,
            project_dir,
            "selene.toml",
            contents,
            "std, mixed_table and exclude follow this project's packages and workflow",
        )?;
    }

    if wants(".vscode/settings.json") {
        let original = upgrade.read(project_dir, ".vscode/settings.json")?;
        let settings = vscode::merge_settings_values(
            vscode::parse_settings(
                &project_dir.join(".vscode/settings.json"),
                original.as_deref().unwrap_or(""),
            )?,
            &vscode::project_settings(),
        )?;
        push(
            &mut upgrade,
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
            &mut upgrade,
            project_dir,
            ".lute/check.luau",
            contents,
            "the generated quality gate",
        )?;

        if wants(".github/workflows/ci.yml") {
            let has_server_packages =
                workflow == PackageWorkflow::Wally && wally_packages::has_server_realm(packages);
            push(
                &mut upgrade,
                project_dir,
                ".github/workflows/ci.yml",
                ci_workflow(
                    workflow,
                    has_server_packages,
                    runner,
                    project.jest_backend(),
                ),
                "the generated CI workflow",
            )?;
        }
    }

    if wants("testez.yml") {
        push(
            &mut upgrade,
            project_dir,
            "testez.yml",
            testez::TESTEZ_STD.to_string(),
            "selene's TestEZ standard library",
        )?;
    }
    if wants("testez-companion.toml") {
        push(
            &mut upgrade,
            project_dir,
            "testez-companion.toml",
            testez::companion_config(),
            "TestEZ Companion's test roots",
        )?;
    }

    if wants("jest.project.json") {
        let source = project_dir.join("default.project.json");
        let original = upgrade
            .read(project_dir, "default.project.json")?
            .context("default.project.json is missing")?;
        let mut production: serde_json::Value = serde_json::from_str(&original)
            .with_context(|| format!("failed to parse {}", source.display()))?;
        jest::ensure_dev_mount(&mut production)?;
        push(
            &mut upgrade,
            project_dir,
            "default.project.json",
            format!("{}\n", serde_json::to_string_pretty(&production)?),
            "development packages for Jest type analysis",
        )?;
        push(
            &mut upgrade,
            project_dir,
            jest::PROJECT_FILE,
            jest::project_contents(&production)?,
            "the generated test-only Rojo project",
        )?;
    }
    if wants("jest.config.json") {
        let original = upgrade.read(project_dir, jest::CONFIG_FILE)?;
        push(
            &mut upgrade,
            project_dir,
            jest::CONFIG_FILE,
            jest::merged_config_text(
                &project_dir.join(jest::CONFIG_FILE),
                original.as_deref(),
                project.jest_backend(),
            )?,
            "Jest runner paths managed by rproj; other options are kept",
        )?;
    }

    // Preserve existing housekeeping eligibility, but review and protect these
    // merges exactly like the composition-derived replacements above.
    let original = upgrade.read(project_dir, ".gitignore")?;
    if let Some(contents) = gitignore::planned_entries(original.as_deref().unwrap_or("")) {
        push(
            &mut upgrade,
            project_dir,
            ".gitignore",
            contents,
            "missing generated-output ignore entries; your existing entries are kept",
        )?;
    }
    let original = upgrade.read(project_dir, ".luaurc")?;
    let update = quality::planned_luaurc(original.as_deref())?;
    plan_luaurc(&mut upgrade, project_dir, ".luaurc", update)?;
    if testez_selected {
        let original = upgrade.read(project_dir, "tests/.luaurc")?;
        let update = testez::planned_tests_luaurc(original.as_deref())?;
        plan_luaurc(&mut upgrade, project_dir, "tests/.luaurc", update)?;
    }
    Ok(upgrade)
}

fn plan_luaurc(
    upgrade: &mut UpgradePlan,
    project_dir: &Path,
    relative: &'static str,
    update: quality::LuaurcUpdate,
) -> Result<()> {
    match update {
        quality::LuaurcUpdate::Unchanged => {}
        quality::LuaurcUpdate::Unparseable => upgrade.skipped.push(if relative == ".luaurc" {
            ".luaurc exists but couldn't be parsed, leaving it alone"
        } else {
            "tests/.luaurc exists but couldn't be parsed, leaving it alone"
        }),
        quality::LuaurcUpdate::Write(contents) => push(
            upgrade,
            project_dir,
            relative,
            contents,
            if relative == ".luaurc" {
                "strict language mode when absent; your aliases and other settings are kept"
            } else {
                "TestEZ globals when absent; your other settings are kept"
            },
        )?,
    }
    Ok(())
}

/// `selene.toml` with only the composition-derived keys updated.
fn selene_config(
    existing: Option<&str>,
    packages: &BTreeSet<String>,
    workflow: PackageWorkflow,
    testez_selected: bool,
) -> Result<String> {
    let scaffolded;
    let existing = match existing {
        Some(existing) => existing,
        None => {
            let config =
                tool_settings::default_toml("selene", &overrides(packages, testez_selected))
                    .context("selene missing from the catalog")?;
            scaffolded = tool_settings::insert_top_level(&config, &vendored_exclude(workflow));
            // Creation must use the same merge as later runs to avoid a second review.
            &scaffolded
        }
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

    let mut updated = tool_settings::checked_toml_merge(existing, &managed)?;
    let required = vendored_excludes(workflow);
    if !required.is_empty() {
        let parsed: toml::Value =
            toml::from_str(existing).context("failed to parse selene.toml")?;
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
        updated = tool_settings::checked_toml_merge(&updated, &[(&exclude, json!(excludes))])?;
    }
    Ok(updated)
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
    upgrade: &mut UpgradePlan,
    project_dir: &Path,
    relative: &str,
    contents: String,
    reason: &'static str,
) -> Result<()> {
    let current = upgrade.read(project_dir, relative)?;
    if current.as_deref() == Some(contents.as_str()) {
        return Ok(());
    }
    upgrade.rewrites.push(Rewrite {
        relative: relative.to_string(),
        contents,
        reason,
        creating: current.is_none(),
    });
    Ok(())
}
