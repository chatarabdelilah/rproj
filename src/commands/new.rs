use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{MultiSelect, Select};

use crate::catalog::wally_packages::{self, companions_for, Category, PackageSpec};
use crate::commands::provision;
use crate::config::{GlobalConfig, PackageWorkflow, ProjectConfig, SavedSetup};
use crate::steps::{
    blender, git, gitattributes, gitignore, modules, quality, rojo, testez, toolchain, vscode,
    wally,
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

    scaffold(&project_dir, name, &config, &packages, package_workflow)?;

    ProjectConfig {
        mode,
        package_workflow,
        packages: packages.iter().cloned().collect(),
        tools_at_creation: config.selected_rokit_tools.clone(),
    }
    .save_to(&project_dir)?;

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
            .map(|p| format!("{} - {} ({})", p.key, p.description, p.maintenance.short_badge()))
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
    config: &GlobalConfig,
    packages: &BTreeSet<String>,
    package_workflow: PackageWorkflow,
) -> Result<()> {
    git::ensure_repo_init(project_dir)?;

    toolchain::ensure_rokit_init(project_dir)?;
    // Pin only the tools this project will actually use. The machine-wide
    // selection is "what I want available"; a project's rokit.toml is "what
    // this project needs", and those aren't the same thing - pinning Wally
    // into a project that vendors its packages as git submodules pulls in a
    // tool it will never run and implies a workflow it isn't using.
    toolchain::add_selected_tools(project_dir, &tools_for_workflow(config, package_workflow))?;
    toolchain::ensure_selene_config(
        project_dir,
        packages.contains("testez"),
        package_workflow,
        wally_packages::allows_mixed_tables(packages),
    )?;
    toolchain::ensure_stylua_config(project_dir)?;

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
    if testez_selected {
        testez::ensure_test_folders(project_dir)?;
    }

    // default.project.json maps a $path (packages/ or modules/) that has to
    // exist before rojo will touch it at all - generating a sourcemap while
    // that folder is missing fails outright ("could not be turned into a
    // Roblox Instance"), not just incompletely. So the package install has
    // to happen, and the folder has to exist, before sourcemap generation -
    // which is why each workflow generates its own sourcemap at the end of
    // its own branch rather than sharing one call afterwards.
    match package_workflow {
        PackageWorkflow::Wally => {
            let package_name = format!("rproj/{}", slugify(name));
            let package_list: Vec<String> = packages.iter().cloned().collect();
            wally::ensure_wally_init(project_dir)?;
            wally::write_wally_toml(project_dir, &package_name, &package_list)?;
            // Installs, generates the sourcemap, and re-adds the exported
            // types a plain `wally install` leaves off. See `wally::sync`.
            wally::sync(project_dir)?;
        }
        PackageWorkflow::GitSubmodules => {
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
            rojo::generate_sourcemap(project_dir)?;
        }
    }

    // Quality gate. The check script is generated from the tools this
    // project actually selected, so it never invokes something that was
    // never installed; CI only lands if there's a script for it to run.
    if testez_selected {
        // All three: selene.toml already says roblox+testez, and without
        // testez.yml to resolve it selene refuses to run at all; luau-lsp
        // ignores both of those and needs tests/.luaurc of its own.
        testez::ensure_selene_std(project_dir)?;
        testez::ensure_tests_luaurc(project_dir)?;
        testez::ensure_companion_config(project_dir)?;
    }

    // Tells the editor which folders are vendored third-party code. Only
    // needed for the submodule workflow - luau-lsp already ignores Wally's
    // `_Index` by default, which is why that workflow never showed this.
    vscode::ensure_project_settings(project_dir, package_workflow)?;

    quality::ensure_luaurc(project_dir)?;
    // The same filtered list the project's rokit.toml got: a check script
    // must only invoke tools this project actually pins, or CI fails on a
    // command that isn't installed.
    if quality::ensure_check_script(
        project_dir,
        &tools_for_workflow(config, package_workflow),
        testez_selected,
    )? {
        quality::ensure_ci_workflow(project_dir, package_workflow, has_server_packages)?;
        quality::lute_setup(project_dir)?;
    }

    gitignore::ensure_entries(project_dir)?;
    gitattributes::ensure_gitattributes(project_dir)?;

    if config.blender_enabled() {
        blender::scaffold_starter_scene(project_dir)?;
    }

    Ok(())
}

fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

/// The machine-wide tool selection filtered to what this project's chosen
/// package workflow actually uses.
///
/// Wally and wally-package-types are Wally-workflow-only: under git
/// submodules there is no wally.toml to install from and no package thunks
/// to retype, so pinning them would just be noise in rokit.toml.
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

    #[test]
    fn wally_projects_keep_every_selected_tool() {
        let config = config_with(&["rojo", "wally", "wally-package-types", "selene"]);
        let tools = tools_for_workflow(&config, PackageWorkflow::Wally);
        assert_eq!(tools, vec!["rojo", "wally", "wally-package-types", "selene"]);
    }
}


