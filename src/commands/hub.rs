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
use crate::config::{GlobalConfig, PackageWorkflow, Setups, project_file, project_template};
use crate::graph::ProjectGraph;
use crate::steps::update_check::{self, UpdateStatus};
use crate::tui::{
    ACCENT, InputState, TerminalSession, centered, is_too_small, render_footer, render_input,
    responsive_panes, selected_style, title_style,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HubOutcome {
    Quit,
    New { name: String },
    EditProjectTemplate,
    ConfigureTools,
    SetupMachine,
    Upgrade,
    Watch,
    Test,
    CopySource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    New,
    Template,
    Configure,
    Setup,
    Upgrade,
    Watch,
    Test,
    Copy,
    Catalog,
}

const ACTIONS: &[(Action, &str, &str)] = &[
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
        Action::Configure,
        "Configure Tools",
        "Walk through settings for a tool used by this project.",
    ),
    (
        Action::Setup,
        "Machine Setup",
        "Install or revise machine-wide apps, tools, plugins, and extensions.",
    ),
    (
        Action::Upgrade,
        "Upgrade Project",
        "Re-apply current generated configuration to this project.",
    ),
    (
        Action::Watch,
        "Watch Project",
        "Install missing dependencies and start the Rojo sourcemap watcher.",
    ),
    (
        Action::Test,
        "Test Project",
        "Restore dependencies and run the test runner recorded by this project.",
    ),
    (
        Action::Copy,
        "Copy Source",
        "Copy source files with relative-path headers to the clipboard.",
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
    has_project_file: bool,
    has_rojo_file: bool,
    has_src: bool,
    graph: Option<ProjectGraph>,
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
        let project_path = project_file::path_in(&cwd);
        let has_project_file = project_path.is_file();
        let graph = match project_file::load_from(&cwd) {
            Ok(graph) => graph,
            Err(error) => {
                warnings.push(format!("Project config: {error:#}"));
                None
            }
        };
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
            has_rojo_file: cwd.join("default.project.json").is_file(),
            has_src: cwd.join("src").is_dir(),
            cwd,
            machine,
            machine_ready,
            setups: Setups::list(),
            template,
            has_project_file,
            graph,
            warnings,
        }
    }

    fn recognized_project(&self) -> bool {
        self.has_project_file || self.has_rojo_file
    }

    fn availability(&self, action: Action) -> std::result::Result<(), &'static str> {
        match action {
            Action::New if !self.machine_ready => {
                Err("Run Machine Setup first; New Project does not install machine applications.")
            }
            Action::Configure if !self.recognized_project() => {
                Err("No project here: default.project.json or rproj.toml is required.")
            }
            Action::Upgrade if !self.has_project_file => {
                Err("Upgrade requires rproj.toml in the current directory.")
            }
            Action::Upgrade if self.graph.is_none() => {
                Err("rproj.toml is invalid; repair it before upgrading.")
            }
            Action::Watch if !self.has_rojo_file => {
                Err("Watch requires default.project.json in the current directory.")
            }
            Action::Test if !self.has_project_file => {
                Err("Test requires rproj.toml in the current directory.")
            }
            Action::Test if self.graph.is_none() => {
                Err("rproj.toml is invalid; repair it before testing.")
            }
            Action::Test
                if self
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.test_runner())
                    .is_none() =>
            {
                Err("Testing is not enabled in this project.")
            }
            Action::Test
                if self
                    .graph
                    .as_ref()
                    .is_some_and(|graph| !graph.testing_is_compatible()) =>
            {
                Err("Jest Roblox requires the Wally dependency workflow.")
            }
            Action::Copy if !self.has_src => {
                Err("Copy Source requires a src directory in the current directory.")
            }
            _ => Ok(()),
        }
    }

    fn project_summary(&self) -> String {
        match &self.graph {
            Some(graph) => format!(
                "recognized\nmode: {}\npackages: {} via {}\ncapabilities: {}",
                nonempty(&graph.mode, "unspecified"),
                graph.packages.len(),
                workflow_name(graph.package_workflow),
                if graph.capabilities.is_empty() {
                    "none".into()
                } else {
                    graph.capability_keys().join(", ")
                }
            ),
            None if self.has_project_file => "rproj.toml is invalid".into(),
            None if self.has_rojo_file => "Rojo project without rproj.toml".into(),
            None => "No project detected in this exact directory.".into(),
        }
    }
}

