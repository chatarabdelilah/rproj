mod model;
mod worker;

use std::collections::{BTreeSet, VecDeque};
use std::io::{self, IsTerminal};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::event::{Event as TerminalEvent, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::config::GlobalConfig;
use crate::steps::execution::MessageKind;
use crate::tui::{
    ACCENT, ConfirmState, ERROR, MUTED, PickerItem, PickerState, TerminalSession,
    render_confirm_default_no,
};
use model::{Category, Selection};
use worker::{Event, Outcome, Status, Worker};

#[derive(Clone, Copy)]
enum Confirmation {
    Apply,
    Stop,
}

struct CategoryDraft {
    category: Category,
    picker: PickerState<String>,
    checked: BTreeSet<String>,
}

struct App {
    selection: Selection,
    selected: usize,
    draft: Option<CategoryDraft>,
    confirmation: Option<Confirmation>,
    help: bool,
    too_small: bool,
    details_focus: bool,
    scroll: u16,
    max_scroll: u16,
    follow_output: bool,
    logs: VecDeque<String>,
    truncated: bool,
    statuses: Vec<Status>,
    started: Option<Instant>,
    elapsed: Option<Duration>,
    running: bool,
    stopping: bool,
    outcome: Option<Outcome>,
    error: Option<String>,
}

impl App {
    fn new(selection: Selection) -> Self {
        Self {
            selection,
            selected: 0,
            draft: None,
            confirmation: None,
            help: false,
            too_small: false,
            details_focus: false,
            scroll: 0,
            max_scroll: 0,
            follow_output: true,
            logs: VecDeque::new(),
            truncated: false,
            statuses: vec![],
            started: None,
            elapsed: None,
            running: false,
            stopping: false,
            outcome: None,
            error: None,
        }
    }

    fn message(&mut self, text: &str) {
        for line in text.lines() {
            if line.chars().count() > 2048 {
                self.truncated = true;
            }
            if self.logs.len() == 500 {
                self.logs.pop_front();
                self.truncated = true;
            }
            self.logs.push_back(
                line.chars()
                    .filter(|character| !character.is_control() || *character == '\t')
                    .take(2048)
                    .collect(),
            );
        }
    }

    fn edit(&mut self, category: Category) {
        self.draft = Some(CategoryDraft {
            category,
            checked: self.selection.keys(category).clone(),
            picker: PickerState::new(
                category
                    .entries()
                    .into_iter()
                    .map(|entry| PickerItem {
                        value: entry.key.into(),
                        label: entry.key.into(),
                        detail: entry.description.into(),
                    })
                    .collect(),
            ),
        });
        self.scroll = 0;
    }

    fn title(&self) -> &str {
        if self.stopping && self.running {
            "Stopping after current item"
        } else if self.running {
            "Installing"
        } else {
            match self.outcome {
                Some(Outcome::Completed) => "Completed",
                Some(Outcome::Warnings) => "Completed with warnings / manual steps",
                Some(Outcome::Cancelled) => "Cancelled - completed installations remain",
                Some(Outcome::Failed) => "Setup failed",
                None => "Machine Setup",
            }
        }
    }

    fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        self.too_small = area.width < 60 || area.height < 16;
        if self.too_small {
            frame.render_widget(
                Paragraph::new(format!(
                    "{}\nResize to at least 60 x 16\n? Help   Esc Back / Stop",
                    self.title()
                )),
                area,
            );
            if self.help {
                frame.render_widget(Paragraph::new("Setup: review choices before Apply.\nEsc/Ctrl+C stops after the current item.\n? closes Help."), area);
            }
            if self.confirmation.is_some() {
                self.render_confirmation(frame);
            }
            return;
        }
        let bands = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);
        let elapsed = self
            .started
            .map(|time| {
                format!(
                    "  {}s",
                    self.elapsed.unwrap_or_else(|| time.elapsed()).as_secs()
                )
            })
            .unwrap_or_default();
        frame.render_widget(
            Paragraph::new(format!(
                "{}{}\n{}",
                self.title(),
                elapsed,
                self.selection.root.display()
            ))
            .style(Style::default().fg(ACCENT)),
            bands[0],
        );
        let panes = if area.width >= 100 {
            Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
                .split(bands[1])
        } else {
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(bands[1])
        };
        let rows: Vec<String> = if let Some(draft) = &self.draft {
            draft
                .picker
                .filtered()
                .iter()
                .map(|item| {
                    format!(
                        "[{}] {}",
                        if draft.checked.contains(&item.value) {
                            "x"
                        } else {
                            " "
                        },
                        item.label
                    )
                })
                .collect()
        } else if self.running || self.outcome.is_some() {
            worker::plan(&self.selection)
                .iter()
                .zip(&self.statuses)
                .map(|(item, status)| format!("{status:?}  {}", item.label()))
                .collect()
        } else {
            Category::ALL
                .iter()
                .map(|category| {
                    let count = category
                        .entries()
                        .iter()
                        .filter(|entry| self.selection.keys(*category).contains(entry.key))
                        .count();
                    format!(
                        "{}  ({count}){}",
                        category.label(),
                        if self.selection.active(*category) {
                            ""
                        } else {
                            " - inactive"
                        }
                    )
                })
                .chain(["Apply Setup".into(), "Back".into()])
                .collect()
        };
        let index = self
            .draft
            .as_ref()
            .map(|draft| draft.picker.selected)
            .unwrap_or(self.selected);
        let mut list_state = ListState::default().with_selected(Some(index));
        let list_title = self
            .draft
            .as_ref()
            .map(|draft| {
                format!(
                    "{} | Filter: {}",
                    draft.category.label(),
                    draft.picker.query.text()
                )
            })
            .unwrap_or_else(|| "Selections / Tasks".into());
        frame.render_stateful_widget(
            List::new(rows.into_iter().map(ListItem::new).collect::<Vec<_>>())
                .block(Block::default().borders(Borders::ALL).title(list_title))
                .highlight_style(Style::default().fg(ACCENT)),
            panes[0],
            &mut list_state,
        );
        let details = if self.running || self.outcome.is_some() {
            format!(
                "{}{}",
                if self.truncated {
                    "[Earlier output truncated]\n"
                } else {
                    ""
                },
                self.logs.iter().cloned().collect::<Vec<_>>().join("\n")
            )
        } else if let Some(draft) = &self.draft {
            draft
                .picker
                .selected_value()
                .and_then(|key| crate::catalog::tool_catalog::find(key))
                .map(|entry| {
                    format!(
                        "{}\n\n{}\n\n{}",
                        entry.key, entry.description, entry.docs_url
                    )
                })
                .unwrap_or_else(|| "No matching entries".into())
        } else {
            let mut text = "Selections describe what to prepare, not what is installed.\nDeselecting never uninstalls software.\nRokit is foundational and checked on Apply.\n\n".to_string();
            text.push_str(&format!(
                "Projects folder: {}\n\n",
                self.selection.root.display()
            ));
            if let Some(category) = Category::ALL.get(self.selected) {
                text.push_str(category.label());
                text.push('\n');
                for key in self.selection.keys(*category) {
                    match crate::catalog::tool_catalog::find(key) {
                        Some(entry) if category.contains(entry) => {
                            text.push_str(&format!("\n{}: {}", entry.key, entry.description));
                        }
                        None => {
                            text.push_str(&format!("\n{key} [unknown, preserved; not executed]"))
                        }
                        Some(entry)
                            if !Category::ALL.iter().any(|other| {
                                other.shares_storage(*category) && other.contains(entry)
                            }) =>
                        {
                            text.push_str(&format!(
                                "\n{key} [unsupported in this category; preserved, not executed]"
                            ))
                        }
                        _ => {}
                    }
                }
                if !self.selection.active(*category) {
                    text.push_str("\n\nInactive until the parent application is selected. Choices remain remembered.");
                }
            } else if self.selected == 6 {
                text.push_str("Planned operations:\n");
                for item in worker::plan(&self.selection) {
                    text.push_str(&format!("\n{}", item.label()));
                }
                text.push_str("\n\nKnown inactive and unsupported choices are preserved but not executed. Marketplace plugins and account linking may require manual steps.");
            }
            text
        };
        let wrapped = crate::tui::wrap_lines(&details, panes[1].width.saturating_sub(2) as usize);
        self.max_scroll = wrapped
            .len()
            .saturating_sub(panes[1].height.saturating_sub(2) as usize)
            .min(u16::MAX as usize) as u16;
        self.scroll = if (self.running || self.outcome.is_some()) && self.follow_output {
            self.max_scroll
        } else {
            self.scroll.min(self.max_scroll)
        };
        frame.render_widget(
            Paragraph::new(wrapped.join("\n"))
                .scroll((self.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.details_focus {
                            "Details / Output [focused]"
                        } else {
                            "Details / Output"
                        }),
                ),
            panes[1],
        );
        let keys = if self.running {
            "Esc/Ctrl+C Stop   Tab focus   PgUp/PgDn scroll   ? Help"
        } else if self.outcome.is_some() {
            "Enter Done   Esc Review   Tab focus   PgUp/PgDn scroll   ? Help"
        } else if self.draft.is_some() {
            "Space toggle   Enter accept   Esc cancel   Tab focus   ? Help"
        } else {
            "Arrows select   Enter edit/apply   Esc Back   Tab focus   ? Help"
        };
        frame.render_widget(
            Paragraph::new(format!("{}\n{keys}", self.error.as_deref().unwrap_or("")))
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(if self.error.is_some() { ERROR } else { MUTED })),
            bands[2],
        );
        if self.help {
            frame.render_widget(ratatui::widgets::Clear, bands[1]);
            frame.render_widget(Paragraph::new("Machine Setup\n\nReview categories, then Apply Setup and confirm. Space toggles a choice; Enter accepts a category, Esc abandons its edits. Typing filters without changing checks.\n\nTab switches list/details focus. Arrows and Page Up/Down scroll details. Home/End jump.\n\nDuring installation, stopping waits for the active item; already installed software remains. Installer and UAC dialogs may appear separately. Results include failures and manual steps. Rerun from Review to retry.\n\n? or Esc closes Help.").wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title(" Help ")), bands[1]);
        }
        if self.confirmation.is_some() {
            self.render_confirmation(frame);
        }
    }

    fn render_confirmation(&self, frame: &mut Frame<'_>) {
        let text = match self.confirmation.unwrap() {
            Confirmation::Apply => format!("Apply these selections? Rokit is required. Projects: {}. Some plugins require manual steps. Nothing is uninstalled.", self.selection.root.display()),
            Confirmation::Stop => "Stop after the current item? Completed installations remain; machine configuration will not be saved.".into(),
        };
        render_confirm_default_no(frame, frame.area(), &ConfirmState::new(text));
    }
}

