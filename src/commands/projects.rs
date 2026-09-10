use std::cell::Cell;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};

use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::config::project_file;
use crate::graph::{ProjectGraph, TestRunner};
use crate::tui::{self, ACCENT, InputState, selected_style, wrap_lines};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectAction {
    Configure,
    Upgrade,
    Watch,
    Test,
    Copy,
}

impl ProjectAction {
    pub const ALL: [Self; 5] = [
        Self::Configure,
        Self::Upgrade,
        Self::Watch,
        Self::Test,
        Self::Copy,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Configure => "Configure Tools",
            Self::Upgrade => "Upgrade Project",
            Self::Watch => "Watch Project",
            Self::Test => "Test Project",
            Self::Copy => "Copy Source",
        }
    }

    pub fn run(self, path: &Path) -> Result<()> {
        crate::interrupt::check()?;
        let context = ProjectContext::load(path.to_owned());
        if let Err(reason) = context.availability(self) {
            bail!("{}: {reason}", path.display());
        }
        crate::diagnostics::event("project.action", format!("{self:?}: {}", path.display()));
        match self {
            Self::Configure => super::configure::run_tools_in(path),
            Self::Upgrade => super::upgrade::run_in(path, false),
            Self::Watch => super::watch::run_in(path),
            Self::Test => super::test::run_in(path, &[]),
            Self::Copy => super::copy::run_in(path),
        }
    }
}

pub struct ProjectContext {
    pub path: PathBuf,
    exists: bool,
    has_rojo: bool,
    has_record: bool,
    has_src: bool,
    has_wally: bool,
    graph: Option<ProjectGraph>,
    warning: Option<String>,
    rojo_warning: Option<String>,
}

impl ProjectContext {
    pub fn load(path: PathBuf) -> Self {
        let (graph, warning) = match project_file::load_from(&path) {
            Ok(graph) => (graph, None),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
        let rojo_path = path.join("default.project.json");
        let rojo_warning = if rojo_path.is_file() {
            fs::read(&rojo_path)
                .map_err(anyhow::Error::from)
                .and_then(|bytes| {
                    serde_json::from_slice::<serde_json::Value>(&bytes).map_err(Into::into)
                })
                .err()
                .map(|error| format!("default.project.json: {error}"))
        } else {
            None
        };
        Self {
            exists: path.is_dir(),
            has_rojo: path.join("default.project.json").is_file(),
            has_record: path.join("rproj.toml").is_file(),
            has_src: path.join("src").is_dir(),
            has_wally: path.join("wally.toml").is_file(),
            path,
            graph,
            warning,
            rojo_warning,
        }
    }

    pub fn availability(&self, action: ProjectAction) -> std::result::Result<(), String> {
        if !self.exists {
            return Err("The selected project directory no longer exists.".into());
        }
        if !self.has_record && !self.has_rojo {
            return Err("No project here: rproj.toml or default.project.json is required.".into());
        }
        if action == ProjectAction::Copy {
            return self
                .has_src
                .then_some(())
                .ok_or_else(|| "Copy Source requires a src directory.".into());
        }
        if action == ProjectAction::Configure {
            return Ok(());
        }
        if matches!(action, ProjectAction::Upgrade | ProjectAction::Watch) && !self.has_rojo {
            return Err(format!("{} requires default.project.json.", action.label()));
        }
        if let Some(warning) = &self.warning {
            return Err(format!(
                "Repair rproj.toml before running this action.\n{warning}"
            ));
        }
        if matches!(action, ProjectAction::Upgrade | ProjectAction::Test) && self.graph.is_none() {
            return Err(format!("{} requires a valid rproj.toml.", action.label()));
        }
        if let Some(graph) = &self.graph {
            if !graph.testing_is_compatible() {
                return Err(
                    "Jest Roblox requires the Wally dependency workflow; repair rproj.toml.".into(),
                );
            }
            if action == ProjectAction::Test && graph.test_runner().is_none() {
                return Err("Testing is not enabled in this project.".into());
            }
            if matches!(action, ProjectAction::Watch | ProjectAction::Test)
                && graph.test_runner() == Some(TestRunner::JestRoblox)
                && !self.has_wally
            {
                return Err("Jest Roblox requires wally.toml; run Upgrade Project.".into());
            }
        }
        Ok(())
    }

    fn summary(&self) -> String {
        let mut text = self.path.display().to_string();
        if !self.exists {
            text.push_str("\n\nProject directory no longer exists.");
        } else if let Some(graph) = &self.graph {
            text.push_str(&format!(
                "\n\nWorkflow: {:?}\nPackages: {}\nCapabilities: {}",
                graph.package_workflow,
                graph.packages.join(", "),
                graph.capability_keys().join(", ")
            ));
        } else if self.has_rojo && !self.has_record {
            text.push_str("\n\nRojo project without rproj.toml.");
        }
        if let Some(warning) = &self.warning {
            text.push_str(&format!("\n\nConfiguration warning\n{warning}"));
        }
        if let Some(warning) = &self.rojo_warning {
            text.push_str(&format!("\n\nRojo warning\n{warning}"));
        }
        text
    }
}

#[derive(Clone, Debug)]
struct ProjectEntry {
    path: PathBuf,
    name: String,
}

#[derive(Default)]
struct Discovery {
    entries: Vec<ProjectEntry>,
    warnings: Vec<String>,
}

fn recognized(path: &Path) -> bool {
    path.join("rproj.toml").is_file() || path.join("default.project.json").is_file()
}

fn linked(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn discover(root: &std::result::Result<PathBuf, String>, launch_dir: &Path) -> Discovery {
    let mut result = Discovery::default();
    let mut paths = BTreeSet::new();
    match root {
        Ok(root) => match fs::read_dir(root) {
            Ok(entries) => {
                let mut skipped_links = 0;
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            result
                                .warnings
                                .push(format!("Could not read a project entry: {error}"));
                            continue;
                        }
                    };
                    match fs::symlink_metadata(entry.path()) {
                        Ok(metadata) if linked(&metadata) => skipped_links += 1,
                        Ok(metadata) if metadata.is_dir() && recognized(&entry.path()) => {
                            add_path(&entry.path(), &mut paths, &mut result);
                        }
                        Ok(_) => {}
                        Err(error) => result
                            .warnings
                            .push(format!("{}: {error}", entry.path().display())),
                    }
                }
                if skipped_links > 0 {
                    result.warnings.push(format!("Skipped {skipped_links} linked entries; only direct project directories are listed."));
                }
            }
            Err(error) => result.warnings.push(format!(
                "Could not read projects folder {}: {error}",
                root.display()
            )),
        },
        Err(error) => result.warnings.push(error.clone()),
    }
    if recognized(launch_dir) {
        add_path(launch_dir, &mut paths, &mut result);
    }
    result.entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });
    result
}

