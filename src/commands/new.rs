use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{MultiSelect, Select};

use crate::catalog::tool_catalog;
use crate::catalog::wally_packages::{self, companions_for, Category, PackageSpec};
use crate::commands::provision;
use crate::catalog::artifacts::{self, Selections, Workflow};
use crate::config::{GlobalConfig, PackageWorkflow, ProjectConfig, SavedSetup};
use crate::steps::{
    blender, figma, git, gitattributes, gitignore, modules, quality, rojo, tarmac, testez,
    toolchain, vscode, wally,
};
use crate::ui;

pub fn run(name: &str, reconfigure: bool, like: Option<&str>, save_setup: Option<&str>) -> Result<()> {
    let mut config = GlobalConfig::load()?;

    let project_dir = config.projects_root()?.join(name);
    if project_dir.exists() {
        bail!("{} already exists", project_dir.display());
    }

    // Resolve --like before doing any work, so a typo'd setup name fails
    // immediately rather than after provisioning and half a scaffold.
    let saved = match like {
        Some(setup) => Some(load_setup(setup)?),
        None => None,
    };

    // Machine setup is a once-per-PC concern, so it's only asked when
    // there's nothing recorded yet (a genuinely fresh machine) or when
    // explicitly requested. Re-asking four multi-selects about winget
    // packages on every new project was pure friction: the answers are
    // almost never different, and `rproj new` is a project command.
    if reconfigure || !config.machine_configured() {
        if !config.machine_configured() {
            println!("Setting your machine up first - this only happens once.\n");
        }
        provision::run(&mut config)?;
        config.save()?;
    } else {
        ui::ok(&format!("machine ready: {}", config.machine_summary()));
        ui::detail("rproj setup to re-check, or rproj new --reconfigure to change");
    }

    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("failed to create {}", project_dir.display()))?;

    ui::section(&format!("Scaffolding {name}"));
    ui::detail(&project_dir.display().to_string());

    let (mode, packages, package_workflow) = match saved {
        Some((setup_name, setup)) => {
            let packages: BTreeSet<String> = setup.packages.into_iter().collect();
            ui::ok(&format!(
                "using saved setup `{setup_name}`: {}",
                if packages.is_empty() { "no packages".to_string() } else { packages.iter().cloned().collect::<Vec<_>>().join(", ") }
            ));
            (format!("like:{setup_name}"), packages, setup.package_workflow)
        }
        None => {
            let (mode, packages) = pick_composition()?;
            let workflow = pick_package_workflow(&packages)?;
            (mode.to_string(), packages, workflow)
        }
    };

    // Git submodules have no dependency resolution: the scaffold clones
    // exactly the list it's given. Wally does its own resolution, so its
    // manifest is left as the user picked it.
    let packages = match package_workflow {
        PackageWorkflow::Wally => packages,
        PackageWorkflow::GitSubmodules => {
            let resolved = wally_packages::with_dependencies(&packages);
            let added: Vec<&str> = resolved
                .iter()
                .filter(|k| !packages.contains(*k))
                .map(String::as_str)
                .collect();
            if !added.is_empty() {
                ui::ok(&format!("added required dependencies: {}", added.join(", ")));
            }
            resolved
        }
    };

    // Asked here rather than inside `scaffold`, so every prompt in this
    // command lives in one place and `scaffold` only ever acts on decisions
    // already made. A saved setup skips the packages question but still
    // asks this one - which files you want is a per-project choice, not
    // part of a package composition.
    let selected_packages: Vec<String> = packages.iter().cloned().collect();
    let project_tools = pick_project_tools(&config, package_workflow)?;
    let chosen_artifacts = pick_artifacts(&Selections {
        packages: &selected_packages,
        tools: &project_tools,
        apps: &config.selected_system_apps,
        extensions: &config.selected_vscode_extensions,
        workflow: match package_workflow {
            PackageWorkflow::Wally => Workflow::Wally,
            PackageWorkflow::GitSubmodules => Workflow::GitSubmodules,
        },
    })?;

    scaffold(
        &project_dir,
        name,
        &packages,
        package_workflow,
        &project_tools,
        &chosen_artifacts,
    )?;

    // Written here rather than in `scaffold`, because it records the mode
    // and the saved-setup name, which are `run`'s knowledge. Gated on the
    // same artifact key so declining it declines it - the cost being that
    // `rproj upgrade` then has nothing to read, which its own error says.
    let writes = |key: &str| chosen_artifacts.iter().any(|k| k == key);
    if writes("rproj.toml") {
        ProjectConfig {
            mode,
            package_workflow,
            packages: packages.iter().cloned().collect(),
            // What *this project* pins, not what the machine has selected.
            // `rproj upgrade` re-renders the check script from this field, so
            // recording the machine list gave a submodule project a script
            // with Wally steps in it - for tools the project never pinned.
            tools_at_creation: project_tools.clone(),
        }
        .save_to(&project_dir)?;
    } else {
        ui::skip("rproj.toml not written, so `rproj upgrade` won't know this project");
    }

    if let Some(setup_name) = save_setup {
        let setup = SavedSetup {
            packages: packages.iter().cloned().collect(),
            package_workflow,
        };
        setup.save(setup_name)?;
        ui::ok(&format!("saved setup `{setup_name}` - reuse with `rproj new <name> --like {setup_name}`"));
    }

    // A new project is exactly when someone doesn't yet know what to run,
    // so end with the next steps rather than just "done".
    // All plain double-width emoji: 🛡 and friends need a variation
    // selector to render as emoji at all, and land narrow without one.
    let (party, folder, eye, gate, book) = match ui::emoji_enabled() {
        true => ("🎉  ", "📁", "👀", "🧪", "📖"),
        false => ("", " ", " ", " ", " "),
    };
    println!(
        "\n{party}{name} is ready.\n\n\
         \x20 {folder}  cd {}\n\
         \x20 {eye}  rproj watch          start the dev loop (Rojo sourcemap watcher)\n\
         \x20 {gate}  lute run check       run the quality gate (types, lint, format)\n\
         \x20 {book}  rproj info <tool>    what a tool does, and the commands to use it\n",
        project_dir.display()
    );
    Ok(())
}

