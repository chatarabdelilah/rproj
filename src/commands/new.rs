use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{MultiSelect, Select};

use crate::catalog::presets::PRESETS;
use crate::catalog::wally_packages::{self, companions_for, Category, PackageSpec};
use crate::config::{GlobalConfig, ProjectConfig};
use crate::steps::{blender, gitignore, rojo, toolchain, wally};

pub fn run(name: &str) -> Result<()> {
    let config = GlobalConfig::load()?;
    if config.selected_rokit_tools.is_empty() {
        bail!("no tools configured yet - run `rproj setup` first");
    }

    let project_dir = config.projects_root()?.join(name);
    if project_dir.exists() {
        bail!("{} already exists", project_dir.display());
    }
    std::fs::create_dir_all(&project_dir)
        .with_context(|| format!("failed to create {}", project_dir.display()))?;

    println!("Scaffolding `{name}` in {}\n", project_dir.display());

    let (mode, preset_key, packages) = pick_composition()?;

    scaffold(&project_dir, name, &config, &packages)?;

    ProjectConfig {
        mode: mode.to_string(),
        preset_key,
        packages: packages.into_iter().collect(),
        tools_at_creation: config.selected_rokit_tools.clone(),
    }
    .save_to(&project_dir)?;

    println!("\n`{name}` is ready. Run `rproj watch` from inside it to start developing.");
    Ok(())
}

fn pick_composition() -> Result<(&'static str, Option<String>, BTreeSet<String>)> {
    let mode = Select::new(
        "How do you want to set up this project's packages?",
        vec![
            "Preset workflow - pick a named bundle, no further questions",
            "Guided walkthrough - answer one question per category, with explanations (recommended for beginners)",
            "Expert checklist - pick anything, no hand-holding",
        ],
    )
    .prompt()?;

    if mode.starts_with("Preset") {
        let (key, packages) = pick_preset()?;
        Ok(("preset", Some(key), packages))
    } else if mode.starts_with("Guided") {
        Ok(("guided", None, pick_guided()?))
    } else {
        Ok(("expert", None, pick_expert()?))
    }
}

fn pick_preset() -> Result<(String, BTreeSet<String>)> {
    let options: Vec<String> = PRESETS
        .iter()
        .map(|p| format!("{} - {}", p.label, p.description))
        .collect();
    let chosen = Select::new("Pick a preset", options).prompt()?;
    let preset = PRESETS
        .iter()
        .find(|p| chosen.starts_with(p.label))
        .context("selected preset not found")?;

    let mut packages: BTreeSet<String> = preset.packages.iter().map(|s| s.to_string()).collect();
    add_companions(&mut packages);
    Ok((preset.key.to_string(), packages))
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

        let mut options: Vec<String> = choices
            .iter()
            .map(|p| format!("{} - {} ({})", p.key, p.description, p.maintenance.badge()))
            .collect();
        options.push("none".to_string());

        let picked = Select::new(&format!("{}: which do you want?", category.label()), options)
            .prompt()?;

        if picked == "none" {
            continue;
        }
        if let Some(spec) = choices.iter().find(|p| picked.starts_with(p.key)) {
            packages.insert(spec.key.to_string());
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
                p.maintenance.badge()
            )
        })
        .collect();

    let selected = MultiSelect::new("Pick every package this project needs", options).prompt()?;

    Ok(wally_packages::PACKAGES
        .iter()
        .filter(|p| selected.iter().any(|s| s.starts_with(&format!("{} - ", p.key))))
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
) -> Result<()> {
    toolchain::ensure_rokit_init(project_dir)?;
    toolchain::add_selected_tools(project_dir, &config.selected_rokit_tools)?;
    toolchain::ensure_selene_config(project_dir)?;
    toolchain::ensure_stylua_config(project_dir)?;

    rojo::ensure_rojo_init(project_dir)?;
    rojo::ensure_packages_in_project_json(project_dir)?;

    let package_name = format!("rproj/{}", slugify(name));
    let package_list: Vec<String> = packages.iter().cloned().collect();
    wally::ensure_wally_init(project_dir)?;
    wally::write_wally_toml(project_dir, &package_name, &package_list)?;
    wally::wally_install(project_dir)?;

    // Only needed once here to produce an initial sourcemap.json for
    // wally-package-types; `rproj watch` owns the actual long-running watcher.
    let mut watcher = rojo::start_sourcemap_watcher(project_dir)?;
    let _ = watcher.kill();
    let _ = watcher.wait();
    wally::wally_package_types(project_dir)?;

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