fn add_path(path: &Path, paths: &mut BTreeSet<PathBuf>, result: &mut Discovery) {
    match fs::canonicalize(path) {
        Ok(path) if paths.insert(path.clone()) => {
            let name = path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned();
            result.entries.push(ProjectEntry { path, name });
        }
        Ok(_) => {}
        Err(error) => result.warnings.push(format!("{}: {error}", path.display())),
    }
}

pub enum BrowserOutcome {
    Home,
    Run {
        path: PathBuf,
        action: ProjectAction,
    },
}

pub struct ProjectsApp {
    root: std::result::Result<PathBuf, String>,
    launch_dir: PathBuf,
    discovery: Discovery,
    filter: InputState,
    selected: usize,
    offset: Cell<usize>,
    preview: Option<ProjectContext>,
    active: Option<ProjectContext>,
    action: usize,
    details: bool,
    scroll: Cell<u16>,
    browser_scroll: u16,
    scroll_limit: Cell<u16>,
    help: bool,
    generation: u64,
    worker: Option<Receiver<(u64, Discovery)>>,
    pending: bool,
    preferred: Option<PathBuf>,
    pub status: String,
}

impl ProjectsApp {
    pub fn new(launch_dir: PathBuf) -> Self {
        Self {
            root: Err("Projects folder has not been loaded.".into()),
            launch_dir,
            discovery: Discovery::default(),
            filter: InputState::new(""),
            selected: 0,
            offset: Cell::new(0),
            preview: None,
            active: None,
            action: 0,
            details: false,
            scroll: Cell::new(0),
            browser_scroll: 0,
            scroll_limit: Cell::new(0),
            help: false,
            generation: 0,
            worker: None,
            pending: false,
            preferred: None,
            status: String::new(),
        }
    }

    pub fn open(&mut self, root: std::result::Result<PathBuf, String>) {
        self.root = root;
        self.active = None;
        self.details = false;
        self.help = false;
        self.refresh();
    }

    fn refresh(&mut self) {
        self.preferred = self
            .preferred
            .take()
            .or_else(|| self.selected_path().cloned());
        self.generation += 1;
        self.pending = true;
        self.start_scan();
        self.refresh_active();
    }

