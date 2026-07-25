use anyhow::Result;
use inquire::MultiSelect;

use crate::catalog::tool_catalog::{ROKIT_TOOLS, STUDIO_PLUGINS, SYSTEM_APPS, VSCODE_EXTENSIONS};
use crate::catalog::Maintenance;
use crate::config::GlobalConfig;
use crate::steps::{blender, bootstrap, notify, rojo, studio_plugin, vscode};

struct Picked {
    system_apps: Vec<String>,
    rokit_tools: Vec<String>,
    studio_plugins: Vec<String>,
    vscode_extensions: Vec<String>,
}

pub fn run() -> Result<()> {
    println!(
        "rproj setup\n\
         This installs and configures everything rproj knows about: system apps,\n\
         the Rojo/Wally toolchain, Studio plugins, and editor extensions. Every\n\
         choice below shows what it does and whether it's actively maintained.\n\
         Nothing gets reinstalled if it's already present, and you can re-run\n\
         this any time to add or remove tools.\n"
    );

    let mut config = GlobalConfig::load()?;
    let picked = pick_everything(&config)?;

    bootstrap::ensure_rokit()?;

    for key in &picked.system_apps {
        let Some(entry) = SYSTEM_APPS.iter().find(|e| e.key == *key) else { continue };
        if bootstrap::is_installed(entry) {
            println!("check: {} already installed", entry.key);
        } else {
            bootstrap::install(entry)?;
        }
    }

    let projects_root = config
        .roblox_projects_root
        .clone()
        .unwrap_or(config.projects_root()?);
    bootstrap::ensure_projects_folder(&projects_root)?;

    if picked.studio_plugins.iter().any(|k| k == "rojo-plugin") {
        rojo::install_studio_plugin()?;
    }
    if picked.studio_plugins.iter().any(|k| k == "hoarcekat") {
        studio_plugin::install_from_latest_release("Kampfkarren/hoarcekat", ".rbxmx")?;
    }

    if picked.system_apps.iter().any(|k| k == "vscode") {
        vscode::ensure_extensions(
            &picked
                .vscode_extensions
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )?;
    }

    if picked.system_apps.iter().any(|k| k == "blender") {
        let zip_path = blender::download_latest_plugin_zip()?;
        blender::install_addon(&zip_path)?;
        blender::print_account_link_instructions();
    }

    config.roblox_projects_root = Some(projects_root);
    config.selected_system_apps = picked.system_apps;
    config.selected_rokit_tools = picked.rokit_tools;
    config.selected_studio_plugins = picked.studio_plugins;
    config.selected_vscode_extensions = picked.vscode_extensions;
    config.last_checked = Some(chrono_now());
    config.save()?;

    notify::summary("rproj setup complete", "Your machine is ready for Roblox development.");
    println!("\nDone. Run `rproj new <name>` to scaffold your first project.");
    Ok(())
}

fn pick_everything(existing: &GlobalConfig) -> Result<Picked> {
    let system_apps = pick_from_catalog(
        "System apps",
        SYSTEM_APPS,
        &existing.selected_system_apps,
    )?;
    let rokit_tools = pick_from_catalog(
        "Rokit-managed CLI tools (rojo/wally/selene/stylua/...)",
        ROKIT_TOOLS,
        &existing.selected_rokit_tools,
    )?;
    let studio_plugins = pick_from_catalog(
        "Roblox Studio plugins",
        STUDIO_PLUGINS,
        &existing.selected_studio_plugins,
    )?;

    let vscode_extensions = if system_apps.iter().any(|k| k == "vscode") {
        pick_from_catalog(
            "VS Code extensions",
            VSCODE_EXTENSIONS,
            &existing.selected_vscode_extensions,
        )?
    } else {
        Vec::new()
    };

    Ok(Picked { system_apps, rokit_tools, studio_plugins, vscode_extensions })
}

fn pick_from_catalog(
    prompt: &str,
    entries: &'static [crate::catalog::tool_catalog::ToolEntry],
    previously_selected: &[String],
) -> Result<Vec<String>> {
    let has_prior = !previously_selected.is_empty();
    let options: Vec<String> = entries
        .iter()
        .map(|e| format!("{} - {} ({})", e.key, e.description, badge(e.maintenance)))
        .collect();

    let default_indices: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            if has_prior {
                previously_selected.iter().any(|k| k == e.key)
            } else {
                e.default_selected
            }
        })
        .map(|(i, _)| i)
        .collect();

    let selected = MultiSelect::new(prompt, options)
        .with_default(&default_indices)
        .prompt()?;

    Ok(entries
        .iter()
        .filter(|e| selected.iter().any(|s| s.starts_with(&format!("{} - ", e.key))))
        .map(|e| e.key.to_string())
        .collect())
}

fn badge(m: Maintenance) -> &'static str {
    m.badge()
}

fn chrono_now() -> String {
    // Avoids pulling in a datetime crate just for a cache-staleness marker.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
