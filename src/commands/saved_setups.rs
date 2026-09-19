use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::creation::{
    model::{Draft, Effect},
    render,
};
use crate::config::{
    Setups,
    setups::{self, Document},
};
use crate::tui::{self, ConfirmState, InputState, PickerItem, PickerState};

#[cfg(test)]
use crate::test_common as common;
#[cfg(test)]
#[path = "saved_setups_tests.rs"]
mod tests;

enum View {
    Browser,
    Actions,
    Editor(Box<Draft>),
}

enum Modal {
    Help,
    Name {
        rename: bool,
        input: InputState,
    },
    Confirm {
        state: ConfirmState,
        yes: bool,
        delete: bool,
        home: bool,
    },
}

pub struct SavedSetupsApp {
    root: Result<PathBuf, String>,
    picker: PickerState<String>,
    list_state: ListState,
    document: Option<Document>,
    view: View,
    modal: Option<Modal>,
    action: usize,
    details_focus: bool,
    scroll: u16,
    browser_scroll: u16,
    status: String,
}

impl SavedSetupsApp {
    pub fn new() -> Self {
        Self::at(Setups::dir().map_err(|error| format!("{error:#}")))
    }

    fn at(root: Result<PathBuf, String>) -> Self {
        Self {
            root,
            picker: PickerState::new(vec![]),
            list_state: ListState::default(),
            document: None,
            view: View::Browser,
            modal: None,
            action: 0,
            details_focus: false,
            scroll: 0,
            browser_scroll: 0,
            status: String::new(),
        }
    }

    pub fn open(&mut self) {
        self.view = View::Browser;
        self.refresh(None);
        crate::diagnostics::event("screen", "Saved Setups");
    }

