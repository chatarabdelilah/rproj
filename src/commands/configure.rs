//! `rproj configure [tool]` - walk through a tool's settings one at a
//! time, explaining what each one does, and write the answers to that
//! tool's config file.
//!
//! Entirely driven by `catalog::tool_settings`: this module knows how to
//! render a `SettingSpec` and how to write the two `ConfigTarget` kinds,
//! and nothing about any specific tool or setting.

use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use inquire::{Confirm, CustomType, Select};
use serde_json::{json, Value};

use crate::catalog::tool_settings::{
    self, ConfigTarget, ConfigurableTool, SettingKind, SettingSpec, CONFIGURABLE_TOOLS,
};

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
        "Enter accepts the shown default. Settings are written to {}.\n",
        target_description(&tool.target)
    );

    let mut answers: Vec<(&SettingSpec, Value)> = Vec::new();
    for setting in tool.settings {
        answers.push((setting, ask(setting)?));
    }

    match &tool.target {
        ConfigTarget::ProjectToml { filename } => write_toml(&project_dir, filename, &answers)?,
        ConfigTarget::VsCodeSettings => write_vscode_settings(&project_dir, &answers)?,
    }
    Ok(())
}

fn pick_tool() -> Result<&'static ConfigurableTool> {
    let options: Vec<String> = CONFIGURABLE_TOOLS
        .iter()
        .map(|t| format!("{} - {}", t.key, t.display_name))
        .collect();
    let picked = Select::new("Which tool do you want to configure?", options)
        .with_formatter(&|o: inquire::list_option::ListOption<&String>| {
            o.value.split(" - ").next().unwrap_or(o.value).to_string()
        })
        .prompt()?;
    let key = picked.split(" - ").next().unwrap_or(&picked);
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
fn ask(setting: &SettingSpec) -> Result<Value> {
    println!("{}\n  {}", setting.key, setting.description);

    let value = match &setting.kind {
        SettingKind::Bool { default } => {
            let answer = Confirm::new("  Enable?").with_default(*default).prompt()?;
            json!(answer)
        }
        SettingKind::Integer { default } => {
            let answer = CustomType::<i64>::new("  Value:").with_default(*default).prompt()?;
            json!(answer)
        }
        SettingKind::Choice { default, options } => {
            let labels: Vec<String> =
                options.iter().map(|o| format!("{} - {}", o.value, o.explanation)).collect();
            let start = options.iter().position(|o| o.value == *default).unwrap_or(0);
            let picked = Select::new("  Value:", labels)
                .with_starting_cursor(start)
                .with_formatter(&|o: inquire::list_option::ListOption<&String>| {
                    o.value.split(" - ").next().unwrap_or(o.value).to_string()
                })
                .prompt()?;
            json!(picked.split(" - ").next().unwrap_or(&picked))
        }
    };

    println!();
    Ok(value)
}

/// Writes a TOML file, grouping `section`-tagged settings into their table.
/// Top-level keys have to be emitted before any `[table]` header, since in
/// TOML every key after a header belongs to that table.
fn write_toml(project_dir: &Path, filename: &str, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let mut out = String::new();

    for (setting, value) in answers.iter().filter(|(s, _)| s.section.is_none()) {
        out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
    }

    let mut sections: Vec<&str> =
        answers.iter().filter_map(|(s, _)| s.section).collect();
    sections.dedup();
    for section in sections {
        out.push_str(&format!("\n[{section}]\n"));
        for (setting, value) in answers.iter().filter(|(s, _)| s.section == Some(section)) {
            out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
        }
    }

    let path = project_dir.join(filename);
    fs::write(&path, out).with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote {filename}");
    Ok(())
}

fn toml_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

/// Merges into `.vscode/settings.json` rather than overwriting it - the
/// file routinely holds unrelated editor preferences, and several catalog
/// tools write into this same file, so each run must leave the other keys
/// alone.
fn write_vscode_settings(project_dir: &Path, answers: &[(&SettingSpec, Value)]) -> Result<()> {
    let dir = project_dir.join(".vscode");
    fs::create_dir_all(&dir)?;
    let path = dir.join("settings.json");

    let mut settings = if path.exists() {
        let text = fs::read_to_string(&path)?;
        match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => map,
            Ok(_) => bail!("{} exists but isn't a JSON object", path.display()),
            // VS Code tolerates comments and trailing commas in this file;
            // serde_json does not. Refuse rather than silently discarding
            // settings we failed to parse.
            Err(err) => bail!(
                "could not parse {} ({err}). It may contain comments or trailing commas, \
                 which this writer can't preserve - fix or move the file, then re-run.",
                path.display()
            ),
        }
    } else {
        serde_json::Map::new()
    };

    for (setting, value) in answers {
        settings.insert(setting.key.to_string(), value.clone());
    }

    fs::write(&path, serde_json::to_string_pretty(&Value::Object(settings))?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote .vscode/settings.json");
    Ok(())
}