pub fn run(config: &mut GlobalConfig) -> Result<()> {
    require_terminal()?;
    let mut terminal = TerminalSession::enter()?;
    if run_in(&mut terminal, config)? {
        Ok(())
    } else {
        Err(crate::interrupt::Cancelled.into())
    }
}

pub fn open() -> Result<()> {
    require_terminal()?;
    let mut terminal = TerminalSession::enter()?;
    if open_in(&mut terminal)? {
        Ok(())
    } else {
        Err(crate::interrupt::Cancelled.into())
    }
}

fn require_terminal() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!(
            "Machine Setup requires an interactive terminal; run `rproj setup` in a terminal"
        );
    }
    Ok(())
}

pub fn open_in(terminal: &mut TerminalSession) -> Result<bool> {
    match GlobalConfig::load() {
        Ok(mut config) => run_in(terminal, &mut config),
        Err(error) => loop {
            terminal.draw(|frame| frame.render_widget(Paragraph::new(format!("Machine configuration could not be loaded:\n{error:#}\n\nNo configuration was changed. Repair the file, then retry.\nEsc/Ctrl+C Back")).wrap(Wrap { trim: false }), frame.area()))?;
            if let TerminalEvent::Key(key) = terminal.read_event()?
                && (key.code == KeyCode::Esc || ctrl_c(key))
            {
                return Ok(false);
            }
        },
    }
}