fn pick_composition() -> Result<(&'static str, BTreeSet<String>)> {
    let mode = Select::new(
        "How do you want to set up this project's packages?",
        vec![
            "Guided walkthrough - answer one question per category, with explanations (recommended for beginners)",
            "Expert checklist - pick anything, no hand-holding",
        ],
    )
    .prompt()?;

    if mode.starts_with("Guided") {
        Ok(("guided", pick_guided()?))
    } else {
        Ok(("expert", pick_expert()?))
    }
}

fn pick_package_workflow(packages: &BTreeSet<String>) -> Result<PackageWorkflow> {
    if packages.is_empty() {
        return Ok(PackageWorkflow::Wally);
    }

    // Some packages (the react-lua family) only ship a working module
    // through an npm/pnpm install step upstream - there's no subfolder of
    // their repo that resolves on its own, so raw git submodules can't
    // vendor them at all. Rather than offer a choice that would break for
    // this selection, go straight to Wally (the workflow that does work
    // for every catalog entry) and say why.
    //
    // Checked over the *transitive* closure, not just what was picked: a
    // package can be perfectly vendorable itself and still be unusable
    // because something it requires isn't. `reactReflex` is the real case -
    // it reaches for React, which upstream only ships through npm - and
    // before this it scaffolded happily into a submodule project and failed
    // at runtime in Studio, with no build error anywhere.
    let blocked = wally_packages::unvendorable_in_closure(packages);
    if !blocked.is_empty() {
        for (key, pulled_in_by) in &blocked {
            match pulled_in_by {
                Some(dependent) => println!(
                    "note: {dependent} requires {key}, which upstream only ships through an \
                     npm/pnpm install step - git submodules can't reproduce that."
                ),
                None => println!(
                    "note: {key} only ships a working module through an npm/pnpm install step \
                     upstream, which git submodules can't reproduce."
                ),
            }
        }
        println!("      Using Wally for this project instead.");
        return Ok(PackageWorkflow::Wally);
    }

    let choice = Select::new(
        "How do you want to pull in this project's packages?",
        vec![
            "Wally - the standard Roblox/Luau package manager (recommended)",
            "Git submodules - clone each package's own repo instead of using Wally",
        ],
    )
    .prompt()?;
    Ok(if choice.starts_with("Wally") {
        PackageWorkflow::Wally
    } else {
        PackageWorkflow::GitSubmodules
    })
}

