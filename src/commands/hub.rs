use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use super::catalog_browser::{CatalogApp, CatalogExit};
use super::projects::{BrowserOutcome, ProjectAction, ProjectsApp};
use crate::config::{GlobalConfig, Setups, project_template};
use crate::steps::update_check::{self, UpdateStatus};
use crate::tui::{
    ACCENT, InputState, TerminalSession, centered, is_too_small, render_footer, render_input,
    responsive_panes, selected_style, title_style,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubOutcome {
    Quit,
    New {
        name: String,
    },
    EditProjectTemplate,
    SetupMachine,
    Project {
        path: PathBuf,
        action: ProjectAction,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Projects,
    New,
    Template,
    Setup,
    Catalog,
}

const ACTIONS: &[(Action, &str, &str)] = &[
    (
        Action::Projects,
        "Projects",
        "Browse projects and choose project actions.",
    ),
    (
        Action::New,
        "New Project",
        "Scaffold a project under your configured RobloxProjects folder.",
    ),
    (
        Action::Template,
        "Edit Project Template",
        "Customize the validated Rojo tree inherited by future projects.",
    ),
    (
        Action::Setup,
        "Machine Setup",
        "Install or revise machine-wide apps, tools, plugins, and extensions.",
    ),
    (
        Action::Catalog,
        "Catalog",
        "Browse packages, tools, capabilities, generated files, and topics.",
    ),
];

struct WorkspaceContext {
    cwd: PathBuf,
    machine: String,
    machine_ready: bool,
    setups: Vec<String>,
    template: String,
    projects_root: std::result::Result<PathBuf, String>,
    warnings: Vec<String>,
}

impl WorkspaceContext {
    fn load(cwd: PathBuf) -> Self {
        let mut warnings = Vec::new();
        let mut machine_ready = false;
        let machine = match GlobalConfig::load() {
            Ok(config) if config.machine_configured() => {
                machine_ready = true;
                format!("configured: {}", config.machine_summary())
            }
            Ok(_) => "not configured".into(),
            Err(error) => {
                warnings.push(format!("Machine config: {error:#}"));
                "unavailable".into()
            }
        };
        let projects_root = GlobalConfig::load()
            .and_then(|config| config.projects_root())
            .map_err(|error| format!("{error:#}"));
        let template = match project_template::path() {
            Ok(path) if path.is_file() => match project_template::load() {
                Ok(Some(_)) => "custom template".into(),
                Ok(None) => "built-in template".into(),
                Err(error) => {
                    warnings.push(format!("Project template: {error:#}"));
                    "custom template needs repair".into()
                }
            },
            Ok(_) => "built-in template".into(),
            Err(error) => {
                warnings.push(format!("Project template: {error:#}"));
                "unavailable".into()
            }
        };
        Self {
            cwd,
            machine,
            machine_ready,
            setups: Setups::list(),
            template,
            projects_root,
            warnings,
        }
    }

    fn availability(&self, action: Action) -> std::result::Result<(), &'static str> {
        if action == Action::New && !self.machine_ready {
            Err("Run Machine Setup first; New Project does not install machine applications.")
        } else {
            Ok(())
        }
    }
}

enum Screen {
    Hub,
    Catalog(CatalogApp),
    Projects,
}

enum Modal {
    Help,
    ProjectName(InputState),
}

struct HubApp {
    context: WorkspaceContext,
    projects: ProjectsApp,
    selected: usize,
    screen: Screen,
    modal: Option<Modal>,
    status: String,
    update: UpdateStatus,
    update_receiver: Option<Receiver<UpdateStatus>>,
}

impl HubApp {
    fn new(context: WorkspaceContext) -> Self {
        let (update, update_receiver) = update_check::background_status();
        Self {
            projects: ProjectsApp::new(context.cwd.clone()),
            context,
            selected: 0,
            screen: Screen::Hub,
            modal: None,
            status: "Choose a task. Direct commands remain available for scripts.".into(),
            update,
            update_receiver,
        }
    }

    fn refresh_update(&mut self) {
        let received = self.update_receiver.as_ref().map(Receiver::try_recv);
        match received {
            Some(Ok(status)) => {
                self.update = status;
                self.update_receiver = None;
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.update = UpdateStatus::Unknown;
                self.update_receiver = None;
            }
            Some(Err(TryRecvError::Empty)) | None => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<HubOutcome> {
        if matches!(self.screen, Screen::Projects) {
            return match self.projects.handle_key(key) {
                Some(BrowserOutcome::Home) => {
                    self.screen = Screen::Hub;
                    crate::diagnostics::event("screen", "Home");
                    None
                }
                Some(BrowserOutcome::Run { path, action }) => {
                    Some(HubOutcome::Project { path, action })
                }
                None => None,
            };
        }
        if let Screen::Catalog(catalog) = &mut self.screen {
            return match catalog.handle_key(key) {
                Some(CatalogExit::Back | CatalogExit::Quit) => {
                    self.screen = Screen::Hub;
                    crate::diagnostics::event("screen", "Home");
                    None
                }
                None => None,
            };
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(HubOutcome::Quit);
        }
        if let Some(modal) = self.modal.take() {
            return self.handle_modal(modal, key);
        }
        match key.code {
            KeyCode::Esc => Some(HubOutcome::Quit),
            KeyCode::Char('?') => {
                self.modal = Some(Modal::Help);
                None
            }
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                None
            }
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(ACTIONS.len() - 1);
                None
            }
            KeyCode::Enter => self.activate(),
            _ => None,
        }
    }

    fn handle_modal(&mut self, modal: Modal, key: KeyEvent) -> Option<HubOutcome> {
        match modal {
            Modal::Help => {
                if !matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                    self.modal = Some(Modal::Help);
                }
                None
            }
            Modal::ProjectName(mut input) => match key.code {
                KeyCode::Esc => None,
                KeyCode::Enter => {
                    let name = input.text().trim();
                    if let Err(error) = super::creation::validate_name(name) {
                        input.error = Some(error.into());
                        self.modal = Some(Modal::ProjectName(input));
                        None
                    } else {
                        Some(HubOutcome::New { name: name.into() })
                    }
                }
                _ => {
                    input.handle_key(key);
                    self.modal = Some(Modal::ProjectName(input));
                    None
                }
            },
        }
    }

    fn activate(&mut self) -> Option<HubOutcome> {
        let action = ACTIONS[self.selected].0;
        crate::diagnostics::event("hub.action", format!("{action:?}"));
        if let Err(reason) = self.context.availability(action) {
            crate::diagnostics::event("hub.unavailable", reason);
            self.status = reason.into();
            return None;
        }
        match action {
            Action::Projects => {
                self.projects.open(self.context.projects_root.clone());
                self.screen = Screen::Projects;
                crate::diagnostics::event("screen", "Projects");
                None
            }
            Action::New => {
                self.modal = Some(Modal::ProjectName(InputState::new("")));
                None
            }
            Action::Template => Some(HubOutcome::EditProjectTemplate),
            Action::Setup => Some(HubOutcome::SetupMachine),
            Action::Catalog => {
                self.screen = Screen::Catalog(CatalogApp::new());
                crate::diagnostics::event("screen", "Catalog");
                None
            }
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>) {
        if matches!(self.screen, Screen::Projects) {
            self.projects.render(frame);
            return;
        }
        if let Screen::Catalog(catalog) = &self.screen {
            catalog.render(frame);
            return;
        }
        let area = frame.area();
        if is_too_small(area) {
            frame.render_widget(
                Paragraph::new("rproj workspace\n\nTerminal is too small. Resize to at least 60 x 16.\n\n? help   Esc exit")
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true })
                    .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))),
                area,
            );
            if matches!(self.modal, Some(Modal::Help)) {
                self.render_help(frame, area);
            }
            return;
        }
        let outer = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("rproj {}", env!("CARGO_PKG_VERSION")),
                    title_style(),
                ),
                Span::raw(format!("  {}", self.context.cwd.display())),
            ]))
            .block(Block::default().borders(Borders::BOTTOM)),
            outer[0],
        );
        let panes = responsive_panes(outer[1], 40);
        self.render_actions(frame, panes[0]);
        self.render_context(frame, panes[1]);
        render_footer(
            frame,
            outer[2],
            &self.status,
            "Arrows navigate  Enter open  Esc exit  ? help",
            self.status.contains("requires") || self.status.starts_with("No project"),
        );
        match &self.modal {
            Some(Modal::Help) => self.render_help(frame, area),
            Some(Modal::ProjectName(input)) => {
                render_input(frame, area, "New project", "Project folder name", input)
            }
            None => {}
        }
    }

    fn render_actions(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let items = ACTIONS.iter().map(|(action, label, _)| {
            let unavailable = self.context.availability(*action).is_err();
            ListItem::new(format!(
                "{}{}",
                label,
                if unavailable { "  [unavailable]" } else { "" }
            ))
            .style(Style::default().fg(if unavailable {
                Color::DarkGray
            } else {
                Color::White
            }))
        });
        let mut state = ListState::default().with_selected(Some(self.selected));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(" Tasks ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(ACCENT)),
                )
                .highlight_style(selected_style(true)),
            area,
            &mut state,
        );
    }

    fn render_context(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let (action, label, description) = ACTIONS[self.selected];
        let availability = self
            .context
            .availability(action)
            .err()
            .map(|reason| format!("\n\nUnavailable: {reason}"))
            .unwrap_or_default();
        let setups = if self.context.setups.is_empty() {
            "none".into()
        } else {
            self.context.setups.join(", ")
        };
        let warnings = if self.context.warnings.is_empty() {
            String::new()
        } else {
            format!("\n\nWarnings\n{}", self.context.warnings.join("\n"))
        };
        let body = format!(
            "{description}{availability}\n\nWorkspace\n{}\n\nMachine\n{}\n\nTemplate\n{}\n\nSaved setups\n{}\n\nVersion\n{}{}",
            self.context
                .projects_root
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|error| error.clone()),
            self.context.machine,
            self.context.template,
            setups,
            self.update.label(),
            warnings
        );
        frame.render_widget(
            Paragraph::new(body).wrap(Wrap { trim: false }).block(
                Block::default()
                    .title(format!(" {label} "))
                    .borders(Borders::ALL),
            ),
            area,
        );
    }

    fn render_help(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let popup = centered(area, 68, 12);
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new("Workspace hub\n  Shows the current directory, machine setup, saved setups, template, and version state.\n\nNavigation\n  Up/Down select; Enter opens; Esc exits\n\nCommands\n  Selected commands run after rproj restores the terminal. Direct CLI commands remain scriptable.")
                .wrap(Wrap { trim: false })
                .block(Block::default().title(" Help ").borders(Borders::ALL).border_style(Style::default().fg(ACCENT))),
            popup,
        );
    }
}