    fn start_scan(&mut self) {
        if self.worker.is_some() || !self.pending {
            return;
        }
        self.pending = false;
        let root = self.root.clone();
        let launch_dir = self.launch_dir.clone();
        let generation = self.generation;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send((generation, discover(&root, &launch_dir)));
        });
        self.worker = Some(receiver);
    }

    pub fn loading(&self) -> bool {
        self.worker.is_some()
    }

    pub fn poll(&mut self) {
        match self.worker.as_ref().map(Receiver::try_recv) {
            Some(Ok((generation, result))) => {
                self.worker = None;
                if generation == self.generation {
                    self.discovery = result;
                    let wanted = self.preferred.take();
                    self.selected = wanted
                        .and_then(|path| self.visible().iter().position(|entry| entry.path == path))
                        .unwrap_or(0);
                    self.update_preview();
                    crate::diagnostics::event(
                        "projects.scan",
                        format!(
                            "{} projects; {} warnings",
                            self.discovery.entries.len(),
                            self.discovery.warnings.len()
                        ),
                    );
                }
                self.start_scan();
            }
            Some(Err(TryRecvError::Disconnected)) => {
                self.worker = None;
                self.status = "Project scanning stopped unexpectedly. Press F5 to retry.".into();
                self.start_scan();
            }
            Some(Err(TryRecvError::Empty)) | None => {}
        }
    }

    fn visible(&self) -> Vec<&ProjectEntry> {
        let query = self.filter.text().to_lowercase();
        self.discovery
            .entries
            .iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query)
                    || entry.path.to_string_lossy().to_lowercase().contains(&query)
            })
            .collect()
    }

    fn selected_path(&self) -> Option<&PathBuf> {
        self.visible().get(self.selected).map(|entry| &entry.path)
    }

    fn update_preview(&mut self) {
        self.preview = self.selected_path().cloned().map(ProjectContext::load);
        self.scroll.set(0);
    }

    pub fn refresh_active(&mut self) {
        if let Some(project) = self.active.take() {
            self.active = Some(ProjectContext::load(project.path));
        }
    }

    pub fn show_created(&mut self, path: PathBuf, root: std::result::Result<PathBuf, String>) {
        let path = fs::canonicalize(&path).unwrap_or(path);
        self.root = root;
        self.filter = InputState::new("");
        self.preferred = Some(path.clone());
        self.active = Some(ProjectContext::load(path));
        self.action = 0;
        self.details = false;
        self.offset.set(0);
        self.scroll.set(0);
        self.refresh();
    }

    pub fn paste(&mut self, text: &str) {
        if self.active.is_none() && !self.details && !self.help {
            for ch in text.chars().filter(|ch| !ch.is_control()) {
                self.filter
                    .handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
            }
            self.selected = 0;
            self.offset.set(0);
            self.update_preview();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<BrowserOutcome> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(BrowserOutcome::Home);
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                self.help = false;
            }
            return None;
        }
        match key.code {
            KeyCode::Char('?') => self.help = true,
            KeyCode::Esc if self.details => {
                self.details = false;
                self.scroll.set(0);
            }
            KeyCode::Esc if self.active.is_some() => {
                self.active = None;
                self.update_preview();
                self.scroll.set(self.browser_scroll);
                crate::diagnostics::event("screen", "Projects");
            }
            KeyCode::Esc => return Some(BrowserOutcome::Home),
            KeyCode::Tab | KeyCode::BackTab => {
                self.details = !self.details;
                self.scroll.set(0);
            }
            KeyCode::F(5) => self.refresh(),
            code if self.details => {
                self.scroll.set(
                    match code {
                        KeyCode::Up => self.scroll.get().saturating_sub(1),
                        KeyCode::Down => self.scroll.get().saturating_add(1),
                        KeyCode::PageUp => self.scroll.get().saturating_sub(10),
                        KeyCode::PageDown => self.scroll.get().saturating_add(10),
                        KeyCode::Home => 0,
                        KeyCode::End => self.scroll_limit.get(),
                        _ => self.scroll.get(),
                    }
                    .min(self.scroll_limit.get()),
                );
            }
            KeyCode::Enter => {
                if let Some(project) = &self.active {
                    let action = ProjectAction::ALL[self.action];
                    if let Err(reason) = project.availability(action) {
                        self.status = reason;
                    } else {
                        return Some(BrowserOutcome::Run {
                            path: project.path.clone(),
                            action,
                        });
                    }
                } else if let Some(path) = self.selected_path().cloned() {
                    self.browser_scroll = self.scroll.get();
                    crate::diagnostics::event("project.open", path.display().to_string());
                    self.active = Some(ProjectContext::load(path));
                    self.action = 0;
                    self.scroll.set(0);
                    self.status.clear();
                }
            }
            code @ (KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End) => {
                let max = if self.active.is_some() {
                    ProjectAction::ALL.len()
                } else {
                    self.visible().len()
                }
                .saturating_sub(1);
                let selected = if self.active.is_some() {
                    &mut self.action
                } else {
                    &mut self.selected
                };
                *selected = match code {
                    KeyCode::Up => selected.saturating_sub(1),
                    KeyCode::Down => selected.saturating_add(1),
                    KeyCode::PageUp => selected.saturating_sub(10),
                    KeyCode::PageDown => selected.saturating_add(10),
                    KeyCode::Home => 0,
                    KeyCode::End => max,
                    _ => *selected,
                }
                .min(max);
                self.update_preview();
                self.status.clear();
            }
            _ if self.active.is_none() => {
                self.filter.handle_key(key);
                self.selected = 0;
                self.offset.set(0);
                self.update_preview();
            }
            _ => {}
        }
        None
    }

    pub fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        if tui::is_too_small(area) {
            frame.render_widget(
                Paragraph::new(
                    "Projects\n\nResize to at least 60 x 16.\n? help   Esc back   Ctrl+C Home",
                )
                .wrap(Wrap { trim: false }),
                area,
            );
        } else {
            let outer = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(3),
            ])
            .split(area);
            let title = if let Some(project) = &self.active {
                format!(
                    "Project: {}",
                    project
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            } else {
                "Projects".into()
            };
            frame.render_widget(
                Paragraph::new(title)
                    .style(tui::title_style())
                    .block(Block::default().borders(Borders::BOTTOM)),
                outer[0],
            );
            let panes = tui::responsive_panes(outer[1], 40);
            let (items, selected): (Vec<ListItem<'_>>, usize) = if let Some(project) = &self.active
            {
                (
                    ProjectAction::ALL
                        .iter()
                        .map(|action| {
                            let disabled = project.availability(*action).is_err();
                            ListItem::new(format!(
                                "{}{}",
                                action.label(),
                                if disabled { "  [unavailable]" } else { "" }
                            ))
                            .style(Style::default().fg(if disabled {
                                Color::DarkGray
                            } else {
                                Color::White
                            }))
                        })
                        .collect(),
                    self.action,
                )
            } else {
                (
                    self.visible()
                        .iter()
                        .map(|entry| ListItem::new(entry.name.clone()))
                        .collect(),
                    self.selected,
                )
            };
            let mut state = ListState::default().with_selected(Some(selected));
            if self.active.is_none() {
                *state.offset_mut() = self.offset.get();
            }
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .title(if self.active.is_some() {
                                " Project actions "
                            } else {
                                " Projects "
                            })
                            .borders(Borders::ALL),
                    )
                    .highlight_style(selected_style(!self.details)),
                panes[0],
                &mut state,
            );
            if self.active.is_none() {
                self.offset.set(state.offset());
            }
            let mut detail = self
                .active
                .as_ref()
                .or(self.preview.as_ref())
                .map(ProjectContext::summary)
                .unwrap_or_else(|| {
                    if self.loading() {
                        "Loading projects...".into()
                    } else {
                        "No matching projects.".into()
                    }
                });
            if let Some(project) = &self.active {
                if let Err(reason) = project.availability(ProjectAction::ALL[self.action]) {
                    detail.push_str(&format!("\n\nUnavailable\n{reason}"));
                }
            } else {
                if let Ok(root) = &self.root {
                    detail.push_str(&format!("\n\nProjects folder\n{}", root.display()));
                }
                if !self.discovery.warnings.is_empty() {
                    detail.push_str(&format!(
                        "\n\nWarnings\n{}",
                        self.discovery.warnings.join("\n")
                    ));
                }
            }
            let lines = wrap_lines(&detail, panes[1].width.saturating_sub(2) as usize);
            let limit = lines
                .len()
                .saturating_sub(panes[1].height.saturating_sub(2) as usize)
                .min(u16::MAX as usize) as u16;
            self.scroll_limit.set(limit);
            self.scroll.set(self.scroll.get().min(limit));
            frame.render_widget(
                Paragraph::new(lines.join("\n"))
                    .scroll((self.scroll.get(), 0))
                    .block(
                        Block::default()
                            .title(" Details ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(if self.details {
                                ACCENT
                            } else {
                                Color::Gray
                            })),
                    ),
                panes[1],
            );
            let status = if !self.status.is_empty() {
                self.status.clone()
            } else if self.loading() {
                "Refreshing projects...".into()
            } else if self.active.is_none() {
                format!("Filter: {}", self.filter.text())
            } else {
                String::new()
            };
            tui::render_footer(
                frame,
                outer[2],
                &status,
                "Arrows navigate  Enter open  Tab details  F5 refresh  Esc back  ? help",
                false,
            );
        }
        if self.help {
            let popup = tui::centered(area, 70, 13);
            frame.render_widget(Clear, popup);
            frame.render_widget(Paragraph::new("Projects\nType to filter by name or path. Enter opens project actions.\nTab switches to scrollable details; arrows, Page Up/Down, Home/End navigate.\nF5 refreshes. Esc backs out; Ctrl+C returns Home.\nOnly direct project directories and the current project are included. No files are changed by browsing.").wrap(Wrap { trim: false }).block(Block::default().title(" Help ").borders(Borders::ALL)), popup);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use tempfile::TempDir;

    fn project(root: &Path, name: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("default.project.json"), "{}").unwrap();
        path
    }

    fn ready(root: &Path, launch: &Path) -> ProjectsApp {
        let mut app = ProjectsApp::new(launch.to_owned());
        app.root = Ok(root.to_owned());
        app.discovery = discover(&app.root, launch);
        app.update_preview();
        app
    }

    fn key(app: &mut ProjectsApp, code: KeyCode) -> Option<BrowserOutcome> {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn discovery_is_shallow_sorted_and_deduplicated() {
        let root = TempDir::new().unwrap();
        let z = project(root.path(), "Zulu");
        project(root.path(), "alpha");
        project(root.path(), "caf\u{e9}");
        project(&root.path().join("container"), "nested");
        fs::write(z.join("rproj.toml"), "broken = [").unwrap();
        let result = discover(&Ok(root.path().to_owned()), &z);
        assert_eq!(
            result
                .entries
                .iter()
                .map(|p| p.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "caf\u{e9}", "Zulu"]
        );
        assert!(ProjectContext::load(z).warning.is_some());
        assert!(
            discover(&Ok(root.path().to_owned()), root.path())
                .entries
                .iter()
                .all(|p| p.name != "nested")
        );
    }

    #[test]
    fn missing_root_keeps_launch_project_without_creating_anything() {
        let temp = TempDir::new().unwrap();
        let launch = project(temp.path(), "outside");
        let missing = temp.path().join("missing");
        let result = discover(&Ok(missing.clone()), &launch);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.warnings.len(), 1);
        assert!(!missing.exists());
        let result = discover(&Err("Configuration unreadable".into()), &launch);
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.warnings, ["Configuration unreadable"]);
        let result = discover(&Ok(launch.join("default.project.json")), &launch);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn discovery_skips_junction_children() {
        let root = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let target = project(outside.path(), "Target");
        let link = root.path().join("Linked");
        let output = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = discover(&Ok(root.path().to_owned()), root.path());
        assert!(result.entries.is_empty());
        assert!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("linked"))
        );
        fs::remove_dir(&link).unwrap();
        assert!(target.join("default.project.json").exists());
    }

    #[test]
    fn malformed_rojo_document_stays_visible_with_a_warning() {
        let root = TempDir::new().unwrap();
        let path = project(root.path(), "Broken");
        fs::write(path.join("default.project.json"), "{").unwrap();
        let app = ready(root.path(), root.path());
        assert_eq!(app.discovery.entries.len(), 1);
        assert!(app.preview.unwrap().summary().contains("Rojo warning"));
    }

    #[test]
    fn selected_actions_carry_the_target_not_the_launch_directory() {
        let root = TempDir::new().unwrap();
        let a = project(root.path(), "A");
        let b = project(root.path(), "B");
        fs::create_dir(a.join("src")).unwrap();
        let mut app = ready(root.path(), &b);
        key(&mut app, KeyCode::Enter);
        app.action = 4;
        let Some(BrowserOutcome::Run { path, action }) = key(&mut app, KeyCode::Enter) else {
            panic!("expected project action")
        };
        assert_eq!(path, fs::canonicalize(&a).unwrap());
        assert_eq!(action, ProjectAction::Copy);
        fs::remove_dir_all(&a).unwrap();
        let error = action.run(&path).unwrap_err().to_string();
        assert!(error.contains("no longer exists"), "{error}");
        assert!(!a.exists());
        assert!(b.exists());
    }

    #[test]
    fn selected_path_process_driver() {
        let Some(path) = std::env::var_os("RPROJ_TEST_SELECTED_PATH") else {
            return;
        };
        let path = PathBuf::from(path);
        ProjectAction::Watch.run(&path).unwrap();
        super::super::upgrade::run_in(&path, true).unwrap();
    }

    #[test]
    #[cfg(windows)]
    fn commands_read_write_and_spawn_in_a_when_launched_from_b() {
        let root = TempDir::new().unwrap();
        let a = project(root.path(), "A");
        let b = project(root.path(), "B");
        project_file::save_to(&ProjectGraph::default(), &a).unwrap();
        fs::write(b.join("rproj.toml"), "invalid = [").unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let output = std::process::Command::new("rustc")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/foreground_tool.rs"
            ))
            .arg("-o")
            .arg(bin.join("rojo.exe"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = std::env::join_paths(
            std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
        )
        .unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "commands::projects::tests::selected_path_process_driver",
                "--nocapture",
            ])
            .current_dir(&b)
            .env("RPROJ_TEST_SELECTED_PATH", &a)
            .env("PATH", path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            fs::read_to_string(a.join("tools.log")).unwrap().trim(),
            "rojo"
        );
        assert!(!b.join("tools.log").exists());
        assert!(a.join(".gitignore").exists());
        assert!(!b.join(".gitignore").exists());
        assert_eq!(
            fs::read_to_string(b.join("rproj.toml")).unwrap(),
            "invalid = ["
        );
    }

    #[test]
    fn back_preserves_filter_selection_and_list_scroll() {
        let root = TempDir::new().unwrap();
        for n in 0..30 {
            project(root.path(), &format!("Project{n:02}"));
        }
        let mut app = ready(root.path(), root.path());
        app.paste("Project");
        key(&mut app, KeyCode::End);
        app.offset.set(20);
        app.scroll.set(3);
        key(&mut app, KeyCode::Enter);
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.selected, 29);
        assert_eq!(app.offset.get(), 20);
        assert_eq!(app.scroll.get(), 3);
        assert_eq!(app.filter.text(), "Project");
        assert!(app.active.is_none());
    }

    #[test]
    fn stale_scan_results_are_discarded_and_refresh_is_serialized() {
        let root = TempDir::new().unwrap();
        project(root.path(), "A");
        let mut app = ready(root.path(), root.path());
        let (sender, receiver) = mpsc::channel();
        app.worker = Some(receiver);
        app.refresh();
        app.refresh();
        assert!(app.pending);
        sender.send((0, Discovery::default())).unwrap();
        app.poll();
        assert_eq!(app.discovery.entries.len(), 1);
        assert!(app.loading());
        assert!(!app.pending);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while app.loading() {
            assert!(std::time::Instant::now() < deadline);
            app.poll();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(app.discovery.entries.len(), 1);
    }

    #[test]
    fn new_project_handoff_selects_it_on_back() {
        let root = TempDir::new().unwrap();
        let path = project(root.path(), "Created");
        let mut app = ready(root.path(), root.path());
        app.paste("does not match");
        app.show_created(path.clone(), Ok(root.path().to_owned()));
        assert_eq!(
            app.active.as_ref().unwrap().path,
            fs::canonicalize(&path).unwrap()
        );
        assert!(app.filter.text().is_empty());
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.selected_path(), Some(&fs::canonicalize(path).unwrap()));
    }

    #[test]
    fn project_renders_wide_narrow_minimum_help_and_disabled_reasons() {
        let root = TempDir::new().unwrap();
        project(root.path(), "Example");
        let mut app = ready(root.path(), root.path());
        for (width, height) in [(120, 30), (280, 70), (80, 24), (40, 10)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| app.render(frame)).unwrap();
            let output: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(output.contains("Projects"));
            app.help = true;
            terminal.draw(|frame| app.render(frame)).unwrap();
            app.help = false;
        }
        key(&mut app, KeyCode::Enter);
        app.action = 1;
        assert!(key(&mut app, KeyCode::Enter).is_none());
        assert!(app.status.contains("rproj.toml"));
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        let output: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(output.contains("[unavailable]"));
        assert!(output.contains("rproj.toml"));
    }
}