    fn refresh(&mut self, select: Option<&str>) {
        let selected = select
            .map(str::to_owned)
            .or_else(|| self.picker.selected_value().cloned());
        let result = self
            .root
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))
            .and_then(|root| setups::list(root));
        match result {
            Ok(entries) => {
                self.picker.items = entries
                    .into_iter()
                    .map(|entry| PickerItem {
                        label: entry.name.clone(),
                        value: entry.name,
                        detail: entry.warning.unwrap_or_default(),
                    })
                    .collect();
                self.picker.selected = selected
                    .and_then(|name| {
                        self.picker
                            .filtered()
                            .iter()
                            .position(|entry| entry.value == name)
                    })
                    .unwrap_or(self.picker.selected)
                    .min(self.picker.filtered().len().saturating_sub(1));
                self.status.clear();
            }
            Err(error) => {
                self.picker.items.clear();
                self.status = format!("{error:#}");
            }
        }
        self.load_selected();
    }

    fn load_selected(&mut self) {
        self.document = None;
        if let (Ok(root), Some(name)) = (&self.root, self.picker.selected_value()) {
            match Document::load(root, name) {
                Ok(document) => self.document = Some(document),
                Err(error) => self.status = format!("{error:#}"),
            }
        }
    }

    fn dirty(&self) -> bool {
        match (&self.view, &self.document) {
            (View::Editor(draft), Some(document)) => {
                document.graph.as_ref().is_some_and(|baseline| {
                    toml::Value::try_from(baseline).ok() != toml::Value::try_from(&draft.graph).ok()
                })
            }
            _ => false,
        }
    }

    fn leave_editor(&mut self, home: bool) -> bool {
        if self.dirty() {
            self.modal = Some(Modal::Confirm {
                state: ConfirmState::new("Discard changes since the last successful save?"),
                yes: false,
                delete: false,
                home,
            });
            false
        } else {
            self.view = View::Actions;
            home
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.modal.is_some() {
            return self.modal_key(key);
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if matches!(self.view, View::Editor(_)) {
            if ctrl && key.code == KeyCode::Char('c') {
                return self.leave_editor(true);
            }
            let effect = if let View::Editor(draft) = &mut self.view {
                draft.key(key)
            } else {
                Effect::None
            };
            match effect {
                Effect::Cancel => return self.leave_editor(false),
                Effect::Save => {
                    if let (View::Editor(draft), Some(document)) =
                        (&mut self.view, &mut self.document)
                    {
                        match document.save(&draft.graph) {
                            Ok(changed) => {
                                draft.status = if changed {
                                    "Saved. Existing projects are unchanged."
                                } else {
                                    "No changes to save."
                                }
                                .into();
                                crate::diagnostics::event(
                                    "setups.save",
                                    if changed { "saved" } else { "unchanged" },
                                );
                            }
                            Err(error) => {
                                draft.status = format!("Save failed: {error:#}");
                                crate::diagnostics::event("setups.save", "failed; draft retained");
                            }
                        }
                    }
                }
                Effect::None => {}
                Effect::Create | Effect::LoadSetup(_) => {
                    self.status = "Project creation is unavailable in the setup manager.".into();
                }
            }
            return false;
        }
        if ctrl && key.code == KeyCode::Char('c') {
            return true;
        }
        match key.code {
            KeyCode::Char('?') => self.modal = Some(Modal::Help),
            KeyCode::Esc if self.details_focus => self.details_focus = false,
            KeyCode::Esc => match self.view {
                View::Browser => return true,
                View::Actions => {
                    self.view = View::Browser;
                    self.scroll = self.browser_scroll;
                    crate::diagnostics::event("screen", "Saved Setups");
                }
                View::Editor(_) => {}
            },
            KeyCode::Tab | KeyCode::BackTab => self.details_focus = !self.details_focus,
            KeyCode::F(5) => {
                self.refresh(None);
            }
            KeyCode::Down if self.details_focus => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up if self.details_focus => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown if self.details_focus => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp if self.details_focus => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Home if self.details_focus => self.scroll = 0,
            KeyCode::End if self.details_focus => {
                self.scroll = self
                    .details()
                    .lines()
                    .count()
                    .saturating_sub(1)
                    .min(u16::MAX as usize) as u16
            }
            KeyCode::Enter if !self.details_focus => match self.view {
                View::Browser if self.document.is_some() => {
                    self.browser_scroll = self.scroll;
                    self.scroll = 0;
                    self.action = 0;
                    self.view = View::Actions;
                    crate::diagnostics::event("screen", "Saved Setup actions");
                }
                View::Actions => self.activate(),
                _ => {}
            },
            _ if !self.details_focus => match self.view {
                View::Browser => {
                    let previous = self.picker.selected_value().cloned();
                    match key.code {
                        KeyCode::Home => self.picker.selected = 0,
                        KeyCode::End => {
                            self.picker.selected = self.picker.filtered().len().saturating_sub(1)
                        }
                        KeyCode::PageUp => {
                            self.picker.selected = self.picker.selected.saturating_sub(10)
                        }
                        KeyCode::PageDown => {
                            self.picker.selected = (self.picker.selected + 10)
                                .min(self.picker.filtered().len().saturating_sub(1))
                        }
                        _ => {
                            self.picker.handle_key(key);
                        }
                    }
                    if previous != self.picker.selected_value().cloned() {
                        self.scroll = 0;
                        self.load_selected();
                    }
                }
                View::Actions => match key.code {
                    KeyCode::Up => self.action = self.action.saturating_sub(1),
                    KeyCode::Down => self.action = (self.action + 1).min(3),
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
        false
    }

    pub fn exit_confirmation_key(&self, key: KeyEvent) -> bool {
        matches!(self.modal, Some(Modal::Confirm { delete: false, .. }))
            && matches!(
                key.code,
                KeyCode::Enter
                    | KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Tab
                    | KeyCode::BackTab
                    | KeyCode::Char('y' | 'Y' | 'n' | 'N')
            )
    }

    fn activate(&mut self) {
        let Some(document) = &self.document else {
            return;
        };
        match self.action {
            0 => match &document.graph {
                Some(graph) if !setups::unsupported(graph) => {
                    let mut draft = Draft::edit_setup(&document.name, graph.clone());
                    match crate::config::GlobalConfig::load() {
                        Ok(config) => {
                            draft.apps = config.selected_system_apps;
                            draft.extensions = config.selected_vscode_extensions;
                        }
                        Err(_) => draft.status = "Machine configuration could not be read; machine-dependent optional files are unavailable. Existing exclusions are preserved.".into(),
                    }
                    self.view = View::Editor(Box::new(draft));
                    crate::diagnostics::event("screen", "Saved Setup editor");
                }
                _ => self.status = "Guided editing is disabled: malformed or unsupported choices cannot be safely revised. Rename, Duplicate and Delete remain available.".into(),
            },
            1 | 2 => self.modal = Some(Modal::Name {
                rename: self.action == 1,
                input: InputState::new(if self.action == 1 { &document.name } else { "" }),
            }),
            3 => self.modal = Some(Modal::Confirm {
                state: ConfirmState::new(format!("Delete saved setup '{}' permanently? Existing projects will not change.", document.name)),
                yes: false, delete: true, home: false,
            }),
            _ => {}
        }
    }

    fn modal_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.modal = None;
            return if matches!(self.view, View::Editor(_)) {
                self.leave_editor(true)
            } else {
                true
            };
        }
        let modal = self.modal.take().unwrap();
        match modal {
            Modal::Help => {
                if !matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                    self.modal = Some(Modal::Help);
                }
            }
            Modal::Name { rename, mut input } => {
                if key.code == KeyCode::Esc {
                    return false;
                }
                if key.code == KeyCode::Enter {
                    let name = input.text().to_owned();
                    let result = self
                        .document
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Refresh and select a setup"))
                        .and_then(|document| {
                            if rename {
                                document.rename(&name)
                            } else {
                                document.duplicate(&name).map(|_| ())
                            }
                        });
                    match result {
                        Ok(()) => {
                            self.view = View::Browser;
                            self.scroll = self.browser_scroll;
                            self.refresh(Some(&name));
                            self.status = if rename {
                                "Renamed."
                            } else {
                                "Duplicated; original bytes preserved."
                            }
                            .into();
                            crate::diagnostics::event(
                                "setups.mutation",
                                if rename { "renamed" } else { "duplicated" },
                            );
                            return false;
                        }
                        Err(error) => {
                            input.error = Some(format!("{error:#}"));
                            self.status = format!("{error:#}");
                            crate::diagnostics::event("setups.mutation", "failed");
                        }
                    }
                } else {
                    input.handle_key(key);
                }
                self.modal = Some(Modal::Name { rename, input });
            }
            Modal::Confirm {
                state,
                mut yes,
                delete,
                home,
            } => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N')
                ) {
                    return false;
                }
                match key.code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => yes = !yes,
                    KeyCode::Char('y') | KeyCode::Char('Y') => yes = true,
                    KeyCode::Enter => {
                        if !yes {
                            return false;
                        }
                        if delete {
                            let result = self
                                .document
                                .as_ref()
                                .ok_or_else(|| anyhow::anyhow!("Refresh and select a setup"))
                                .and_then(Document::delete);
                            match result {
                                Ok(()) => {
                                    self.view = View::Browser;
                                    self.refresh(None);
                                    self.status =
                                        "Deleted saved setup; existing projects are unchanged."
                                            .into();
                                    crate::diagnostics::event("setups.mutation", "deleted");
                                }
                                Err(error) => self.status = format!("Delete failed: {error:#}"),
                            }
                        } else {
                            self.view = View::Actions;
                            return home;
                        }
                        return false;
                    }
                    _ => {}
                }
                self.modal = Some(Modal::Confirm {
                    state,
                    yes,
                    delete,
                    home,
                });
            }
        }
        false
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(Modal::Name { input, .. }) = &mut self.modal {
            for ch in text.chars().filter(|ch| !ch.is_control()) {
                input.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
            }
        } else if self.modal.is_none() {
            if let View::Editor(draft) = &mut self.view {
                draft.paste(text);
            } else if matches!(self.view, View::Browser) && !self.details_focus {
                for ch in text.chars().filter(|ch| !ch.is_control()) {
                    self.picker
                        .handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
                }
                self.load_selected();
            }
        }
    }

    fn details(&self) -> String {
        let Some(document) = &self.document else {
            return "No saved setup selected. New setups can be saved during New Project.\nStorage is not created by browsing.".into();
        };
        let path = self
            .root
            .as_ref()
            .map(|root| {
                root.join(format!("{}.toml", document.name))
                    .display()
                    .to_string()
            })
            .unwrap_or_default();
        let warning = document.problem.clone().unwrap_or_else(|| {
            if document.graph.as_ref().is_some_and(setups::unsupported) {
                "Unsupported choices: guided editing is disabled; original bytes are retained."
                    .into()
            } else {
                String::new()
            }
        });
        format!(
            "{}\n{path}\n\nFuture reuse only; existing projects are unchanged.\n{warning}\n\n{}",
            document.name,
            String::from_utf8_lossy(&document.bytes)
        )
    }

    pub fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        if let View::Editor(draft) = &self.view {
            render::draw(frame, draft, "");
        } else if tui::is_too_small(area) {
            frame.render_widget(
                Paragraph::new(
                    "Saved Setups\nResize to at least 60x16\n? Help  Esc Back  Ctrl+C Home",
                ),
                area,
            );
        } else {
            let rows = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);
            frame.render_widget(
                Paragraph::new("Saved Setups").style(tui::title_style()),
                rows[0],
            );
            let panes = tui::responsive_panes(rows[1], 40);
            let (items, selected, title) = if matches!(self.view, View::Browser) {
                (
                    self.picker
                        .filtered()
                        .iter()
                        .map(|item| {
                            ListItem::new(format!(
                                "{}{}",
                                item.label,
                                if item.detail.is_empty() {
                                    ""
                                } else {
                                    " [warning]"
                                }
                            ))
                        })
                        .collect::<Vec<_>>(),
                    self.picker.selected,
                    format!(" Setups | Filter: {} ", self.picker.query.text()),
                )
            } else {
                (
                    ["Edit", "Rename", "Duplicate", "Delete"]
                        .iter()
                        .map(|label| ListItem::new(*label))
                        .collect(),
                    self.action,
                    " Actions ".into(),
                )
            };
            self.list_state.select(if items.is_empty() {
                None
            } else {
                Some(selected)
            });
            frame.render_stateful_widget(
                List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(title))
                    .highlight_style(tui::selected_style(!self.details_focus)),
                panes[0],
                &mut self.list_state,
            );
            frame.render_widget(
                Paragraph::new(format!("{}\n\n{}", self.status, self.details()))
                    .wrap(Wrap { trim: false })
                    .scroll((self.scroll, 0))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Composition "),
                    ),
                panes[1],
            );
            tui::render_footer(
                frame,
                rows[2],
                &self.status,
                "Enter select  Tab focus  F5 refresh  PgUp/Dn scroll  Esc back  Ctrl+C Home  ? help",
                !self.status.is_empty(),
            );
        }
        match &self.modal {
            Some(Modal::Name { rename, input }) => tui::render_input(
                frame,
                area,
                if *rename {
                    "Rename setup"
                } else {
                    "Duplicate setup"
                },
                "Choose a distinct setup name",
                input,
            ),
            Some(Modal::Confirm { state, yes, .. }) => {
                let popup = tui::centered(area, 72, 10);
                frame.render_widget(Clear, popup);
                frame.render_widget(
                    Paragraph::new(format!(
                        "{}\n\n{}  {}\nLeft/Right select; Enter confirms; Esc cancels",
                        state.prompt,
                        if !yes { "[No]" } else { " No " },
                        if *yes { "[Yes]" } else { " Yes " }
                    ))
                    .wrap(Wrap { trim: false })
                    .block(Block::default().borders(Borders::ALL).title(" Confirm ")),
                    popup,
                );
            }
            Some(Modal::Help) => {
                let popup = tui::centered(area, 76, 16);
                frame.render_widget(Clear, popup);
                frame.render_widget(Paragraph::new("Saved Setups\n\nType to filter names. Enter opens actions. F5 refreshes storage.\nTab switches panes; arrows and Page Up/Down scroll details.\nEdit uses composition controls; Ctrl+S saves from Review and stays open.\nEsc returns locally; Ctrl+C returns Home. Unsaved edits require confirmation.\nRename and Duplicate preserve bytes. Delete defaults to No.\nChanged composition saves may reformat TOML and remove comments.\nExternal modifications require leaving the editor and refreshing.\nNo projects or applications are created or changed.")
                    .wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL).title(" Help | Esc close ")), popup);
            }
            None => {}
        }
    }
}