fn nonempty<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.is_empty() { fallback } else { value }
}

fn workflow_name(workflow: PackageWorkflow) -> &'static str {
    match workflow {
        PackageWorkflow::Wally => "Wally",
        PackageWorkflow::GitSubmodules => "git submodules",
        PackageWorkflow::None => "no package manager",
    }
}

enum Screen {
    Hub,
    Catalog(CatalogApp),
}

enum Modal {
    Help,
    ProjectName(InputState),
}

struct HubApp {
    context: WorkspaceContext,
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
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(HubOutcome::Quit);
        }
        if let Screen::Catalog(catalog) = &mut self.screen {
            return match catalog.handle_key(key) {
                Some(CatalogExit::Back) => {
                    self.screen = Screen::Hub;
                    None
                }
                Some(CatalogExit::Quit) => Some(HubOutcome::Quit),
                None => None,
            };
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
            Action::New => {
                self.modal = Some(Modal::ProjectName(InputState::new("")));
                None
            }
            Action::Template => Some(HubOutcome::EditProjectTemplate),
            Action::Configure => Some(HubOutcome::ConfigureTools),
            Action::Setup => Some(HubOutcome::SetupMachine),
            Action::Upgrade => Some(HubOutcome::Upgrade),
            Action::Watch => Some(HubOutcome::Watch),
            Action::Test => Some(HubOutcome::Test),
            Action::Copy => Some(HubOutcome::CopySource),
            Action::Catalog => {
                self.screen = Screen::Catalog(CatalogApp::new());
                None
            }
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>) {
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
            self.context.project_summary(),
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

pub fn run() -> Result<HubOutcome> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!("the rproj workspace hub requires an interactive terminal");
    }
    let cwd = std::env::current_dir().context("could not read the current directory")?;
    let mut app = HubApp::new(WorkspaceContext::load(cwd));
    let outcome = {
        let mut terminal = TerminalSession::enter()?;
        loop {
            app.refresh_update();
            terminal.draw(|frame| app.render(frame))?;
            let event = if app.update_receiver.is_some() {
                terminal.poll_event(Duration::from_millis(150))?
            } else {
                Some(terminal.read_event()?)
            };
            if let Some(Event::Paste(text)) = &event
                && let Some(Modal::ProjectName(input)) = &mut app.modal
            {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    input.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
                }
            }
            if let Some(Event::Key(key)) = event
                && key.kind != crossterm::event::KeyEventKind::Release
                && let Some(outcome) = app.handle_key(key)
            {
                break outcome;
            }
        }
    };
    Ok(outcome)
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
    fn project_actions_stay_visible_and_explain_why_they_are_disabled() {
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        app.selected = ACTIONS
            .iter()
            .position(|(action, _, _)| *action == Action::Watch)
            .unwrap();
        assert!(rendered(&app, 120, 30).contains("[unavailable]"));
        assert!(
            app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
                .is_none()
        );
        assert!(app.status.contains("default.project.json"));
    }

    #[test]
    fn test_action_tracks_the_recorded_runner() {
        let temp = TempDir::new().unwrap();
        let mut graph = ProjectGraph::default();
        graph.choose("test", Some("testez"));
        crate::config::project_file::save_to(&graph, temp.path()).unwrap();
        let mut app = app_at(temp.path());
        app.selected = ACTIONS
            .iter()
            .position(|(action, _, _)| *action == Action::Test)
            .unwrap();
        assert_eq!(app.activate(), Some(HubOutcome::Test));

        let empty = TempDir::new().unwrap();
        crate::config::project_file::save_to(&ProjectGraph::default(), empty.path()).unwrap();
        let context = WorkspaceContext::load(empty.path().to_path_buf());
        assert_eq!(
            context.availability(Action::Test),
            Err("Testing is not enabled in this project.")
        );
    }

    #[test]
    fn new_project_requires_a_nonempty_unicode_name() {
        let temp = TempDir::new().unwrap();
        let mut app = app_at(temp.path());
        app.context.machine_ready = true;
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

    #[test]
    fn exact_directory_project_detection_does_not_walk_upward() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("default.project.json"), "{}").unwrap();
        let child = temp.path().join("src");
        std::fs::create_dir(&child).unwrap();
        let parent = WorkspaceContext::load(temp.path().to_path_buf());
        let nested = WorkspaceContext::load(child);
        assert!(parent.recognized_project());
        assert!(!nested.recognized_project());
    }
}