fn ctrl_c(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

pub fn run_in(terminal: &mut TerminalSession, config: &mut GlobalConfig) -> Result<bool> {
    let result = run_with(terminal, config, Worker::start, GlobalConfig::save);
    if matches!(result, Ok(true)) {
        crate::steps::notify::summary(
            "rproj setup",
            "Setup finished. Review any warnings and manual steps.",
        );
    }
    result
}

fn run_with(
    terminal: &mut TerminalSession,
    config: &mut GlobalConfig,
    start: impl Fn(Selection) -> Worker,
    save: impl Fn(&GlobalConfig) -> Result<()>,
) -> Result<bool> {
    let mut app = App::new(Selection::load(config)?);
    let mut worker: Option<Worker> = None;
    let mut pending_outcome = None;
    let mut last_outcome = None;
    crate::diagnostics::event("setup.open", "review");
    loop {
        if let Some(active) = &worker {
            while let Ok(event) = active.receiver.as_ref().unwrap().try_recv() {
                match event {
                    Event::Started(index) => {
                        app.statuses[index] = Status::Running;
                        app.selected = index;
                        crate::diagnostics::event("setup.item", index.to_string());
                    }
                    Event::Message(kind, text) => {
                        app.message(&text);
                        if kind == MessageKind::Warning {
                            app.error = Some("See output for details".into());
                        }
                    }
                    Event::Finished(index, status) => app.statuses[index] = status,
                    Event::Done(outcome) => {
                        app.elapsed = app.started.map(|started| started.elapsed());
                        pending_outcome = Some(outcome);
                    }
                }
            }
            if active.dropped.swap(0, Ordering::Relaxed) > 0 {
                app.truncated = true;
            }

            if crate::interrupt::requested() {
                active.request_stop();
                crate::interrupt::reset();
                app.stopping = true;
            }
            if pending_outcome.is_some() && app.confirmation.is_none() {
                let mut outcome = pending_outcome.take().unwrap();
                if active.stop.load(Ordering::SeqCst) {
                    outcome = Outcome::Cancelled;
                }
                app.running = false;
                if matches!(outcome, Outcome::Completed | Outcome::Warnings)
                    && let Err(error) = persist(&app.selection, config, &save)
                {
                    app.message(&format!("Configuration could not be saved: {error:#}"));
                    outcome = Outcome::Failed;
                }
                for status in &mut app.statuses {
                    if *status == Status::Pending {
                        *status = Status::Skipped;
                    }
                }
                if matches!(outcome, Outcome::Failed | Outcome::Warnings) {
                    app.message(&crate::diagnostics::path_message());
                }
                app.outcome = Some(outcome);
                last_outcome = Some(outcome);
                crate::diagnostics::event("setup.result", format!("{outcome:?}"));
                worker.take();
            }
        }
        terminal.draw(|frame| app.render(frame))?;
        let Some(event) = terminal.poll_event(Duration::from_millis(100))? else {
            continue;
        };
        let key = match event {
            TerminalEvent::Key(key) if key.kind != crossterm::event::KeyEventKind::Release => key,
            TerminalEvent::Paste(text) => {
                if app.too_small {
                    continue;
                }
                if let Some(draft) = &mut app.draft {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        draft.picker.handle_key(KeyEvent::new(
                            KeyCode::Char(character),
                            KeyModifiers::NONE,
                        ));
                    }
                }
                continue;
            }
            _ => continue,
        };
        if app.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) || ctrl_c(key) {
                app.help = false;
            }
            continue;
        }
        if let Some(confirmation) = app.confirmation {
            if app.too_small
                && matches!(confirmation, Confirmation::Apply)
                && !matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N')
                )
                && !ctrl_c(key)
            {
                continue;
            }
            if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
                match confirmation {
                    Confirmation::Apply => {
                        app.statuses = vec![Status::Pending; worker::plan(&app.selection).len()];
                        app.logs.clear();
                        app.error = None;
                        app.truncated = false;
                        app.scroll = 0;
                        app.follow_output = true;
                        app.started = Some(Instant::now());
                        app.elapsed = None;
                        app.running = true;
                        app.stopping = false;
                        worker = Some(start(app.selection.clone()));
                        crate::diagnostics::event("setup.apply", "confirmed");
                    }
                    Confirmation::Stop => {
                        if let Some(worker) = &worker {
                            worker.request_stop();
                            app.stopping = true;
                        }
                    }
                }
                app.confirmation = None;
            } else if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('n') | KeyCode::Char('N')
            ) || ctrl_c(key)
            {
                app.confirmation = None;
            }
            continue;
        }
        if key.code == KeyCode::Char('?') {
            app.help = true;
            continue;
        }
        if app.too_small && key.code != KeyCode::Esc && !ctrl_c(key) {
            continue;
        }
        if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
            app.details_focus = !app.details_focus;
            continue;
        }
        if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            || app.details_focus
                && matches!(
                    key.code,
                    KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
                )
        {
            match key.code {
                KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(10),
                KeyCode::PageDown => app.scroll = app.scroll.saturating_add(10),
                KeyCode::Up => app.scroll = app.scroll.saturating_sub(1),
                KeyCode::Down => app.scroll = app.scroll.saturating_add(1),
                KeyCode::Home => app.scroll = 0,
                KeyCode::End => app.scroll = app.max_scroll,
                _ => {}
            }
            app.follow_output = key.code == KeyCode::End;
            continue;
        }
        if app.running {
            if !app.details_focus {
                match key.code {
                    KeyCode::Up => app.selected = app.selected.saturating_sub(1),
                    KeyCode::Down => {
                        app.selected = (app.selected + 1).min(app.statuses.len().saturating_sub(1))
                    }
                    KeyCode::Home => app.selected = 0,
                    KeyCode::End => app.selected = app.statuses.len().saturating_sub(1),
                    _ => {}
                }
            }
            if (key.code == KeyCode::Esc || ctrl_c(key)) && !app.stopping {
                app.confirmation = Some(Confirmation::Stop);
            }
            continue;
        }
        if let Some(outcome) = app.outcome {
            match key.code {
                KeyCode::Up => app.selected = app.selected.saturating_sub(1),
                KeyCode::Down => {
                    app.selected = (app.selected + 1).min(app.statuses.len().saturating_sub(1))
                }
                KeyCode::Home => app.selected = 0,
                KeyCode::End => app.selected = app.statuses.len().saturating_sub(1),
                KeyCode::Enter => return exit_result(Some(outcome)),
                KeyCode::Esc => {
                    app.outcome = None;
                    app.selected = 0;
                    app.scroll = 0;
                    app.error = None;
                    app.started = None;
                }
                _ if ctrl_c(key) => return exit_result(last_outcome),
                _ => {}
            }
            continue;
        }
        if ctrl_c(key) {
            return exit_result(last_outcome);
        }
        if let Some(draft) = &mut app.draft {
            match key.code {
                KeyCode::Esc => app.draft = None,
                KeyCode::Enter => {
                    app.selection
                        .replace_category(draft.category, &draft.checked);
                    app.draft = None;
                }
                KeyCode::Char(' ') => {
                    if let Some(key) = draft.picker.selected_value().cloned()
                        && !draft.checked.remove(&key)
                    {
                        draft.checked.insert(key);
                    }
                }
                _ => {
                    draft.picker.handle_key(key);
                }
            }
            continue;
        }
        match key.code {
            KeyCode::Esc => return exit_result(last_outcome),
            KeyCode::Up => app.selected = app.selected.saturating_sub(1),
            KeyCode::Down => app.selected = (app.selected + 1).min(7),
            KeyCode::Home => app.selected = 0,
            KeyCode::End => app.selected = 7,
            KeyCode::Enter => match app.selected {
                0..=5 => app.edit(Category::ALL[app.selected]),
                6 => app.confirmation = Some(Confirmation::Apply),
                _ => return exit_result(last_outcome),
            },
            _ => {}
        }
    }
}

