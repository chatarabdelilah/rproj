//! `rproj configure [key]` - configure a project tool or route to the
//! machine-wide Rojo project template editor.
//!
//! Entirely driven by `catalog::tool_settings`: this module knows how to
//! render a `SettingSpec` and how to write the two `ConfigTarget` kinds,
//! and nothing about any specific tool or setting.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use inquire::{Confirm, CustomType, Select};
use serde_json::{Value, json};

use crate::catalog::tool_settings::{
    self, CONFIGURABLE_TOOLS, ConfigTarget, ConfigurableTool, SettingKind, SettingSpec,
};
use crate::steps::vscode;
use crate::ui;

pub fn run(key: Option<&str>) -> Result<()> {
    if key == Some("project") {
        return super::project_template::run();
    }
    let project_dir = std::env::current_dir()?;

    let tool = match key {
        Some(key) => tool_settings::find(key).with_context(|| {
            format!(
                "no configurable tool called `{key}`. Available: {}",
                CONFIGURABLE_TOOLS
                    .iter()
                    .map(|t| t.key)
                    .chain(std::iter::once("project"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?,
        None => match pick_target()? {
            Some(tool) => tool,
            None => return super::project_template::run(),
        },
    };

    println!(
        "\n{} - {}\n{}\n",
        tool.display_name, tool.summary, tool.docs_url
    );
    println!(
        "Enter keeps what this project already uses. Settings are written to {}.\n",
        target_description(&tool.target)
    );

    let current = current_values(&project_dir, tool)?;

    let mut answers: Vec<(&SettingSpec, Value)> = Vec::new();
    for (setting, current) in tool.settings.iter().zip(current) {
        if let Some(value) = ask(setting, current.as_ref())?
            && current.as_ref() != Some(&value)
        {
            answers.push((setting, value));
        }
    }

    if answers.is_empty() {
        ui::ok("no settings changed");
        return Ok(());
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
                toml::from_str::<toml::Table>(&existing).with_context(|| {
                    format!(
                        "could not parse {} - fix or delete it, then re-run",
                        path.display()
                    )
                })?;
            }
            Ok(tool_settings::current_toml_values(tool, &existing))
        }
        ConfigTarget::VsCodeSettings => {
            let settings = vscode::read_settings(project_dir)?;
            Ok(tool
                .settings
                .iter()
                .map(|s| settings.get(s.key).cloned())
                .collect())
        }
    }
}

fn read_if_present(path: &Path) -> Result<String> {
    match path.exists() {
        true => {
            fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
        }
        false => Ok(String::new()),
    }
}

fn pick_target() -> Result<Option<&'static ConfigurableTool>> {
    let mut options: Vec<String> = CONFIGURABLE_TOOLS
        .iter()
        .map(|t| format!("{}{}{}", t.key, ui::OPTION_SEPARATOR, t.display_name))
        .collect();
    options.push(format!(
        "project{}Default project tree for future projects",
        ui::OPTION_SEPARATOR
    ));
    let picked = Select::new("What do you want to configure?", options)
        .with_formatter(&ui::compact_select_answer)
        .prompt()?;
    let key = ui::option_key(&picked);
    if key == "project" {
        Ok(None)
    } else {
        Ok(Some(
            tool_settings::find(key).context("internal: picked an unknown tool")?,
        ))
    }
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
fn ask(setting: &SettingSpec, current: Option<&Value>) -> Result<Option<Value>> {
    crate::diagnostics::event("prompt.setting", setting.display_key());
    println!("{}\n  {}", setting.display_key(), setting.description);

    let current = if current.is_some_and(|value| !can_prompt(&setting.kind, value)) {
        println!("  The current value is outside this guided editor's supported choices.");
        if !Confirm::new("  Replace the existing value?")
            .with_default(false)
            .prompt()?
        {
            crate::diagnostics::event("setting.kept", setting.display_key());
            println!();
            return Ok(None);
        }
        None
    } else {
        current
    };

    let value = match &setting.kind {
        SettingKind::Bool { default } => {
            let default = current.and_then(Value::as_bool).unwrap_or(*default);
            json!(Confirm::new("  Enable?").with_default(default).prompt()?)
        }
        SettingKind::Integer { default } => {
            let default = current.and_then(Value::as_i64).unwrap_or(*default);
            json!(
                CustomType::<i64>::new("  Value:")
                    .with_default(default)
                    .prompt()?
            )
        }
        SettingKind::Choice { default, options } => {
            let labels: Vec<String> = options
                .iter()
                .map(|o| format!("{}{}{}", o.value, ui::OPTION_SEPARATOR, o.explanation))
                .collect();
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

    crate::diagnostics::event(
        "choice.setting",
        format!("{}={value}", setting.display_key()),
    );
    println!();
    Ok(Some(value))
}

fn can_prompt(kind: &SettingKind, value: &Value) -> bool {
    match kind {
        SettingKind::Bool { .. } => value.is_boolean(),
        SettingKind::Integer { .. } => value.as_i64().is_some(),
        SettingKind::Choice { options, .. } => options
            .iter()
            .any(|option| value.as_str() == Some(option.value)),
    }
}

/// Applies the answers to the tool's TOML file, merging rather than
/// replacing (see `tool_settings::merge_toml`). Replacing deleted every key
/// the catalog doesn't describe - `selene.toml`'s scaffolded `exclude`
/// among them.
fn write_toml(project_dir: &Path, filename: &str, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let path = project_dir.join(filename);
    let existing = read_if_present(&path)?;
    let merged = checked_toml_merge(&existing, answers)
        .with_context(|| format!("{} was not changed", path.display()))?;
    fs::write(&path, merged).with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok(&format!("wrote {filename}"));
    Ok(())
}

fn checked_toml_merge(existing: &str, answers: &[(&SettingSpec, Value)]) -> Result<String> {
    let mut expected = toml::from_str::<toml::Table>(existing)
        .context("could not parse current TOML; fix it and re-run configure")?;
    for (setting, value) in answers {
        let table = match setting.section {
            Some(section) => expected
                .entry(section)
                .or_insert_with(|| toml::Value::Table(toml::Table::new()))
                .as_table_mut()
                .with_context(|| format!("{section} is not a TOML table"))?,
            None => &mut expected,
        };
        table.insert(setting.key.into(), toml::Value::try_from(value)?);
    }
    let merged = tool_settings::merge_toml(existing, answers);
    let actual = toml::from_str::<toml::Table>(&merged)
        .context("cannot safely edit this TOML layout; edit the file manually")?;
    // The legacy line writer must not change content inside multiline values or other tables.
    ensure!(
        actual == expected,
        "cannot safely edit this TOML layout without changing other values; edit the file manually"
    );
    Ok(merged)
}

/// Applies the answers to `.vscode/settings.json`, merging rather than
/// replacing (see `steps::vscode::merge_settings`).
fn write_vscode_settings(project_dir: &Path, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let entries: Vec<(&str, Value)> = answers.iter().map(|(s, v)| (s.key, v.clone())).collect();
    vscode::merge_settings(project_dir, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_support_is_typed_and_does_not_treat_unknown_values_as_missing() {
        let boolean = SettingKind::Bool { default: true };
        let integer = SettingKind::Integer { default: 4 };
        assert!(can_prompt(&boolean, &json!(false)));
        assert!(!can_prompt(&boolean, &json!("false")));
        assert!(can_prompt(&integer, &json!(i64::MAX)));
        for value in [json!(4.5), json!(u64::MAX), Value::Null, json!([4])] {
            assert!(!can_prompt(&integer, &value));
        }
        let selene = tool_settings::find("selene").unwrap();
        assert!(can_prompt(&selene.settings[0].kind, &json!("roblox")));
        assert!(!can_prompt(
            &selene.settings[0].kind,
            &json!("roblox+custom")
        ));
        for value in ["['custom']", "{ future = true }", "nan", "1979-05-27"] {
            let current = tool_settings::current_toml_values(selene, &format!("std = {value}"));
            assert!(current[0].is_some(), "{value} must not become absent");
        }
    }

    #[test]
    fn merge_validation_rejects_valid_toml_with_unintended_semantic_changes() {
        let selene = tool_settings::find("selene").unwrap();
        let setting = &selene.settings[0];
        let existing = "std = \"roblox\"\nnotes = '''\nstd = \"example\"\n'''\n";
        let answers = [(setting, json!("roblox+testez"))];
        let unchecked = tool_settings::merge_toml(existing, &answers);
        assert!(toml::from_str::<toml::Table>(&unchecked).is_ok());
        assert!(checked_toml_merge(existing, &answers).is_err());
        assert!(checked_toml_merge("std = \"roblox\"\n", &answers).is_ok());
    }
}
