use anyhow::Result;
use inquire::list_option::ListOption;
use inquire::MultiSelect;

use crate::catalog::tool_catalog::{
    self, ToolEntry, PLUGINS, ROKIT_TOOLS, SYSTEM_APPS, VSCODE_EXTENSIONS,
};
use crate::config::GlobalConfig;
use crate::steps::{blender, bootstrap, rojo, studio_plugin, toolchain, vscode};

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

    for key in &system_apps {
        let Some(entry) = SYSTEM_APPS.iter().find(|e| e.key == *key) else { continue };
        if bootstrap::is_installed(entry) {
            println!("check: {} already installed", entry.key);
        } else if let Err(err) = bootstrap::install(entry) {
            warn_and_continue(entry.key, &err);
        }
    }

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

    if plugins.iter().any(|k| k == "rojo-plugin")
        && let Err(err) = rojo::install_studio_plugin()
    {
        warn_and_continue("rojo-plugin", &err);
    }
    if plugins.iter().any(|k| k == "hoarcekat")
        && let Err(err) = studio_plugin::install_from_latest_release("Kampfkarren/hoarcekat", ".rbxmx")
    {
        warn_and_continue("hoarcekat", &err);
    }
    if plugins.iter().any(|k| k == "luau-lsp-plugin")
        && let Err(err) = studio_plugin::install_from_latest_release("JohnnyMorganz/luau-lsp", ".rbxm")
    {
        warn_and_continue("luau-lsp-plugin", &err);
    }
    if plugins.iter().any(|k| k == "blender-plugin") {
        match blender::download_latest_plugin_zip().and_then(|zip| blender::install_addon(&zip)) {
            Ok(()) => blender::print_account_link_instructions(),
            Err(err) => warn_and_continue("blender-plugin", &err),
        }
    }

    if system_apps.iter().any(|k| k == "vscode")
        && let Err(err) =
            vscode::ensure_extensions(&vscode_extensions.iter().map(String::as_str).collect::<Vec<_>>())
    {
        warn_and_continue("vscode extensions", &err);
    }

    config.roblox_projects_root = Some(projects_root);
    config.selected_system_apps = system_apps;
    config.selected_rokit_tools = rokit_tools;
    config.selected_studio_plugins = plugins;
    config.selected_vscode_extensions = vscode_extensions;
    config.last_checked = Some(chrono_now());
    Ok(())
}

fn warn_and_continue(what: &str, err: &anyhow::Error) {
    eprintln!("warning: {what} failed, continuing without it - {err:#}\n");
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
        .with_help_message("↑↓ to move, space to select one, → to all, ← to none, enter to confirm, type to filter")
        .with_formatter(&compact_answer)
        .prompt()?;

    Ok(entries
        .iter()
        .filter(|e| selected.iter().any(|s| s.starts_with(&format!("{} - ", e.key))))
        .map(|e| e.key.to_string())
        .collect())
}

/// Post-answer summary shown after submitting a MultiSelect - inquire's
/// default joins every option's full "key - description (badge)" text,
/// which turns into an unreadable wall of text once more than a couple of
/// entries are selected. This prints just the keys instead.
fn compact_answer(opts: &[ListOption<&String>]) -> String {
    if opts.is_empty() {
        return "none".to_string();
    }
    let keys: Vec<&str> = opts.iter().map(|o| o.value.split(" - ").next().unwrap_or(o.value)).collect();
    format!("{} selected: {}", keys.len(), keys.join(", "))
}

fn chrono_now() -> String {
    // Avoids pulling in a datetime crate just for a cache-staleness marker.
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}
