mod model;
mod render;
#[cfg(test)]
mod tests;

use std::io::IsTerminal;

use anyhow::{Result, bail};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use super::new;
use crate::{
    config::{GlobalConfig, Setups},
    tui,
};
use model::{Draft, Effect};

pub(super) use model::validate_name;

pub fn run(name: &str) -> Result<()> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        bail!("Interactive creation requires a terminal; use `rproj new <name>`.");
    }
    model::validate_name(name).map_err(anyhow::Error::msg)?;
    let mut config = GlobalConfig::load()?;
    if !config.machine_configured() {
        bail!("Run Machine Setup from the hub, or `rproj setup`, before creating a project here.");
    }
    let (_, template) = new::prepare_project(&config, name)?;
    let root = config.projects_root()?;
    let mut draft = Draft::new(
        name,
        Setups::list(),
        config.selected_system_apps.clone(),
        config.selected_vscode_extensions.clone(),
    );
    let create = {
        let mut terminal = tui::TerminalSession::enter()?;
        loop {
            let mut small = false;
            terminal.draw(|frame| {
                small = tui::is_too_small(frame.area());
                render::draw(frame, &draft, &root.join(&draft.name).display().to_string());
            })?;
            let effect = match terminal.read_event()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    if small
                        && !matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
                        && !(key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c'))
                    {
                        continue;
                    }
                    draft.key(key)
                }
                Event::Paste(text) if !small => {
                    draft.paste(&text);
                    Effect::None
                }
                _ => Effect::None,
            };
            match effect {
                Effect::None => {}
                Effect::Cancel => break false,
                Effect::LoadSetup(name) => match new::read_setup(&name) {
                    Ok((graph, warnings)) => draft.loaded(&name, graph, warnings),
                    Err(error) => {
                        crate::diagnostics::event(
                            "creation.validation",
                            error
                                .to_string()
                                .lines()
                                .next()
                                .unwrap_or("Saved setup could not be loaded"),
                        );
                        draft.status = format!("{error:#}");
                    }
                },
                Effect::Create => {
                    if root.join(&draft.name).exists() {
                        crate::diagnostics::event(
                            "creation.validation",
                            "destination already exists",
                        );
                        draft.status =
                            "Destination already exists. Change the project name.".into();
                    } else if let Some(name) = draft
                        .save_setup
                        .as_deref()
                        .filter(|name| Setups::load(name).map_or(true, |setup| setup.is_some()))
                    {
                        crate::diagnostics::event(
                            "creation.validation",
                            "setup destination exists or is unreadable",
                        );
                        draft.status = format!(
                            "Setup `{name}` now exists or cannot be read. Choose another setup name."
                        );
                    } else {
                        break true;
                    }
                }
            }
        }
    };
    if !create {
        crate::diagnostics::event("creation.cancel", "nothing created");
        println!("Nothing created.");
        return Ok(());
    }
    crate::diagnostics::event(
        "creation.confirmed",
        format!(
            "workflow={:?}; capabilities={:?}; packages={:?}; dropped={:?}",
            draft.graph.package_workflow,
            draft.graph.capabilities,
            draft.graph.packages,
            draft.graph.dropped
        ),
    );
    let planned = draft.graph.plan(&draft.apps, &draft.extensions);
    new::execute_confirmed(
        &draft.name,
        &root.join(&draft.name),
        &draft.graph,
        &planned,
        template.as_ref(),
        &mut config,
        draft.save_setup.as_deref().map(new::SetupSave::New),
    )
}