fn pick_guided() -> Result<BTreeSet<String>> {
    let mut packages = BTreeSet::new();

    for category in Category::ALL {
        let choices: Vec<&PackageSpec> = wally_packages::in_category(category)
            .filter(|p| p.primary_choice)
            .collect();
        if choices.is_empty() {
            continue;
        }

        let options: Vec<String> = choices
            .iter()
            .map(|p| ui::option_line(p.key, p.description, p.maintenance.short_badge()))
            .collect();
        // Just the category name. The question is implied by a picker, and
        // the answer line reads as `State management: reflex` rather than
        // `State management: which do you want? reflex`.
        let prompt = format!("{}:", category.label());

        if category.allows_multiple() {
            let selected = MultiSelect::new(&prompt, options)
                .with_help_message(ui::MULTISELECT_HELP)
                .with_formatter(&ui::compact_multi_answer)
                .prompt()?;
            for spec in &choices {
                if selected.iter().any(|s| ui::option_is(s, spec.key)) {
                    packages.insert(spec.key.to_string());
                }
            }
        } else {
            let mut options = options;
            options.push("none".to_string());
            let picked = Select::new(&prompt, options).with_formatter(&ui::compact_select_answer).prompt()?;
            if picked != "none"
                // Matched on the full "key - " prefix, not a bare
                // starts_with: two keys where one prefixes the other
                // (react/reactRoblox, charm/charmSync) would otherwise
                // resolve to whichever the catalog happened to list first.
                && let Some(spec) = choices.iter().find(|p| ui::option_is(&picked, p.key))
            {
                packages.insert(spec.key.to_string());
            }
        }
    }

    add_companions(&mut packages);
    Ok(packages)
}

fn pick_expert() -> Result<BTreeSet<String>> {
    let options: Vec<String> = wally_packages::PACKAGES
        .iter()
        .map(|p| {
            format!(
                "{} - {} [{}] ({})",
                p.key,
                p.description,
                p.category.label(),
                p.maintenance.short_badge()
            )
        })
        .collect();

    let selected = MultiSelect::new("Pick every package this project needs", options)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    Ok(wally_packages::PACKAGES
        .iter()
        .filter(|p| selected.iter().any(|s| ui::option_is(s, p.key)))
        .map(|p| p.key.to_string())
        .collect())
}


/// Which CLI tools this project pins in its own `rokit.toml`.
///
/// This used to be no question at all: every project pinned every tool the
/// machine had selected. That is the wrong default in both directions - it
/// pins tools a project will never run, and it made the *files* question
/// incoherent, because five artifacts follow from which tools are pinned and
/// the answer was always "all of them".
///
/// **Pinning is a reproducibility choice, not a functional one**, which is
/// why nothing here is entailed and every entry stays a checkbox. Every tool
/// also resolves from rokit's global manifest (provisioning adds them there),
/// so a project with no `rokit.toml` still runs `rojo` and `selene` fine on
/// this machine - it just doesn't promise a teammate the same versions. That
/// is a preference, the same class as `stylua.toml`, so it is offered rather
/// than decided.
///
/// Pre-checked, so pressing enter keeps the previous behaviour. `←` clears
/// them all, which is the one keystroke that makes "just the Rojo basics"
/// reachable without changing machine-wide setup.
fn pick_project_tools(config: &GlobalConfig, workflow: PackageWorkflow) -> Result<Vec<String>> {
    let available = tools_for_workflow(config, workflow);
    if available.is_empty() {
        return Ok(Vec::new());
    }

    let entries: Vec<&'static tool_catalog::ToolEntry> = available
        .iter()
        .filter_map(|key| tool_catalog::find(key))
        .collect();
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let options: Vec<String> = entries
        .iter()
        .map(|t| ui::option_line(t.key, t.description, t.maintenance.short_badge()))
        .collect();
    let all: Vec<usize> = (0..options.len()).collect();

    let picked = MultiSelect::new("Tools to pin in this project", options)
        .with_default(&all)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    Ok(entries
        .iter()
        .filter(|t| picked.iter().any(|p| ui::option_is(p, t.key)))
        .map(|t| t.key.to_string())
        .collect())
}

