use anyhow::Result;

use crate::config::project_template;
use crate::project_editor::{self, Outcome};
use crate::steps::rojo;
use crate::ui;

pub fn run() -> Result<()> {
    let draft = match project_template::read_text()? {
        Some(text) => text,
        None => format!(
            "{}\n",
            serde_json::to_string_pretty(&rojo::builtin_project_template())?
        ),
    };

    match project_editor::run(draft, rojo::validate_template_with_rojo)? {
        Outcome::Save(template) => {
            let path = project_template::save(&template)?;
            ui::ok(&format!("saved project template to {}", path.display()));
            ui::detail("future projects will inherit this tree; existing projects are unchanged");
        }
        Outcome::Reset => {
            project_template::reset()?;
            ui::ok("restored rproj's built-in project template");
            ui::detail("existing projects are unchanged");
        }
        Outcome::Cancel => ui::skip("nothing changed"),
    }
    Ok(())
}
