pub(super) mod model;
pub(super) mod render;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result, bail};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

use super::new;
use crate::{
    config::{GlobalConfig, Setups, project_file},
    graph::ProjectGraph,
    tui,
};
use model::{Draft, Effect};

pub(super) use model::validate_name;

pub struct Prepared {
    draft: Draft,
    root: std::path::PathBuf,
    template: Option<serde_json::Value>,
    config: GlobalConfig,
}

pub struct ProjectEdit {
    pub source: String,
    pub original: ProjectGraph,
    pub updated: ProjectGraph,
}

impl Prepared {
    pub fn project_dir(&self) -> std::path::PathBuf {
        self.root.join(&self.draft.name)
    }
}

pub fn prepare(terminal: &mut tui::TerminalSession, name: &str) -> Result<Option<Prepared>> {
    terminal.draw(|frame| {
        frame.render_widget(
            ratatui::widgets::Paragraph::new(
                "Preparing New Project...\nValidating the project template.",
            ),
            frame.area(),
        )
    })?;
    model::validate_name(name).map_err(anyhow::Error::msg)?;
    let config = GlobalConfig::load()?;
    if !config.machine_configured() {
        bail!("Run Machine Setup from the hub, or `rproj setup`, before creating a project here.");
    }
    let template =
        std::thread::scope(|scope| -> Result<Option<Option<serde_json::Value>>> {
            let (sender, receiver) = std::sync::mpsc::channel();
            let config = &config;
            scope.spawn(move || {
                let _ = sender.send(new::prepare_project(config, name));
            });
            let mut cancelled = false;
            loop {
                if let Some(Event::Key(key)) =
                    terminal.poll_event(std::time::Duration::from_millis(50))?
                    && key.kind != KeyEventKind::Release
                    && (key.code == KeyCode::Esc
                        || (key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c')))
                {
                    cancelled = true;
                    crate::interrupt::request();
                }
                terminal.draw(|frame| {
                    frame.render_widget(
                ratatui::widgets::Paragraph::new(if cancelled {
                    "Cancelling preparation... Waiting for the active validation process."
                } else {
                    "Preparing New Project...\nValidating the project template.\nEsc cancels."
                }).wrap(ratatui::widgets::Wrap { trim: false }), frame.area())
                })?;
                match receiver.try_recv() {
                    Ok(_) if cancelled => return Ok(None),
                    Ok(result) => return result.map(|(_, template)| Some(template)),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        bail!("Project preparation stopped unexpectedly")
                    }
                }
            }
        })?;
    let Some(template) = template else {
        crate::diagnostics::event("creation.cancel", "preparation cancelled; nothing created");
        return Ok(None);
    };
    let root = config.projects_root()?;
    let mut draft = Draft::new(
        name,
        Setups::list(),
        config.selected_system_apps.clone(),
        config.selected_vscode_extensions.clone(),
    );
    let create = {
        loop {
            let destination = root.join(&draft.name).display().to_string();
            let effect = next_effect(terminal, &mut draft, &destination)?;
            match effect {
                Effect::None => {}
                Effect::Save => anyhow::bail!("Setup save is unavailable in project creation"),
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
        return Ok(None);
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
    Ok(Some(Prepared {
        draft,
        root,
        template,
        config,
    }))
}

pub fn edit_project_in(
    terminal: &mut tui::TerminalSession,
    project_dir: &std::path::Path,
) -> Result<Option<ProjectEdit>> {
    let record_path = project_file::path_in(project_dir);
    if !record_path.is_file() {
        anyhow::bail!(
            "no rproj.toml in {}; project composition editing needs a recorded project",
            project_dir.display()
        );
    }
    let source = std::fs::read_to_string(&record_path)
        .with_context(|| format!("failed to read {}", record_path.display()))?;
    let graph = project_file::load_from(project_dir)?.ok_or_else(|| {
        anyhow::anyhow!(
            "no rproj.toml in {}; project composition editing needs a recorded project",
            project_dir.display()
        )
    })?;
    let config = GlobalConfig::load().ok();
    let name = project_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project");
    let original = graph.clone();
    let mut draft = Draft::edit_project(
        name,
        graph,
        config
            .as_ref()
            .map(|config| config.selected_system_apps.clone())
            .unwrap_or_default(),
        config
            .as_ref()
            .map(|config| config.selected_vscode_extensions.clone())
            .unwrap_or_default(),
    );
    loop {
        let effect = next_effect(terminal, &mut draft, &project_dir.display().to_string())?;
        match effect {
            Effect::None => {}
            Effect::Save => {
                return Ok(Some(ProjectEdit {
                    source,
                    original,
                    updated: draft.graph,
                }));
            }
            Effect::Cancel => return Ok(None),
            Effect::Create | Effect::LoadSetup(_) => {
                draft.status = "That action is unavailable while editing a project.".into()
            }
        }
    }
}

fn next_effect(
    terminal: &mut tui::TerminalSession,
    draft: &mut Draft,
    destination: &str,
) -> Result<Effect> {
    let mut small = false;
    terminal.draw(|frame| {
        small = tui::is_too_small(frame.area());
        render::draw(frame, draft, destination);
    })?;
    Ok(match terminal.read_event()? {
        Event::Key(key) if key.kind != KeyEventKind::Release => {
            if small
                && !matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
                && !(key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('c'))
            {
                Effect::None
            } else {
                draft.key(key)
            }
        }
        Event::Paste(text) if !small => {
            draft.paste(&text);
            Effect::None
        }
        Event::Mouse(mouse) if !small => {
            match mouse.kind {
                crossterm::event::MouseEventKind::ScrollUp => {
                    draft.scroll = draft.scroll.saturating_sub(3)
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    draft.scroll = draft.scroll.saturating_add(3)
                }
                _ => {}
            }
            Effect::None
        }
        _ => Effect::None,
    })
}

impl Prepared {
    pub fn execute(self) -> Result<()> {
        let Self {
            draft,
            root,
            template,
            mut config,
        } = self;
        crate::interrupt::check()?;
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
}