/// Which optional files this project gets.
///
/// One prompt, not one per category: six extra questions to answer before a
/// project exists is worse than a single list. The category rides along as
/// the badge slot, which both groups the list visually and keeps
/// `option_line`'s width handling in charge of truncation.
///
/// **Only genuine choices reach the checkbox list.** Two kinds never do:
///
/// - *Mandatory* - a Rojo project without a source tree or a project file is
///   not a project.
/// - *Entailed* - an earlier answer already decided it. Picking six packages
///   and then being offered the chance to decline `wally.toml` was a question
///   with one sane answer, and answering it the other way made the six
///   packages silently evaporate.
///
/// Entailed artifacts are printed with the answer that caused them, before
/// the prompt. That is the part that makes this honest rather than merely
/// firmer: the user can see exactly which earlier answer to change, and
/// "just the Rojo basics" stays reachable by changing it.
fn pick_artifacts(selections: &Selections) -> Result<Vec<String>> {
    for line in settled_lines(&artifacts::entailed(selections)) {
        ui::detail(&line);
    }

    let offered = artifacts::offered(selections);
    if offered.is_empty() {
        // Everything left was either mandatory or already settled, so there
        // is no question to ask. Saying so beats a prompt with no options.
        return Ok(resolved_keys(selections, &[]));
    }

    let options: Vec<String> = offered
        .iter()
        .map(|a| ui::option_line(a.key, a.description, a.category.label()))
        .collect();
    let defaults: Vec<usize> = offered
        .iter()
        .enumerate()
        .filter(|(_, a)| a.default_selected)
        .map(|(i, _)| i)
        .collect();

    let picked = MultiSelect::new("Files to generate", options)
        .with_default(&defaults)
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    let ticked: Vec<String> = offered
        .iter()
        .filter(|a| picked.iter().any(|p| ui::option_is(p, a.key)))
        .map(|a| a.key.to_string())
        .collect();
    Ok(resolved_keys(selections, &ticked))
}

/// The block printed above the picker naming what earlier answers already
/// settled, and why.
///
/// Pure, and its own function, so the wording is a test rather than something
/// only visible by running the prompt. The "why" is the load-bearing half:
/// without it this is a tool announcing decisions, and with it the user can
/// see which answer to change.
fn settled_lines(settled: &[(&'static artifacts::Artifact, &'static str)]) -> Vec<String> {
    if settled.is_empty() {
        return Vec::new();
    }
    let width = settled.iter().map(|(a, _)| a.key.len()).max().unwrap_or(0);
    let mut lines = vec!["Already settled by your answers so far:".to_string()];
    lines.extend(
        settled
            .iter()
            .map(|(artifact, why)| format!("  {:<width$}  {why}", artifact.key)),
    );
    lines.push("To drop one of these, change the answer it follows from.".to_string());
    lines
}

/// Resolution happens here, not in the caller. It drops artifacts whose
/// dependencies were not ticked and adds back the entailed ones, and doing
/// it once means `run` and `scaffold` cannot disagree about what is written.
fn resolved_keys(selections: &Selections, ticked: &[String]) -> Vec<String> {
    artifacts::resolve(selections, ticked)
        .iter()
        .map(|a| a.key.to_string())
        .collect()
}

fn add_companions(packages: &mut BTreeSet<String>) {
    let primaries: Vec<String> = packages.iter().cloned().collect();
    let snapshot = packages.clone();
    for key in primaries {
        for companion in companions_for(&key, |k| snapshot.contains(k)) {
            packages.insert(companion.to_string());
        }
    }
}