pub fn run() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!("the rproj workspace hub requires an interactive terminal");
    }
    let cwd = std::env::current_dir().context("could not read the current directory")?;
    crate::interrupt::install()?;
    crate::diagnostics::quiet_hub();
    let mut app = HubApp::new(WorkspaceContext::load(cwd));
    {
        let mut terminal = TerminalSession::enter()?;
        loop {
            app.refresh_update();
            app.projects.poll();
            let mut small = false;
            terminal.draw(|frame| {
                small = is_too_small(frame.area());
                app.render(frame);
            })?;
            let event = if app.update_receiver.is_some() || app.projects.loading() {
                terminal.poll_event(Duration::from_millis(150))?
            } else {
                Some(terminal.read_event()?)
            };
            if let Some(Event::Paste(text)) = &event
                && matches!(app.screen, Screen::Projects)
            {
                app.projects.paste(text);
            }
            if let Some(Event::Paste(text)) = &event
                && let Some(Modal::ProjectName(input)) = &mut app.modal
            {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    input.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
                }
            }
            if let Some(Event::Key(key)) = event
                && key.kind != crossterm::event::KeyEventKind::Release
                && (!small
                    || matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
                    || (key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')))
                && let Some(outcome) = app.handle_key(key)
            {
                if outcome == HubOutcome::Quit {
                    break;
                }
                crate::diagnostics::event(
                    "hub.dispatch",
                    match &outcome {
                        HubOutcome::New { .. } => "New (name omitted)".into(),
                        outcome => format!("{outcome:?}"),
                    },
                );
                crate::interrupt::reset();
                let mut acknowledged = false;
                let mut cancelled = false;
                let project_action = matches!(outcome, HubOutcome::Project { .. });
                let watch = matches!(
                    outcome,
                    HubOutcome::Project {
                        action: ProjectAction::Watch,
                        ..
                    }
                );
                let result = match outcome {
                    HubOutcome::EditProjectTemplate => {
                        super::project_template::run_in(&mut terminal)
                    }
                    HubOutcome::New { name } => {
                        match super::creation::prepare(&mut terminal, &name) {
                            Ok(Some(prepared)) => {
                                terminal.suspend()?;
                                let path = prepared.project_dir();
                                let result = prepared.execute();
                                acknowledge(&result, false, result.is_ok())?;
                                if result.is_ok() {
                                    app.projects
                                        .show_created(path, app.context.projects_root.clone());
                                    app.screen = Screen::Projects;
                                }
                                acknowledged = true;
                                terminal.resume()?;
                                result
                            }
                            Ok(None) => {
                                cancelled = true;
                                Ok(())
                            }
                            Err(error) => Err(error),
                        }
                    }
                    outcome => {
                        terminal.suspend()?;
                        let result = crate::dispatch_hub(outcome);
                        if result
                            .as_ref()
                            .is_err_and(|error| error.is::<crate::tui::TerminalFailure>())
                        {
                            return result;
                        }
                        acknowledge(&result, watch, project_action)?;
                        acknowledged = true;
                        terminal.resume()?;
                        result
                    }
                };
                if result
                    .as_ref()
                    .is_err_and(|error| error.is::<crate::tui::TerminalFailure>())
                {
                    return result;
                }
                if result.is_err() && !acknowledged {
                    terminal.suspend()?;
                    acknowledge(&result, false, false)?;
                    terminal.resume()?;
                }
                app.status = action_status(&result, cancelled, watch).into();
                crate::diagnostics::event("hub.result", &app.status);
                crate::interrupt::reset();
                app.context = WorkspaceContext::load(app.context.cwd.clone());
                if matches!(app.screen, Screen::Projects) {
                    app.projects.status = app.status.clone();
                    app.projects.refresh_active();
                    crate::diagnostics::event("screen", "Project");
                } else {
                    crate::diagnostics::event("screen", "Home");
                }
            }
        }
    }
    Ok(())
}

