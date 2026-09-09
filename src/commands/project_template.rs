use anyhow::Result;

use crate::config::project_template;
use crate::project_editor;
use crate::steps::rojo;

pub fn run() -> Result<()> {
    project_editor::run(draft()?, save, || project_template::reset().map(|_| ()))?;
    Ok(())
}

pub fn run_in(terminal: &mut crate::tui::TerminalSession) -> Result<()> {
    project_editor::run_in(terminal, draft()?, save, || {
        project_template::reset().map(|_| ())
    })?;
    Ok(())
}

fn draft() -> Result<String> {
    Ok(match project_template::read_text()? {
        Some(text) => text,
        None => format!(
            "{}\n",
            serde_json::to_string_pretty(&rojo::builtin_project_template())?
        ),
    })
}

fn save(value: &serde_json::Value) -> Result<()> {
    rojo::validate_template_with_rojo(value)?;
    project_template::save(value)?;
    Ok(())
}