fn scaffold(
    project_dir: &Path,
    name: &str,
    packages: &BTreeSet<String>,
    package_workflow: PackageWorkflow,
    project_tools: &[String],
    chosen_artifacts: &[String],
) -> Result<()> {
    // Which files this project gets, resolved from the selections rather
    // than decided by the order of the calls below. Everything after this
    // asks `writes(...)` instead of inventing its own condition - which is
    // how six artifacts came to be written whatever the user answered.
    // Already resolved by `pick_artifacts`, so this is a membership test
    // rather than a second resolution that could disagree with the first.
    let writes = |key: &str| chosen_artifacts.iter().any(|k| k == key);

    git::ensure_repo_init(project_dir)?;

    // Both calls sit behind the same gate. `rokit add` writes rokit.toml
    // itself, so pinning while the file was declined would have re-created it
    // anyway - the answer could not have been honoured, which is why the
    // artifact is entailed by having any tool to pin.
    //
    // Ungated, this was worse than untidy: on a machine with nine tools
    // selected and a project whose rokit.toml had been declined, `rokit add`
    // ran nine times against a directory with no manifest, rokit walked up to
    // the global one, and the project pinned **nothing** - silently, because
    // each failure is warned and continued. Verified on a real scaffold:
    // `t1/` contained no rokit.toml at all.
    if writes("rokit.toml") {
        toolchain::ensure_rokit_init(project_dir)?;
        toolchain::add_selected_tools(project_dir, project_tools)?;
    }
    if writes("selene.toml") {
        toolchain::ensure_selene_config(
            project_dir,
            packages.contains("testez"),
            package_workflow,
            wally_packages::allows_mixed_tables(packages),
        )?;
    }
    if writes("stylua.toml") {
        toolchain::ensure_stylua_config(project_dir)?;
    }

    let testez_selected = packages.contains("testez");
    // Only the Wally workflow has realms at all - a submodule checkout is
    // just files, mounted wholesale under modules/ regardless.
    let has_server_packages =
        package_workflow == PackageWorkflow::Wally && wally_packages::has_server_realm(packages);
    rojo::scaffold_project_json(
        project_dir,
        name,
        package_workflow,
        testez_selected,
        has_server_packages,
    )?;
    if writes("tests") {
        testez::ensure_test_folders(project_dir)?;
        // Part of the folder, not a separate decision: it declares TestEZ's
        // globals so the specs written a line above don't light up red in
        // the editor. It used to be its own catalog entry, i.e. a checkbox
        // asking whether you wanted the files you just asked for to work.
        testez::ensure_tests_luaurc(project_dir)?;
    }

    // default.project.json maps a $path (packages/ or modules/) that has to
    // exist before rojo will touch it at all - generating a sourcemap while
    // that folder is missing fails outright ("could not be turned into a
    // Roblox Instance"), not just incompletely. So the package install has
    // to happen, and the folder has to exist, before sourcemap generation -
    // which is why each workflow generates its own sourcemap at the end of
    // its own branch rather than sharing one call afterwards.
    match package_workflow {
        // The `writes` checks here look redundant against the `match` -
        // the artifacts require these exact workflows. They are what makes
        // the requirement live in one place rather than two, and what lets
        // a test assert every offered artifact has a gate.
        PackageWorkflow::Wally if writes("wally.toml") => {
            let package_name = format!("rproj/{}", slugify(name));
            let package_list: Vec<String> = packages.iter().cloned().collect();
            wally::ensure_wally_init(project_dir)?;
            wally::write_wally_toml(project_dir, &package_name, &package_list)?;
            // Installs, generates the sourcemap, and re-adds the exported
            // types a plain `wally install` leaves off. See `wally::sync`.
            wally::sync(project_dir)?;
        }
        PackageWorkflow::GitSubmodules if writes("modules") => {
            // Dedupe by target directory, not by package: monorepos like
            // littensy/charm back several catalog entries (charm,
            // charmSync, videCharm) from one clone.
            let mut cloned = BTreeSet::new();
            for spec in modules::vendorable(packages) {
                let sub = spec.submodule.expect("vendorable() filtered to Some");
                if cloned.insert(sub.dir) {
                    git::add_submodule(project_dir, spec.git_repo, sub.dir)?;
                }
            }
            // Both of these have to exist before the sourcemap runs below:
            // the nested project file is what stops Rojo from walking into
            // the vendored repos' own project files, and the link files are
            // what project code actually requires.
            modules::write_submodules_project(project_dir, packages)?;
            modules::write_link_files(project_dir, packages)?;
            // Now that modules/ exists, an initial sourcemap.json for
            // luau-lsp. (The Wally branch got its own inside `wally::sync`,
            // which needs it for wally-package-types.)
            if writes("sourcemap.json") {
                rojo::generate_sourcemap(project_dir)?;
            }
        }
        // Declined the dependency manifest, so there is nothing to install.
        // A project can legitimately want the tree and manage packages by
        // hand.
        _ => {
            if writes("sourcemap.json") {
                rojo::generate_sourcemap(project_dir)?;
            }
        }
    }

    // Quality gate. The check script is generated from the tools this
    // project actually selected, so it never invokes something that was
    // never installed; CI only lands if there's a script for it to run.
    // selene.toml says std = "roblox+testez", and without testez.yml to
    // resolve it selene prints "Could not find all standard library files"
    // and lints nothing at all - src/ included. Hence entailed by selene
    // being pinned rather than offered.
    if writes("testez.yml") {
        testez::ensure_selene_std(project_dir)?;
    }
    if writes("testez-companion.toml") {
        testez::ensure_companion_config(project_dir)?;
    }

    // Tells the editor which folders are vendored third-party code. Only
    // needed for the submodule workflow - luau-lsp already ignores Wally's
    // `_Index` by default, which is why that workflow never showed this.
    if writes(".vscode/settings.json") {
        vscode::ensure_project_settings(project_dir, package_workflow)?;
    }

    if writes(".luaurc") {
        quality::ensure_luaurc(project_dir)?;
    }
    // The same list the project's rokit.toml got: a check script must only
    // invoke tools this project actually pins, or CI fails on a command that
    // isn't installed.
    if writes(".lute/check.luau")
        && quality::ensure_check_script(project_dir, project_tools, testez_selected)?
    {
        // The workflow only exists to run the script, so the artifact model
        // requires one on the other - unticking the script drops the CI
        // file rather than leaving a workflow whose first command is
        // missing.
        if writes(".github/workflows/ci.yml") {
            quality::ensure_ci_workflow(project_dir, package_workflow, has_server_packages)?;
        }
        quality::lute_setup(project_dir)?;
    }

    if writes(".gitignore") {
        gitignore::ensure_entries(project_dir)?;
    }
    if writes(".gitattributes") {
        gitattributes::ensure_gitattributes(project_dir)?;
    }

    // Was `config.blender_enabled()`, which meant that selecting Blender once at
    // setup silently added a folder to every project afterwards. Now it is
    // an artifact requiring the app AND defaulting to off, so it is opt-in
    // per project.
    if writes("blender") {
        blender::scaffold_starter_scene(project_dir)?;
    }
    if writes("figma") {
        figma::scaffold_design_folder(project_dir)?;
    }
    // After figma/, so `asset_source` can see the exports folder and point
    // Tarmac at it rather than at a generic assets/ directory.
    if writes("tarmac.toml") {
        tarmac::ensure_config(project_dir, &slugify(name))?;
    }

    Ok(())
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// The machine-wide tool selection filtered to what this project's chosen
/// package workflow *could* use - the candidate list `pick_project_tools`
/// then offers.
///
/// Wally and wally-package-types are Wally-workflow-only: under git
/// submodules there is no wally.toml to install from and no package thunks
/// to retype, so offering them would be offering noise.
fn tools_for_workflow(config: &GlobalConfig, workflow: PackageWorkflow) -> Vec<String> {
    const WALLY_ONLY: &[&str] = &["wally", "wally-package-types"];
    config
        .selected_rokit_tools
        .iter()
        .filter(|key| workflow == PackageWorkflow::Wally || !WALLY_ONLY.contains(&key.as_str()))
        .cloned()
        .collect()
}

/// Resolves `--like`, failing with the list of what *is* available rather
/// than a bare "not found" - the names are user-chosen, so a typo is the
/// likely cause and the correction is right there.
///
/// Also re-applies the workflow guard that `pick_package_workflow` would
/// have applied interactively. A saved setup records an answer given at
/// some earlier point; a package can stop being vendorable since (or the
/// file can be edited by hand), and scaffolding a submodule project around
/// a package that has no vendorable source produces a broken tree.
fn load_setup(name: &str) -> Result<(String, SavedSetup)> {
    if let Some(mut setup) = SavedSetup::load(name)? {
        // Same transitive check as the interactive path: a saved setup can
        // name only vendorable packages and still be unbuildable because one
        // of them requires something that isn't.
        let selected: BTreeSet<String> = setup.packages.iter().cloned().collect();
        let blocked = wally_packages::unvendorable_in_closure(&selected);
        if setup.package_workflow == PackageWorkflow::GitSubmodules && !blocked.is_empty() {
            let reasons: Vec<String> = blocked
                .iter()
                .map(|(key, via)| match via {
                    Some(dependent) => format!("{key} (required by {dependent})"),
                    None => (*key).to_string(),
                })
                .collect();
            ui::warn(&format!(
                "setup `{name}` asks for git submodules, but {} can't be vendored that way - using Wally",
                reasons.join(", ")
            ));
            setup.package_workflow = PackageWorkflow::Wally;
        }

        let unknown: Vec<&str> = setup
            .packages
            .iter()
            .filter(|k| wally_packages::find(k).is_none())
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            ui::warn(&format!(
                "setup `{name}` names packages no longer in the catalog, skipping them: {}",
                unknown.join(", ")
            ));
            setup.packages.retain(|k| wally_packages::find(k).is_some());
        }

        return Ok((name.to_string(), setup));
    }
    let available = SavedSetup::list();
    if available.is_empty() {
        bail!(
            "no saved setup called `{name}` - none have been saved yet. \
             Add `--save-setup <name>` to a `rproj new` run to create one."
        );
    }
    bail!("no saved setup called `{name}`. Available: {}", available.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(tools: &[&str]) -> GlobalConfig {
        GlobalConfig {
            selected_rokit_tools: tools.iter().map(|t| t.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Choosing git submodules and then having Wally pinned into the
    /// project anyway is the workflow leaking across its own boundary.
    #[test]
    fn submodule_projects_do_not_pin_wally_tools() {
        let config = config_with(&["rojo", "wally", "wally-package-types", "selene"]);
        let tools = tools_for_workflow(&config, PackageWorkflow::GitSubmodules);
        assert_eq!(tools, vec!["rojo", "selene"]);
    }

    /// A machine that has never been provisioned must still get the
    /// questions; one that has must not be asked again.
    #[test]
    fn machine_is_only_configured_once() {
        let fresh = GlobalConfig::default();
        assert!(!fresh.machine_configured(), "a fresh config needs provisioning");

        let provisioned = GlobalConfig { last_checked: Some("123".into()), ..Default::default() };
        assert!(provisioned.machine_configured(), "already provisioned, don't re-ask");
    }

    /// The skip path prints this instead of the pickers, so it has to say
    /// something rather than being blank on a real config.
    #[test]
    fn machine_summary_describes_what_is_set_up() {
        let config = GlobalConfig {
            selected_system_apps: vec!["git".into(), "vscode".into()],
            selected_rokit_tools: vec!["rojo".into()],
            ..Default::default()
        };
        let summary = config.machine_summary();
        assert!(summary.contains("2 apps"), "{summary}");
        assert!(summary.contains("1 tools"), "{summary}");
        assert!(!summary.contains("plugins"), "empty groups shouldn't be listed: {summary}");
        assert_eq!(GlobalConfig::default().machine_summary(), "nothing selected");
    }

    /// **"Just the Rojo basics" has to survive entailment.**
    ///
    /// Found by checking the real machine config rather than reasoning: it
    /// has all nine rokit tools selected, so `rokit.toml` entailed by "any
    /// tool to pin" would have made a bare project unreachable without
    /// editing machine-wide setup. That is why which tools a project pins is
    /// now its own question - with none pinned, nothing is entailed and the
    /// minimum is still the minimum.
    #[test]
    fn pinning_no_tools_keeps_the_bare_project_reachable_on_a_full_machine() {
        let config = config_with(&[
            "rojo", "wally", "wally-package-types", "selene", "stylua", "lute", "luau-lsp-cli",
            "tarmac", "mantle",
        ]);
        // What the picker would offer, and what it returns if you press `←`.
        assert_eq!(tools_for_workflow(&config, PackageWorkflow::Wally).len(), 9);
        let pinned: Vec<String> = Vec::new();

        let (packages, apps, extensions) = (Vec::new(), Vec::new(), Vec::new());
        let selections = Selections {
            packages: &packages,
            tools: &pinned,
            apps: &apps,
            extensions: &extensions,
            workflow: Workflow::Wally,
        };
        assert!(
            crate::catalog::artifacts::entailed(&selections).is_empty(),
            "nothing may be forced when nothing is pinned"
        );
        let written: Vec<&str> = crate::catalog::artifacts::resolve(&selections, &[])
            .iter()
            .map(|a| a.key)
            .collect();
        assert_eq!(written, ["src", "default.project.json"]);
    }

    /// The other direction: pinning the tools *does* settle their configs, so
    /// pressing enter through both prompts gives a project whose linter can
    /// actually run.
    #[test]
    fn pinning_selene_settles_its_config() {
        let (packages, apps, extensions) = (Vec::new(), Vec::new(), Vec::new());
        let pinned = vec!["selene".to_string()];
        let selections = Selections {
            packages: &packages,
            tools: &pinned,
            apps: &apps,
            extensions: &extensions,
            workflow: Workflow::Wally,
        };
        let written: Vec<&str> = crate::catalog::artifacts::resolve(&selections, &[])
            .iter()
            .map(|a| a.key)
            .collect();
        assert!(written.contains(&"rokit.toml"), "{written:?}");
        assert!(written.contains(&"selene.toml"), "{written:?}");
    }

    /// The exact block a real selection prints. Reviewed as text, because
    /// this is the whole user-visible half of the fix: the user is told what
    /// was settled, *why*, and how to change it.
    #[test]
    fn the_settled_block_names_each_file_its_reason_and_the_way_out() {
        let packages = vec!["testez".to_string()];
        let tools = vec!["rojo".to_string(), "selene".to_string()];
        let (apps, extensions) = (Vec::new(), Vec::new());
        let selections = Selections {
            packages: &packages,
            tools: &tools,
            apps: &apps,
            extensions: &extensions,
            workflow: Workflow::Wally,
        };

        let lines = settled_lines(&crate::catalog::artifacts::entailed(&selections));
        assert_eq!(
            lines,
            vec![
                "Already settled by your answers so far:",
                "  rokit.toml   it is where this project's tool versions are pinned",
                "  wally.toml   the packages you picked are installed from it",
                "  selene.toml  selene defaults to the Lua 5.1 std, so every Roblox global lints as undefined",
                "  testez.yml   selene.toml sets std = roblox+testez, and selene will not run without it",
                "To drop one of these, change the answer it follows from.",
            ]
        );
    }

    /// Nothing settled means nothing printed - not a heading with an empty
    /// list under it.
    #[test]
    fn the_settled_block_is_silent_when_everything_is_still_a_choice() {
        assert!(settled_lines(&[]).is_empty());
    }

    #[test]
    fn wally_projects_keep_every_selected_tool() {
        let config = config_with(&["rojo", "wally", "wally-package-types", "selene"]);
        let tools = tools_for_workflow(&config, PackageWorkflow::Wally);
        assert_eq!(tools, vec!["rojo", "wally", "wally-package-types", "selene"]);
    }
}


