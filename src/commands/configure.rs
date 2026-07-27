//! `rproj configure [tool]` - walk through a tool's settings one at a
//! time, explaining what each one does, and write the answers to that
//! tool's config file.
//!
//! Entirely driven by `catalog::tool_settings`: this module knows how to
//! render a `SettingSpec` and how to write the two `ConfigTarget` kinds,
//! and nothing about any specific tool or setting.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use inquire::{Confirm, CustomType, Select};
use serde_json::{json, Value};

use crate::catalog::tool_settings::{
    self, ConfigTarget, ConfigurableTool, SettingKind, SettingSpec, CONFIGURABLE_TOOLS,
};
use crate::steps::vscode;
use crate::ui;

pub fn run(key: Option<&str>) -> Result<()> {
    let project_dir = std::env::current_dir()?;

    let tool = match key {
        Some(key) => tool_settings::find(key).with_context(|| {
            format!(
                "no configurable tool called `{key}`. Available: {}",
                CONFIGURABLE_TOOLS.iter().map(|t| t.key).collect::<Vec<_>>().join(", ")
            )
        })?,
        None => pick_tool()?,
    };

    println!("\n{} - {}\n{}\n", tool.display_name, tool.summary, tool.docs_url);
    println!(
        "Enter keeps what this project already uses. Settings are written to {}.\n",
        target_description(&tool.target)
    );

    // Seeded from the file on disk, so pressing enter through the
    // walkthrough changes nothing. It used to seed from the catalog
    // instead, which quietly reverted every earlier choice - a TestEZ
    // project's `std = "roblox+testez"` went back to plain `roblox`, and any
    // editor setting turned off went back on.
    let current = current_values(&project_dir, tool)?;

    let mut answers: Vec<(&SettingSpec, Value)> = Vec::new();
    for (setting, current) in tool.settings.iter().zip(current) {
        answers.push((setting, ask(setting, current.as_ref())?));
    }

    match &tool.target {
        ConfigTarget::ProjectToml { filename } => write_toml(&project_dir, filename, &answers)?,
        ConfigTarget::VsCodeSettings => write_vscode_settings(&project_dir, &answers)?,
    }
    Ok(())
}

/// What each setting is set to right now, one entry per `tool.settings`.
fn current_values(project_dir: &Path, tool: &ConfigurableTool) -> Result<Vec<Option<Value>>> {
    match &tool.target {
        ConfigTarget::ProjectToml { filename } => {
            let path = project_dir.join(filename);
            let existing = read_if_present(&path)?;
            // A config the tool itself can't read is a problem worth
            // surfacing now: every prompt default would otherwise be a
            // catalog value silently disagreeing with the file.
            if !existing.trim().is_empty() {
                toml::from_str::<toml::Table>(&existing)
                    .with_context(|| format!("could not parse {} - fix or delete it, then re-run", path.display()))?;
            }
            Ok(tool_settings::current_toml_values(tool, &existing))
        }
        ConfigTarget::VsCodeSettings => {
            let settings = vscode::read_settings(project_dir)?;
            Ok(tool.settings.iter().map(|s| settings.get(s.key).cloned()).collect())
        }
    }
}

fn read_if_present(path: &Path) -> Result<String> {
    match path.exists() {
        true => fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display())),
        false => Ok(String::new()),
    }
}

fn pick_tool() -> Result<&'static ConfigurableTool> {
    let options: Vec<String> = CONFIGURABLE_TOOLS
        .iter()
        .map(|t| format!("{}{}{}", t.key, ui::OPTION_SEPARATOR, t.display_name))
        .collect();
    let picked = Select::new("Which tool do you want to configure?", options)
        .with_formatter(&ui::compact_select_answer)
        .prompt()?;
    let key = ui::option_key(&picked);
    tool_settings::find(key).context("internal: picked an unknown tool")
}

fn target_description(target: &ConfigTarget) -> String {
    match target {
        ConfigTarget::ProjectToml { filename } => (*filename).to_string(),
        ConfigTarget::VsCodeSettings => ".vscode/settings.json".to_string(),
    }
}

/// Prompts for one setting. The description is printed above the prompt
/// rather than crammed into it, so a long explanation stays readable and
/// the question line itself remains short.
///
/// `current` is what the project's config file already says, and takes
/// precedence over the catalog default - a walkthrough that proposes
/// reverting your own settings is worse than no walkthrough.
fn ask(setting: &SettingSpec, current: Option<&Value>) -> Result<Value> {
    println!("{}\n  {}", setting.display_key(), setting.description);

    let value = match &setting.kind {
        SettingKind::Bool { default } => {
            let default = current.and_then(Value::as_bool).unwrap_or(*default);
            json!(Confirm::new("  Enable?").with_default(default).prompt()?)
        }
        SettingKind::Integer { default } => {
            let default = current.and_then(Value::as_i64).unwrap_or(*default);
            json!(CustomType::<i64>::new("  Value:").with_default(default).prompt()?)
        }
        SettingKind::Choice { default, options } => {
            let labels: Vec<String> =
                options.iter().map(|o| format!("{}{}{}", o.value, ui::OPTION_SEPARATOR, o.explanation)).collect();
            // A current value the catalog doesn't list (a hand-written
            // config, or one written by an older rproj) can't be the
            // starting cursor, so fall back to the documented default.
            let selected = current.and_then(Value::as_str).unwrap_or(default);
            let start = options
                .iter()
                .position(|o| o.value == selected)
                .or_else(|| options.iter().position(|o| o.value == *default))
                .unwrap_or(0);
            let picked = Select::new("  Value:", labels)
                .with_starting_cursor(start)
                .with_formatter(&ui::compact_select_answer)
                .prompt()?;
            json!(ui::option_key(&picked))
        }
    };

    println!();
    Ok(value)
}

/// Applies the answers to the tool's TOML file, merging rather than
/// replacing (see `tool_settings::merge_toml`). Replacing deleted every key
/// the catalog doesn't describe - `selene.toml`'s scaffolded `exclude`
/// among them.
fn write_toml(project_dir: &Path, filename: &str, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let path = project_dir.join(filename);
    let existing = read_if_present(&path)?;
    fs::write(&path, tool_settings::merge_toml(&existing, answers))
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok(&format!("wrote {filename}"));
    Ok(())
}

/// Applies the answers to `.vscode/settings.json`, merging rather than
/// replacing (see `steps::vscode::merge_settings`).
fn write_vscode_settings(project_dir: &Path, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let entries: Vec<(&str, Value)> =
        answers.iter().map(|(s, v)| (s.key, v.clone())).collect();
    vscode::merge_settings(project_dir, &entries)
}
