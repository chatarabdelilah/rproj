use anyhow::{Context, Result};
use inquire::{Confirm, Editor, Select};

use crate::config::project_template;
use crate::steps::rojo;
use crate::ui;

const EDIT: &str = "Edit the current template";
const RESET: &str = "Restore rproj's built-in template";
const CANCEL: &str = "Cancel";
const REOPEN: &str = "Reopen the draft";
const DISCARD: &str = "Discard these changes";

pub fn run() -> Result<()> {
    let saved = project_template::read_text()?;
    if saved.is_some() {
        match Select::new("What do you want to do?", vec![EDIT, RESET, CANCEL]).prompt()? {
            EDIT => {}
            RESET => return reset(),
            _ => {
                ui::skip("nothing changed");
                return Ok(());
            }
        }
    }

    let mut draft = match saved {
        Some(text) => text,
        None => serde_json::to_string_pretty(&rojo::builtin_project_template())?,
    };
    loop {
        let edited = Editor::new("Edit the project template")
            .with_predefined_text(&draft)
            .with_file_extension(".project.json")
            .with_help_message("Press e to edit; close the editor, then press enter to validate")
            .prompt()?;

        match validate(&edited) {
            Ok(template) => {
                let path = project_template::save(&template)?;
                ui::ok(&format!("saved project template to {}", path.display()));
                ui::detail(
                    "future projects will inherit this tree; existing projects are unchanged",
                );
                return Ok(());
            }
            Err(error) => {
                ui::warn(&format!("template was not saved: {error:#}"));
                match Select::new("What next?", vec![REOPEN, DISCARD]).prompt()? {
                    REOPEN if !edited.trim().is_empty() => draft = edited,
                    REOPEN => {}
                    _ => {
                        ui::skip("changes discarded; the last valid template is still active");
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn validate(text: &str) -> Result<serde_json::Value> {
    let template = serde_json::from_str(text).context("invalid JSON")?;
    rojo::validate_template_with_rojo(&template)?;
    Ok(template)
}

fn reset() -> Result<()> {
    if !Confirm::new("Restore the built-in template for future projects?")
        .with_default(false)
        .prompt()?
    {
        ui::skip("nothing changed");
        return Ok(());
    }
    project_template::reset()?;
    ui::ok("restored rproj's built-in project template");
    ui::detail("existing projects are unchanged");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_json_is_rejected_before_rojo_runs() {
        let error = validate("not json").unwrap_err().to_string();
        assert!(error.contains("invalid JSON"), "{error}");
    }
}
