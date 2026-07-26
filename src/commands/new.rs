use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{MultiSelect, Select};

use crate::catalog::wally_packages::{self, companions_for, Category, PackageSpec};
use crate::commands::provision;
use crate::config::{GlobalConfig, PackageWorkflow, ProjectConfig};
use crate::steps::{blender, git, gitignore, rojo, toolchain, wally};

pub fn run(name: &str) -> Result<()> {
    let mut config = GlobalConfig::load()?;

    let project_dir = config.projects_root()?.join(name);
    if project_dir.exists() {
        bail!("{} already exists", project_dir.display());
    }

    // Self-sufficient: no `rproj setup` prerequisite. Runs the same
    // tool/plugin/extension selection inline (defaulting to whatever was
    // picked before, so repeat runs are a quick confirm-through) - picking
    // Blender here is what surfaces its plugin question in the next step.
    println!(
        "rproj new {name}\n\
         First, let's make sure your machine has what it needs. Every choice\n\
         below shows what it does and whether it's actively maintained - nothing\n\
         gets reinstalled if it's already present.\n"
    );
    provision::run(&mut config)?;
    config.save()?;

    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("failed to create {}", project_dir.display()))?;

    println!("\nScaffolding `{name}` in {}\n", project_dir.display());

    let (mode, packages) = pick_composition()?;
    let package_workflow = pick_package_workflow(&packages)?;

    scaffold(&project_dir, name, &config, &packages, package_workflow)?;

    ProjectConfig {
        mode: mode.to_string(),
        package_workflow,
        packages: packages.into_iter().collect(),
        tools_at_creation: config.selected_rokit_tools.clone(),
    }
    .save_to(&project_dir)?;

    println!("\n`{name}` is ready. Run `rproj watch` from inside it to start developing.");
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
        let prompt = format!("{}: which do you want?", category.label());

        if category.allows_multiple() {
            let selected = MultiSelect::new(&prompt, options)
                .with_help_message(
                    "↑↓ to move, space to select one, → to all, ← to none, enter to confirm, type to filter",
                )
                .with_formatter(&compact_multi_answer)
                .prompt()?;
            for spec in &choices {
                if selected.iter().any(|s| s.starts_with(&format!("{} - ", spec.key))) {
                    packages.insert(spec.key.to_string());
                }
            }
        } else {
            let mut options = options;
            options.push("none".to_string());
            let picked = Select::new(&prompt, options).with_formatter(&compact_select_answer).prompt()?;
            if picked != "none"
                && let Some(spec) = choices.iter().find(|p| picked.starts_with(p.key))
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
        .with_help_message("↑↓ to move, space to select one, → to all, ← to none, enter to confirm, type to filter")
        .with_formatter(&compact_multi_answer)
        .prompt()?;

    Ok(wally_packages::PACKAGES
        .iter()
        .filter(|p| selected.iter().any(|s| s.starts_with(&format!("{} - ", p.key))))
        .map(|p| p.key.to_string())
        .collect())
}

/// Post-answer summaries for the package pickers above - same reasoning as
/// `commands::provision`'s formatters: inquire's defaults echo back the
/// full "key - description (badge)" text, which is fine while choosing but
/// unreadable once printed as a single confirmed-answer line.
fn compact_multi_answer(opts: &[inquire::list_option::ListOption<&String>]) -> String {
    if opts.is_empty() {
        return "none".to_string();
    }
    let keys: Vec<&str> = opts.iter().map(|o| o.value.split(" - ").next().unwrap_or(o.value)).collect();
    format!("{} selected: {}", keys.len(), keys.join(", "))
}

fn compact_select_answer(opt: inquire::list_option::ListOption<&String>) -> String {
    opt.value.split(" - ").next().unwrap_or(opt.value).to_string()
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
    toolchain::add_selected_tools(project_dir, &config.selected_rokit_tools)?;
    toolchain::ensure_selene_config(project_dir, packages.contains("testez"))?;
    toolchain::ensure_stylua_config(project_dir)?;

    rojo::scaffold_project_json(project_dir, name, package_workflow)?;

    // default.project.json maps a $path (packages/ or Modules/) that has to
    // exist before rojo will touch it at all - generating a sourcemap while
    // that folder is missing fails outright ("could not be turned into a
    // Roblox Instance"), not just incompletely. So the package install has
    // to happen, and the folder has to exist, before sourcemap generation.
    match package_workflow {
        PackageWorkflow::Wally => {
            let package_name = format!("rproj/{}", slugify(name));
            let package_list: Vec<String> = packages.iter().cloned().collect();
            wally::ensure_wally_init(project_dir)?;
            wally::write_wally_toml(project_dir, &package_name, &package_list)?;
            wally::wally_install(project_dir)?;
            // wally install may not create the folder at all when there are
            // zero dependencies - ensure it exists regardless.
            std::fs::create_dir_all(project_dir.join("packages"))?;
        }
        PackageWorkflow::GitSubmodules => {
            let mut added_repos = BTreeSet::new();
            for key in packages {
                let Some(spec) = wally_packages::find(key) else { continue };
                if added_repos.insert(spec.git_repo) {
                    git::add_submodule(project_dir, spec.git_repo, spec.repo_folder_name())?;
                }
            }
            std::fs::create_dir_all(project_dir.join("Modules"))?;
        }
    }

    // Generate an initial sourcemap.json now that the packages/Modules
    // folder actually exists - useful on its own for luau-lsp, and
    // wally-package-types additionally needs it below when using Wally.
    let mut watcher = rojo::start_sourcemap_watcher(project_dir)?;
    let _ = watcher.kill();
    let _ = watcher.wait();

    if package_workflow == PackageWorkflow::Wally {
        wally::wally_package_types(project_dir)?;
    }

    gitignore::ensure_entries(project_dir)?;

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
