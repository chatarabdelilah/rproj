use anyhow::Result;
use std::cell::RefCell;

use crate::config::project_template;
use crate::project_editor;
use crate::steps::rojo;

pub fn run() -> Result<()> {
    let session = RefCell::new(project_template::EditSession::open()?);
    let text = draft(&session.borrow())?;
    project_editor::run(
        text,
        |value| save(&session, value),
        || session.borrow_mut().reset().map(|_| ()),
    )?;
    Ok(())
}

pub fn run_in(terminal: &mut crate::tui::TerminalSession) -> Result<()> {
    let session = RefCell::new(project_template::EditSession::open()?);
    let text = draft(&session.borrow())?;
    project_editor::run_in(
        terminal,
        text,
        |value| save(&session, value),
        || session.borrow_mut().reset().map(|_| ()),
    )?;
    Ok(())
}

fn draft(session: &project_template::EditSession) -> Result<String> {
    Ok(match session.text() {
        Some(text) => text.to_owned(),
        None => format!(
            "{}\n",
            serde_json::to_string_pretty(&rojo::builtin_project_template())?
        ),
    })
}

fn save(session: &RefCell<project_template::EditSession>, value: &serde_json::Value) -> Result<()> {
    session
        .borrow_mut()
        .save(value, rojo::validate_template_with_rojo)?;
    Ok(())
}
