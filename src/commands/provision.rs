use anyhow::Result;
use inquire::MultiSelect;

use crate::catalog::tool_catalog::{
    self, ToolEntry, PLUGINS, ROKIT_TOOLS, SYSTEM_APPS, VSCODE_EXTENSIONS,
};
use crate::config::GlobalConfig;
use crate::steps::{blender, bootstrap, rojo, studio_plugin, vscode};

/// Everything `rproj setup` used to do standalone, now shared so `rproj new`
/// can run the same tool/plugin/extension selection inline instead of
/// requiring `setup` as a prerequisite - the vision is `rproj new` being
/// self-sufficient, with things like the Blender add-on question appearing
/// contextually in the plugins step right after Blender itself is picked.
///
/// Mutates `config` in place with the resulting selections and installs
/// everything picked (idempotently - nothing already present gets
/// reinstalled). Does not save the config or print a completion message;
/// callers decide that.
pub fn run(config: &mut GlobalConfig) -> Result<()> {
    let system_apps = pick_from_catalog(
        "System apps",
        &SYSTEM_APPS.iter().collect::<Vec<_>>(),
        &config.selected_system_apps,
    )?;

    let rokit_tools = pick_from_catalog(
        "Rokit-managed CLI tools (rojo/wally/selene/stylua/...)",
        &ROKIT_TOOLS.iter().collect::<Vec<_>>(),
        &config.selected_rokit_tools,
    )?;

    let plugins = pick_plugins(&system_apps, &config.selected_studio_plugins)?;

    let vscode_extensions = if system_apps.iter().any(|k| k == "vscode") {
        pick_from_catalog(
            "VS Code extensions & themes",
            &VSCODE_EXTENSIONS.iter().collect::<Vec<_>>(),
            &config.selected_vscode_extensions,
        )?
    } else {
        Vec::new()
    };

    bootstrap::ensure_rokit()?;

    for key in &system_apps {
        let Some(entry) = SYSTEM_APPS.iter().find(|e| e.key == *key) else { continue };
        if bootstrap::is_installed(entry) {
            println!("check: {} already installed", entry.key);
        } else {
            bootstrap::install(entry)?;
        }
    }

    let projects_root = config.roblox_projects_root.clone().unwrap_or(config.projects_root()?);
    bootstrap::ensure_projects_folder(&projects_root)?;

    if plugins.iter().any(|k| k == "rojo-plugin") {
        rojo::install_studio_plugin()?;
    }
    if plugins.iter().any(|k| k == "hoarcekat") {
        studio_plugin::install_from_latest_release("Kampfkarren/hoarcekat", ".rbxmx")?;
    }
    if plugins.iter().any(|k| k == "luau-lsp-plugin") {
        studio_plugin::install_from_latest_release("JohnnyMorganz/luau-lsp", ".rbxm")?;
    }
    if plugins.iter().any(|k| k == "blender-plugin") {
        let zip_path = blender::download_latest_plugin_zip()?;
        blender::install_addon(&zip_path)?;
        blender::print_account_link_instructions();
    }

    if system_apps.iter().any(|k| k == "vscode") {
        vscode::ensure_extensions(&vscode_extensions.iter().map(String::as_str).collect::<Vec<_>>())?;
    }

    config.roblox_projects_root = Some(projects_root);
    config.selected_system_apps = system_apps;
    config.selected_rokit_tools = rokit_tools;
    config.selected_studio_plugins = plugins;
    config.selected_vscode_extensions = vscode_extensions;
    config.last_checked = Some(chrono_now());
    Ok(())
}

/// The plugins step, contextually filtered: the Blender add-on entry only
/// appears if Blender was picked as a system app in this same run - picking
/// Blender earlier surfaces "Blender plugin: Roblox Upload" here.
fn pick_plugins(system_apps: &[String], previously_selected: &[String]) -> Result<Vec<String>> {
    let relevant: Vec<&ToolEntry> = PLUGINS
        .iter()
        .filter(|p| {
            !matches!(p.kind, tool_catalog::ToolKind::BlenderAddon { .. })
                || system_apps.iter().any(|k| k == "blender")
        })
        .collect();
    pick_from_catalog("Plugins", &relevant, previously_selected)
}

fn pick_from_catalog(
    prompt: &str,
    entries: &[&ToolEntry],
    previously_selected: &[String],
) -> Result<Vec<String>> {
    let has_prior = !previously_selected.is_empty();
    let options: Vec<String> = entries
        .iter()
        .map(|e| format!("{} - {} ({})", e.key, e.description, e.maintenance.badge()))
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
        .with_help_message("↑↓ to move, space to select one, → to all, ← to none, enter to confirm, type to filter")
        .prompt()?;

    Ok(entries
        .iter()
        .filter(|e| selected.iter().any(|s| s.starts_with(&format!("{} - ", e.key))))
        .map(|e| e.key.to_string())
        .collect())
}

fn chrono_now() -> String {
    // Avoids pulling in a datetime crate just for a cache-staleness marker.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
