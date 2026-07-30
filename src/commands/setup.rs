use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::catalog::{tool_catalog, tool_usage};
use crate::commands::provision;
use crate::config::GlobalConfig;
use crate::steps::{notify, tarmac, toolchain};
use crate::ui;

/// `rproj setup` with no tool: machine-wide provisioning.
/// `rproj setup <tool>`: that one tool, in the current project.
///
/// The two share a verb because they are the same request at two scopes -
/// "make this ready to use" - and which scope is meant is unambiguous from
/// whether a tool was named.
pub fn run(tool: Option<&str>) -> Result<()> {
    match tool {
        Some(key) => setup_tool(key),
        None => setup_machine(),
    }
}

fn setup_machine() -> Result<()> {
    // One line, not the seven-line explanation this used to print.
    //
    // Everything that paragraph said is either already known (the user
    // typed `setup`), visible in the pickers that follow (each option
    // carries its own description and badge), or belongs on the welcome
    // screen, which already describes what this command is for. Seven
    // lines of preamble ahead of the first question is a wall between the
    // user and the thing they asked for.
    ui::detail("Nothing already installed is reinstalled. Safe to re-run any time.");

    let mut config = GlobalConfig::load()?;
    provision::run(&mut config)?;
    config.save()?;

    notify::summary("rproj setup complete", "Your machine is ready for Roblox development.");
    println!("\nDone. Run `rproj new <name>` to scaffold your first project.");
    Ok(())
}

/// Sets one tool up in the project the user is standing in.
///
/// Replaces the previous answer to "how do I start using Tarmac", which was
/// `rproj info tarmac`, read four commands, and do it by hand.
fn setup_tool(key: &str) -> Result<()> {
    let entry = tool_catalog::find(key).with_context(|| {
        format!("no tool called `{key}`. `rproj info` lists everything rproj knows about")
    })?;

    let project_dir = find_project_root()?;
    ui::section(&format!("Setting up {} in {}", entry.key, project_dir.display()));

    // 1. Pin it, so the version is the project's rather than whatever the
    //    machine happens to have. Idempotent: rokit leaves an existing pin
    //    alone.
    match entry.kind {
        tool_catalog::ToolKind::RokitTool { .. } => {
            // A project scaffolded without rokit.toml - now a legitimate
            // choice - has nothing to pin into, and `rokit add` fails with
            // its own "no manifest found". Creating it first is idempotent
            // and is what the user meant by asking to set a tool up here.
            toolchain::ensure_rokit_init(&project_dir)?;
            toolchain::add_selected_tools(&project_dir, std::slice::from_ref(&entry.key.to_string()))?;
        }
        // Everything else is machine-wide, so there is nothing project-local
        // to pin. Saying so beats silently doing nothing.
        _ => ui::skip(&format!(
            "{} is a {}, so there is nothing to pin per project",
            entry.key,
            entry.kind.label()
        )),
    }

    // 2. Write its config, where rproj knows the format.
    match entry.key {
        "tarmac" => tarmac::ensure_config(&project_dir, &project_name(&project_dir))?,
        _ => {
            // `rproj configure` owns the tools with a settings walkthrough;
            // pointing at it beats reimplementing it here.
            if tool_usage::find(entry.key).is_some() {
                ui::skip(&format!(
                    "no config file for {} - `rproj configure {}` if it has settings",
                    entry.key, entry.key
                ));
            }
        }
    }

    // 3. The part that cannot be scripted, and the first command to run.
    if let Some(usage) = tool_usage::find(entry.key) {
        if !usage.notes.is_empty() {
            println!();
            ui::detail("Worth knowing:");
            for note in usage.notes {
                ui::detail(&format!("  - {note}"));
            }
        }
        if let Some((command, what)) = usage.commands.first() {
            println!();
            ui::detail("Try next:");
            ui::detail(&format!("  {command}"));
            ui::detail(&format!("  {what}"));
        }
    }
    Ok(())
}

/// The nearest ancestor holding a Rojo project file.
///
/// Walks upwards rather than requiring the exact project root, because
/// `rproj setup tarmac` from inside `src/` is a reasonable thing to type.
fn find_project_root() -> Result<PathBuf> {
    let start = env::current_dir().context("could not read the current directory")?;
    for dir in start.ancestors() {
        if dir.join("default.project.json").is_file() {
            return Ok(dir.to_path_buf());
        }
    }
    bail!(
        "not inside a Roblox project - no default.project.json here or above {}.\n\
         `rproj setup` with no tool name sets up the machine instead, and \
         `rproj new <name>` creates a project.",
        start.display()
    )
}

/// The project's name, for configs that want one.
fn project_name(project_dir: &Path) -> String {
    project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("roblox-project")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unknown tool must say so, and say where the list is - not fail
    /// with something about a missing directory.
    #[test]
    fn an_unknown_tool_is_named_in_the_error() {
        let err = setup_tool("definitely-not-a-tool").unwrap_err();
        let message = format!("{err}");
        assert!(message.contains("definitely-not-a-tool"), "{message}");
        assert!(message.contains("rproj info"), "{message}");
    }

    /// The name is the folder, which is what a user would expect it to be.
    #[test]
    fn the_project_name_comes_from_the_folder() {
        assert_eq!(project_name(Path::new("/games/my-game")), "my-game");
    }

    /// Every tool `rproj setup <tool>` accepts should have something useful
    /// to say - otherwise the command succeeds and prints nothing, which
    /// reads as a failure.
    #[test]
    fn the_documented_setup_candidates_all_have_usage_text() {
        // `luau-lsp-cli`, not `luau-lsp`: the latter is the VS Code
        // extension's key. Both are real catalog entries, so a name that
        // looks right resolves to the wrong one - `rproj setup luau-lsp`
        // finds the extension and correctly reports there is nothing
        // project-local to pin.
        for key in ["tarmac", "mantle", "lute", "luau-lsp-cli"] {
            assert!(
                tool_catalog::find(key).is_some(),
                "{key} is not in the tool catalog"
            );
            let usage = tool_usage::find(key)
                .unwrap_or_else(|| panic!("{key} has no usage entry, so setup would print nothing"));
            assert!(!usage.commands.is_empty(), "{key} lists no commands");
        }
    }
}