fn action_status(result: &Result<()>, cancelled: bool, watch: bool) -> &'static str {
    match result {
        Err(error) if crate::interrupt::is_cancelled(error) => "Cancelled.",
        Err(_) => "Failed. Details recorded in the diagnostic log.",
        Ok(()) if cancelled => "Cancelled. Nothing created.",
        Ok(()) if watch => "Stopped.",
        Ok(()) => "Completed.",
    }
}

fn acknowledge(result: &Result<()>, watch: bool, project: bool) -> Result<()> {
    use std::io::Write;
    match result {
        Err(error) if crate::interrupt::is_cancelled(error) => println!("\nCancelled."),
        Err(error) => {
            crate::ui::error(error);
            eprintln!("{}", crate::diagnostics::path_message());
        }
        Ok(()) => println!("\n{}", if watch { "Stopped." } else { "Completed." }),
    }
    print!(
        "Press Enter to return {}.",
        if project { "the project" } else { "Home" }
    );
    std::io::stdout().flush()?;
    crate::interrupt::reset();
    let mut input = String::new();
    loop {
        match std::io::stdin().read_line(&mut input) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => {
                result?;
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::path::Path;
    use tempfile::TempDir;

    fn app_at(path: &Path) -> HubApp {
        let mut app = HubApp::new(WorkspaceContext::load(path.to_path_buf()));
        app.update_receiver = None;
        app.update = UpdateStatus::Current;
        app
    }

    #[test]
    fn creation_requires_setup_and_rejects_invalid_names_without_leaving() {
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        app.context.machine_ready = false;
        assert!(
            app.context
                .availability(Action::New)
                .unwrap_err()
                .contains("Machine Setup")
        );
        assert!(app.context.availability(Action::Setup).is_ok());
        app.context.machine_ready = true;
        app.modal = Some(Modal::ProjectName(InputState::new("../outside")));
        assert!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .is_none()
        );
        assert!(matches!(app.modal, Some(Modal::ProjectName(ref input)) if input.error.is_some()));
    }

    fn rendered(app: &HubApp, width: u16, height: u16) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn wide_narrow_help_and_tiny_states_render() {
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        assert!(rendered(&app, 120, 30).contains("Tasks"));
        assert!(rendered(&app, 80, 24).contains("Workspace"));
        assert!(rendered(&app, 40, 10).contains("too small"));
        app.modal = Some(Modal::Help);
        assert!(rendered(&app, 120, 30).contains("Workspace hub"));
    }

    #[test]
    fn home_contains_only_machine_destinations() {
        assert_eq!(
            ACTIONS
                .iter()
                .map(|(_, label, _)| *label)
                .collect::<Vec<_>>(),
            [
                "Projects",
                "New Project",
                "Edit Project Template",
                "Machine Setup",
                "Catalog"
            ]
        );
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        app.context.machine_ready = false;
        assert!(app.context.availability(Action::Projects).is_ok());
        assert!(app.activate().is_none());
        assert!(matches!(app.screen, Screen::Projects));
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.screen, Screen::Hub));
    }

    #[test]
    fn new_project_requires_a_nonempty_unicode_name() {
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        app.context.machine_ready = true;
        app.selected = 1;
        app.activate();
        assert!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .is_none()
        );
        for character in "café".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        assert_eq!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(HubOutcome::New {
                name: "café".into()
            })
        );
    }
}