fn exit_result(outcome: Option<Outcome>) -> Result<bool> {
    match outcome {
        Some(Outcome::Completed | Outcome::Warnings) => Ok(true),
        Some(Outcome::Failed) => anyhow::bail!(
            "Machine Setup failed; see {}",
            crate::diagnostics::path_message()
        ),
        Some(Outcome::Cancelled) | None => Ok(false),
    }
}

fn persist(
    selection: &Selection,
    config: &mut GlobalConfig,
    save: impl Fn(&GlobalConfig) -> Result<()>,
) -> Result<()> {
    let checked_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let updated = selection.to_config(checked_at);
    save(&updated)?;
    *config = updated;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_save_retains_recorded_configuration_and_draft() {
        let mut config = GlobalConfig {
            roblox_projects_root: Some("fixture-projects".into()),
            last_checked: Some("old".into()),
            ..Default::default()
        };
        let mut selection = Selection::load(&config).unwrap();
        selection.apps.insert("studio".into());
        let before = toml::to_string(&config).unwrap();
        assert!(
            persist(&selection, &mut config, |_| anyhow::bail!(
                "fixture write failure"
            ))
            .is_err()
        );
        assert_eq!(toml::to_string(&config).unwrap(), before);
        assert!(selection.apps.contains("studio"));
        persist(&selection, &mut config, |_| Ok(())).unwrap();
        assert_eq!(config.selected_system_apps, vec!["studio"]);
        assert_ne!(config.last_checked.as_deref(), Some("old"));
    }

    #[test]
    fn setup_pty_driver() {
        let Some(root) = std::env::var_os("RPROJ_MACHINE_SETUP_FIXTURE") else {
            return;
        };
        let root = std::path::PathBuf::from(root);
        let mut config = GlobalConfig {
            roblox_projects_root: Some(root.join("projects")),
            last_checked: Some("old".into()),
            ..Default::default()
        };
        let mut terminal = TerminalSession::enter().unwrap();
        let mode = std::env::var("RPROJ_MACHINE_SETUP_MODE").unwrap_or_default();
        let result = run_with(
            &mut terminal,
            &mut config,
            |selection| {
                let root = root.clone();
                let mode = mode.clone();
                Worker::spawn(selection, move |item, _, reporter| {
                    std::fs::write(root.join(item.label().replace(' ', "-")), "ran").unwrap();
                    if mode == "failure" {
                        anyhow::bail!("fixture fatal failure");
                    }
                    if mode == "hold" && *item == worker::Item::Rokit {
                        reporter.capture(
                            std::process::Command::new(std::env::current_exe().unwrap())
                                .args([
                                    "steps::execution::tests::fixture_child",
                                    "--exact",
                                    "--nocapture",
                                ])
                                .env("RPROJ_EXECUTION_FIXTURE_ROOT", &root)
                                .env("RPROJ_EXECUTION_FIXTURE_MODE", "hold"),
                        )?;
                    }
                    reporter.message(MessageKind::Detail, "Fixture item finished");
                    Ok(())
                })
            },
            |updated| {
                if mode == "save-failure" {
                    anyhow::bail!("fixture save failure");
                }
                std::fs::write(root.join("saved.toml"), toml::to_string(updated).unwrap())?;
                Ok(())
            },
        );
        drop(terminal);
        println!("Setup returned: {result:?}");
    }

    fn fixture_session(root: &std::path::Path, mode: &str) -> crate::test_common::Session {
        crate::test_common::Session::start_program(
            &std::env::current_exe().unwrap(),
            root,
            &[
                "commands::machine_setup::tests::setup_pty_driver",
                "--exact",
                "--nocapture",
            ],
            &[
                ("RPROJ_MACHINE_SETUP_FIXTURE", root.to_str().unwrap()),
                ("RPROJ_MACHINE_SETUP_MODE", mode),
            ],
        )
    }

    fn apply(session: &mut crate::test_common::Session) {
        session.wait_for("Apply Setup");
        for _ in 0..6 {
            session.send(crate::test_common::DOWN);
        }
        session.send(crate::test_common::ENTER);
        session.wait_for("[No]");
        session.send("y");
    }

    #[test]
    fn pty_review_cancel_and_confirmation_default_no_do_not_execute() {
        use crate::test_common::{ENTER, ESC};
        let root = tempfile::tempdir().unwrap();
        let mut session = fixture_session(root.path(), "");
        session.wait_for("Apply Setup");
        session.send(ENTER);
        session.wait_for("Space toggle");
        session.send(" ");
        session.send(ESC);
        session.wait_for("Enter edit/apply");
        for _ in 0..6 {
            session.send(crate::test_common::DOWN);
        }
        session.send(ENTER);
        session.wait_for("[No]");
        session.send(ENTER);
        session.wait_for("Enter edit/apply");
        session.send(ESC);
        session.wait_for("Setup returned: Ok(false)");
        assert_eq!(session.finish().code, 0);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn pty_success_failure_and_save_failure_preserve_return_paths() {
        for mode in ["success", "success-back", "failure", "save-failure"] {
            let root = tempfile::tempdir().unwrap();
            let mut session = fixture_session(root.path(), mode);
            apply(&mut session);
            session.wait_for("Enter Done");
            if mode == "success-back" {
                session.send(crate::test_common::ESC);
                session.wait_for("Enter edit/apply");
                session.send(crate::test_common::ESC);
            } else {
                session.send(crate::test_common::ENTER);
            }
            session.wait_for(if mode.starts_with("success") {
                "Setup returned: Ok(true)"
            } else {
                "Setup returned: Err("
            });
            assert_eq!(session.finish().code, 0);
            assert_eq!(
                root.path().join("saved.toml").exists(),
                mode.starts_with("success")
            );
            assert!(!root.path().join("projects").exists());
            if mode == "failure" {
                assert!(!root.path().join("Projects-folder").exists());
            }
        }
    }

    #[test]
    fn pty_stop_waits_for_child_and_never_saves_or_starts_next_item() {
        let root = tempfile::tempdir().unwrap();
        let mut session = fixture_session(root.path(), "hold");
        apply(&mut session);
        session.wait_for("Fixture child active");
        session.send("\x03");
        session.wait_for("Stop after the current item?");
        session.send("y");
        session.wait_for("Stopping after current item");
        assert!(!root.path().join("saved.toml").exists());
        assert!(!root.path().join("child-finished").exists());
        std::fs::write(root.path().join("release"), "").unwrap();
        session.wait_for("Cancelled - completed installations remain");
        session.send(crate::test_common::ENTER);
        session.wait_for("Setup returned: Ok(false)");
        assert_eq!(session.finish().code, 0);
        assert!(root.path().join("child-finished").exists());
        assert!(!root.path().join("Projects-folder").exists());
        assert!(!root.path().join("saved.toml").exists());
    }
    #[test]
    fn review_progress_and_modal_render_at_supported_sizes() {
        for (width, height) in [(120, 30), (280, 70), (80, 24), (40, 10)] {
            let config = GlobalConfig {
                roblox_projects_root: Some("fixture-projects".into()),
                ..Default::default()
            };
            let mut app = App::new(Selection::load(&config).unwrap());
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| app.render(frame)).unwrap();
            app.confirmation = Some(Confirmation::Apply);
            terminal.draw(|frame| app.render(frame)).unwrap();
            app.confirmation = None;
            app.running = true;
            app.statuses = vec![Status::Pending; worker::plan(&app.selection).len()];
            app.message("fixture output");
            terminal.draw(|frame| app.render(frame)).unwrap();
            app.help = true;
            terminal.draw(|frame| app.render(frame)).unwrap();
        }
    }
}
