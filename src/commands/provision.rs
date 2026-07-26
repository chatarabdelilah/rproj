use anyhow::Result;
use inquire::MultiSelect;

use crate::catalog::tool_catalog::{
    self, ToolEntry, PLUGINS, ROKIT_TOOLS, SYSTEM_APPS, VSCODE_EXTENSIONS,
};
use crate::config::GlobalConfig;
use crate::steps::{blender, bootstrap, rojo, studio_plugin, toolchain, vscode};
use crate::ui::{self, Tally};

/// Machine-wide provisioning: system apps, global CLI tools, Studio
/// plugins and editor extensions. This is `rproj setup`'s whole job, and
/// `rproj new` runs it inline *only* on a machine that has never been
/// provisioned (or with `--reconfigure`) - it's a once-per-PC concern, not
/// a per-project one. Some questions are contextual: picking Blender here
/// is what surfaces its add-on in the plugins step.
///
/// Mutates `config` in place with the resulting selections and installs
/// everything picked (idempotently - nothing already present gets
/// reinstalled). Does not save the config or print a completion message;
/// callers decide that.
///
/// Individual install failures (a flaky winget hash, a network hiccup) are
/// printed as warnings and skipped rather than aborting the whole run -
/// one broken installer shouldn't block everything else that already
/// succeeded, or the project scaffolding that follows.
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

    // Rokit itself is foundational - if this fails, nothing rokit-managed
    // can work anyway, so this one stays a hard failure.
    bootstrap::ensure_rokit()?;

    ui::section("Machine setup");
    let mut apps = Tally::new();
    for key in &system_apps {
        let Some(entry) = SYSTEM_APPS.iter().find(|e| e.key == *key) else { continue };
        if bootstrap::is_installed(entry) {
            apps.already(entry.key);
        } else {
            match bootstrap::install(entry) {
                Ok(()) => apps.did(entry.key),
                Err(err) => {
                    ui::warn(&format!("{}: {err}", entry.key));
                    if format!("{err}").contains("installer hash") {
                        ui::detail(bootstrap::WINGET_HASH_HELP);
                    }
                }
            }
        }
    }
    apps.finish("system apps");

    let projects_root = config.roblox_projects_root.clone().unwrap_or(config.projects_root()?);
    bootstrap::ensure_projects_folder(&projects_root)?;

    // Installs each selected rokit tool into rokit's global manifest so
    // it's resolvable from any directory - required before the plugins
    // step below, since e.g. `rojo plugin install` needs `rojo` itself to
    // already resolve globally (there's no project directory yet at this
    // point in `rproj new`, so a project-local rokit.toml can't help here).
    if let Err(err) = toolchain::add_global_tools(&rokit_tools) {
        warn_and_continue("rokit tools", &err);
    }

    // Rojo's plugin installer locates Studio via the Windows registry and
    // fails with a confusing "couldn't find registry keys" error if Studio
    // isn't actually there - check first and skip with a clear reason
    // instead (this comes up in practice: Roblox Studio's winget package
    // has its own known hash-mismatch issue, see bootstrap::install).
    let studio_installed = SYSTEM_APPS.iter().find(|e| e.key == "studio").is_some_and(bootstrap::is_installed);
    if plugins.iter().any(|k| k == "rojo-plugin") {
        if studio_installed {
            if let Err(err) = rojo::install_studio_plugin() {
                warn_and_continue("rojo-plugin", &err);
            }
        } else {
            ui::skip("rojo-plugin needs Roblox Studio, which isn't installed yet");
        }
    }
    // Sourced from the catalog entry (github_repo/asset_suffix) rather than
    // duplicated as literals here - a hardcoded ".rbxmx" here for hoarcekat
    // is exactly how that repo/suffix pair drifted from its actual release
    // asset (a plain ".rbxm") without anyone noticing.
    for key in ["hoarcekat", "luau-lsp-plugin"] {
        if plugins.iter().any(|k| k == key)
            && let Some((repo, suffix)) = studio_plugin_repo(key)
            && let Err(err) = studio_plugin::install_from_latest_release(repo, suffix)
        {
            warn_and_continue(key, &err);
        }
    }
    if plugins.iter().any(|k| k == "blender-plugin")
        && let Some(repo) = blender_addon_repo()
    {
        match blender::download_latest_plugin_zip(repo).and_then(|zip| blender::install_addon(&zip)) {
            Ok(()) => blender::print_account_link_instructions(),
            Err(err) => warn_and_continue("blender-plugin", &err),
        }
    }

    if system_apps.iter().any(|k| k == "vscode") {
        // vscode_extensions holds catalog keys (e.g. "luau-lsp"), not the
        // actual marketplace extension ids (e.g. "JohnnyMorganz.luau-lsp") -
        // look those up rather than passing the key straight to `code
        // --install-extension`, which just fails with "not found" silently
        // for every single entry.
        let extension_ids: Vec<&str> = vscode_extensions
            .iter()
            .filter_map(|key| match tool_catalog::find(key)?.kind {
                tool_catalog::ToolKind::VsCodeExtension { extension_id } => Some(extension_id),
                _ => None,
            })
            .collect();
        if let Err(err) = vscode::ensure_extensions(&extension_ids) {
            warn_and_continue("vscode extensions", &err);
        }
    }

    config.roblox_projects_root = Some(projects_root);
    config.selected_system_apps = system_apps;
    config.selected_rokit_tools = rokit_tools;
    config.selected_studio_plugins = plugins;
    config.selected_vscode_extensions = vscode_extensions;
    config.last_checked = Some(now_unix_seconds());
    Ok(())
}

fn warn_and_continue(what: &str, err: &anyhow::Error) {
    ui::warn(&format!("{what} skipped - {err}"));
}

fn studio_plugin_repo(key: &str) -> Option<(&'static str, &'static str)> {
    match tool_catalog::find(key)?.kind {
        tool_catalog::ToolKind::StudioPlugin { github_repo, asset_suffix } => Some((github_repo, asset_suffix)),
        _ => None,
    }
}

fn blender_addon_repo() -> Option<&'static str> {
    match tool_catalog::find("blender-plugin")?.kind {
        tool_catalog::ToolKind::BlenderAddon { github_repo } => Some(github_repo),
        _ => None,
    }
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
        .map(|e| format!("{} - {} ({})", e.key, e.description, e.maintenance.short_badge()))
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
        .with_help_message(ui::MULTISELECT_HELP)
        .with_formatter(&ui::compact_multi_answer)
        .prompt()?;

    Ok(entries
        .iter()
        .filter(|e| selected.iter().any(|s| ui::option_is(s, e.key)))
        .map(|e| e.key.to_string())
        .collect())
}


/// Unix seconds, as a string. Doubles as the "has this machine been
/// provisioned" marker (`GlobalConfig::machine_configured`).
fn now_unix_seconds() -> String {
    // Avoids pulling in a datetime crate just for a marker.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
