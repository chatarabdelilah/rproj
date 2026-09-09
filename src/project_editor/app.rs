use std::collections::BTreeSet;
use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::{Value, json};
use unicode_width::UnicodeWidthStr;

use super::metadata::{Metadata, ValueKind};
use super::model::{EditorModel, TreeRow, representable};
use super::text_buffer::TextBuffer;
use crate::steps::rojo;
use crate::tui::{
    ConfirmState, InputState, PickerItem, PickerState, TerminalSession, centered, is_too_small,
    render_confirm, render_footer, render_input, render_picker, responsive_panes,
};

const SETTINGS: &[(&str, &str, SettingKind)] = &[
    ("servePort", "Serve port", SettingKind::Port),
    ("serveAddress", "Serve address", SettingKind::Text),
    (
        "servePlaceIds",
        "Allowed place IDs",
        SettingKind::IntegerList,
    ),
    ("placeId", "Place ID", SettingKind::Integer),
    ("gameId", "Game ID", SettingKind::Integer),
    ("serveAllowedHosts", "Allowed hosts", SettingKind::TextList),
    (
        "globIgnorePaths",
        "Ignored path globs",
        SettingKind::TextList,
    ),
    (
        "emitLegacyScripts",
        "Emit legacy scripts",
        SettingKind::Bool,
    ),
];

#[derive(Clone, Copy)]
enum SettingKind {
    Port,
    Text,
    Integer,
    IntegerList,
    TextList,
    Bool,
}

pub enum Outcome {
    Reset,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Explorer,
    Json,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Tree,
    Inspector,
}

#[derive(Clone)]
enum InspectorRow {
    Class,
    IgnoreUnknown,
    Property(String, Option<ValueKind>),
    AddProperty,
    Attribute(String, Option<ValueKind>),
    AddAttribute,
    Setting(&'static str, &'static str, SettingKind),
}

#[derive(Clone)]
enum InputAction {
    Rename(Vec<String>),
    Property(Vec<String>, String, ValueKind),
    AttributeName(Vec<String>),
    AttributeValue(Vec<String>, String, ValueKind),
    Setting(&'static str, SettingKind),
}

#[derive(Clone)]
enum PickAction {
    AddClass(Vec<String>),
    ChangeClass(Vec<String>),
    AddProperty(Vec<String>),
    Move(Vec<String>),
    EnumProperty(Vec<String>, String, ValueKind),
    AttributeKind(Vec<String>, String),
}

#[derive(Clone)]
enum ConfirmAction {
    Delete(Vec<String>),
    Reset,
    Exit,
}

#[derive(Clone)]
struct PickOption {
    label: String,
    detail: String,
    service: bool,
}

enum Modal {
    Help,
    Error {
        message: String,
        scroll: std::cell::Cell<u16>,
    },
    Input {
        title: String,
        hint: String,
        state: InputState,
        action: InputAction,
    },
    Picker {
        title: String,
        state: PickerState<PickOption>,
        action: PickAction,
    },
    Confirm {
        state: ConfirmState,
        action: ConfirmAction,
    },
}

pub struct App {
    model: Option<EditorModel>,
    metadata: Metadata,
    mode: Mode,
    focus: Focus,
    selected_tree: usize,
    selected_inspector: usize,
    collapsed: BTreeSet<Vec<String>>,
    json: TextBuffer,
    json_error: Option<String>,
    modal: Option<Modal>,
    status: String,
    initially_corrupt: bool,
}

impl App {
    fn from_text(text: String) -> Self {
        let parsed = parse_explorer_json(&text);
        let (model, mode, json_error, initially_corrupt) = match parsed {
            Ok(value) => match EditorModel::new(value) {
                Ok(model) => (Some(model), Mode::Explorer, None, false),
                Err(error) => (None, Mode::Json, Some(error.to_string()), true),
            },
            Err(error) => (None, Mode::Json, Some(format!("{error:#}")), true),
        };
        Self {
            model,
            metadata: Metadata::bundled(),
            mode,
            focus: Focus::Tree,
            selected_tree: 0,
            selected_inspector: 0,
            collapsed: BTreeSet::new(),
            json: TextBuffer::new(&text),
            json_error,
            modal: None,
            status: if initially_corrupt {
                "Repair the JSON or restore the built-in template.".into()
            } else {
                "Future projects use this template; existing projects are unchanged.".into()
            },
            initially_corrupt,
        }
    }

    fn model(&self) -> &EditorModel {
        self.model.as_ref().expect("Explorer always has a model")
    }
    fn model_mut(&mut self) -> &mut EditorModel {
        self.model.as_mut().expect("Explorer always has a model")
    }

    fn visible_rows(&self) -> Vec<TreeRow> {
        self.model()
            .rows()
            .into_iter()
            .filter(|row| {
                !self
                    .collapsed
                    .iter()
                    .any(|path| row.path.len() > path.len() && row.path.starts_with(path))
            })
            .collect()
    }

    fn selected_path(&self) -> Vec<String> {
        let rows = self.visible_rows();
        rows.get(self.selected_tree)
            .or_else(|| rows.last())
            .map(|row| row.path.clone())
            .unwrap_or_default()
    }

    fn inspector_rows(&self) -> Vec<InspectorRow> {
        let path = self.selected_path();
        let node = self.model().node(&path).expect("selected node exists");
        let class = self.model().class_name(&path);
        let mut rows = vec![InspectorRow::Class, InspectorRow::IgnoreUnknown];
        if let Some(properties) = node.get("$properties").and_then(Value::as_object) {
            for name in properties
                .keys()
                .filter(|name| name.as_str() != "Attributes")
            {
                rows.push(InspectorRow::Property(
                    name.clone(),
                    self.metadata
                        .property(&class, name)
                        .map(|property| property.kind),
                ));
            }
        }
        rows.push(InspectorRow::AddProperty);
        if let Some(attributes) = node
            .get("$properties")
            .and_then(Value::as_object)
            .and_then(|p| p.get("Attributes"))
            .and_then(Value::as_object)
        {
            for (name, value) in attributes {
                rows.push(InspectorRow::Attribute(name.clone(), attribute_kind(value)));
            }
        }
        rows.push(InspectorRow::AddAttribute);
        if path.is_empty() {
            rows.extend(
                SETTINGS
                    .iter()
                    .map(|(key, label, kind)| InspectorRow::Setting(key, label, *kind)),
            );
        }
        rows
    }

    fn clamp_selection(&mut self) {
        self.selected_tree = self
            .selected_tree
            .min(self.visible_rows().len().saturating_sub(1));
        self.selected_inspector = self
            .selected_inspector
            .min(self.inspector_rows().len().saturating_sub(1));
    }

    fn enter_json(&mut self) {
        crate::diagnostics::event("template.mode", "advanced JSON; content omitted");
        self.json = TextBuffer::new(&format!(
            "{}\n",
            serde_json::to_string_pretty(self.model().value()).expect("JSON value serializes")
        ));
        self.json_error = None;
        self.mode = Mode::Json;
    }

    fn leave_json(&mut self) {
        match parse_explorer_json(&self.json.text()) {
            Ok(value) => {
                if let Some(model) = &mut self.model {
                    if let Err(error) = model.replace_from_json(value) {
                        self.json_error = Some(error.to_string());
                        return;
                    }
                } else {
                    match EditorModel::new(value) {
                        Ok(model) => self.model = Some(model),
                        Err(error) => {
                            self.json_error = Some(error.to_string());
                            return;
                        }
                    }
                }
                self.json_error = None;
                self.mode = Mode::Explorer;
                self.selected_tree = 0;
                self.status = "JSON changes applied to the draft.".into();
            }
            Err(error) => self.json_error = Some(format!("{error:#}")),
        }
    }

    fn dirty(&self) -> bool {
        match self.mode {
            Mode::Explorer => self.initially_corrupt || self.model().is_dirty(),
            Mode::Json => {
                self.initially_corrupt
                    || self.model.as_ref().is_none_or(|model| {
                        parse_explorer_json(&self.json.text())
                            .map_or(true, |value| !model.matches_saved(&value))
                    })
            }
        }
    }

    fn error(&mut self, error: impl ToString) {
        crate::diagnostics::event("template.error", "draft error; contents omitted");
        self.status = error.to_string();
    }
}

pub fn run(
    text: String,
    save: impl FnMut(&Value) -> Result<()>,
    reset: impl FnMut() -> Result<()>,
) -> Result<Outcome> {
    crate::diagnostics::event("template.open", "template content omitted");
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!("`rproj configure project` requires an interactive terminal");
    }
    let mut terminal = TerminalSession::enter()?;
    run_in(&mut terminal, text, save, reset)
}

pub fn run_in(
    terminal: &mut TerminalSession,
    text: String,
    mut save: impl FnMut(&Value) -> Result<()>,
    mut reset: impl FnMut() -> Result<()>,
) -> Result<Outcome> {
    crate::diagnostics::event("screen", "Template");
    let mut app = App::from_text(text);
    let mut pending = None;
    let mut last_draw = std::time::Instant::now();
    let mut small = false;
    loop {
        if pending.is_none() || last_draw.elapsed() >= std::time::Duration::from_millis(16) {
            terminal.draw(|frame| {
                small = is_too_small(frame.area());
                render(frame, &app);
            })?;
            last_draw = std::time::Instant::now();
        }
        let event = match pending.take() {
            Some(event) => event,
            None => terminal.read_event()?,
        };
        let previous_status = app.status.clone();
        let request = match event {
            Event::Key(key) if key.kind != event::KeyEventKind::Release => {
                if small
                    && !matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
                    && !(key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c'))
                {
                    continue;
                }
                handle_key(&mut app, key)
            }
            Event::Paste(text) if !small && app.mode == Mode::Json && app.modal.is_none() => {
                app.json.insert_text(&text);
                None
            }
            Event::Resize(_, _) => None,
            _ => None,
        };
        if app.status != previous_status {
            crate::diagnostics::event("template.status", "draft status changed; contents omitted");
        }
        match request {
            Some(Request::Save(value)) => {
                crate::diagnostics::event("template.save", "requested; validating");
                app.status = "Validating every generated Rojo project variant...".into();
                terminal.draw(|frame| render(frame, &app))?;
                validate_save(&mut app, value, &mut save);
            }
            Some(Request::Reset) => {
                crate::diagnostics::event("template.reset", "confirmed");
                match reset() {
                    Ok(()) => return Ok(Outcome::Reset),
                    Err(error) => {
                        app.error(format!("Template was not reset: {error:#}"));
                        app.modal = Some(Modal::Error {
                            message: format!(
                                "{}\n\n{}",
                                app.status,
                                crate::diagnostics::path_message()
                            ),
                            scroll: 0.into(),
                        });
                    }
                }
            }
            Some(Request::Cancel) => {
                crate::diagnostics::event("template.close", "discarded unsaved changes only");
                return Ok(Outcome::Cancel);
            }
            None => {}
        }
        pending = terminal.poll_event(std::time::Duration::ZERO)?;
    }
}

fn validate_save(
    app: &mut App,
    value: Value,
    validate: &mut impl FnMut(&Value) -> Result<()>,
) -> bool {
    match representable(&value)
        .and_then(|()| rojo::validate_template_structure(&value))
        .and_then(|()| validate(&value))
    {
        Ok(()) => {
            if let Some(model) = &mut app.model {
                if model.value() != &value {
                    model
                        .replace_from_json(value.clone())
                        .expect("save validates representability");
                }
                model.mark_saved();
            } else {
                app.model =
                    Some(EditorModel::new(value.clone()).expect("save validates representability"));
            }
            app.initially_corrupt = false;
            app.json_error = None;
            app.status = "Template saved. Future projects will inherit this tree.".into();
            crate::diagnostics::event("template.saved", "atomic replacement complete");
            true
        }
        Err(error) => {
            app.error(format!("Template was not saved: {error:#}"));
            if app.mode == Mode::Json {
                app.json_error = Some(format!("{error:#}"));
            }
            app.modal = Some(Modal::Error {
                message: format!("{}\n\n{}", app.status, crate::diagnostics::path_message()),
                scroll: 0.into(),
            });
            false
        }
    }
}

enum Request {
    Save(Value),
    Reset,
    Cancel,
}

fn handle_key(app: &mut App, key: KeyEvent) -> Option<Request> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        if app.dirty() {
            app.modal = Some(confirm("Discard changes and exit?", ConfirmAction::Exit));
            return None;
        }
        return Some(Request::Cancel);
    }
    if app.modal.is_some() {
        return handle_modal(app, key);
    }
    if key.code == KeyCode::Char('?') {
        app.modal = Some(Modal::Help);
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
        app.modal = Some(confirm(
            "Restore the built-in template and discard this draft?",
            ConfirmAction::Reset,
        ));
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
        return save_request(app);
    }
    match app.mode {
        Mode::Json => handle_json_key(app, key),
        Mode::Explorer => handle_explorer_key(app, key),
    }
}

fn save_request(app: &mut App) -> Option<Request> {
    let value = if app.mode == Mode::Json {
        match parse_explorer_json(&app.json.text()) {
            Ok(value) => value,
            Err(error) => {
                app.json_error = Some(format!("{error:#}"));
                app.modal = Some(Modal::Error {
                    message: format!(
                        "Template was not saved: {error:#}\n\n{}",
                        crate::diagnostics::path_message()
                    ),
                    scroll: 0.into(),
                });
                crate::diagnostics::event(
                    "template.save",
                    "rejected invalid JSON; contents omitted",
                );
                return None;
            }
        }
    } else {
        app.model().value().clone()
    };
    Some(Request::Save(value))
}

fn handle_json_key(app: &mut App, key: KeyEvent) -> Option<Request> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Esc => {
            if app.initially_corrupt && app.model.is_none() {
                app.modal = Some(confirm(
                    "Exit without repairing the saved template?",
                    ConfirmAction::Exit,
                ));
            } else {
                app.leave_json();
            }
        }
        KeyCode::Char('z') if ctrl => {
            app.json.undo();
        }
        KeyCode::Char('y') if ctrl => {
            app.json.redo();
        }
        KeyCode::Char(character) if !ctrl => app.json.insert_char(character),
        KeyCode::Enter => app.json.newline(),
        KeyCode::Backspace => app.json.backspace(),
        KeyCode::Delete => app.json.delete(),
        KeyCode::Left => app.json.left(),
        KeyCode::Right => app.json.right(),
        KeyCode::Up => app.json.up(),
        KeyCode::Down => app.json.down(),
        KeyCode::Home => app.json.home(),
        KeyCode::End => app.json.end(),
        KeyCode::PageUp => app.json.page_up(15),
        KeyCode::PageDown => app.json.page_down(15),
        KeyCode::Tab => app.json.insert_text("  "),
        _ => {}
    }
    None
}

fn handle_explorer_key(app: &mut App, key: KeyEvent) -> Option<Request> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('e') if ctrl => app.enter_json(),
        KeyCode::Char('z') if ctrl => {
            if !app.model_mut().undo() {
                app.error("Nothing to undo.");
            }
            app.clamp_selection();
        }
        KeyCode::Char('y') if ctrl => {
            if !app.model_mut().redo() {
                app.error("Nothing to redo.");
            }
            app.clamp_selection();
        }
        KeyCode::Tab => {
            app.focus = if app.focus == Focus::Tree {
                Focus::Inspector
            } else {
                Focus::Tree
            };
        }
        KeyCode::BackTab => {
            app.focus = if app.focus == Focus::Tree {
                Focus::Inspector
            } else {
                Focus::Tree
            };
        }
        KeyCode::Esc => {
            if app.dirty() {
                app.modal = Some(confirm(
                    "Discard all unsaved template changes?",
                    ConfirmAction::Exit,
                ));
            } else {
                return Some(Request::Cancel);
            }
        }
        KeyCode::Up => match app.focus {
            Focus::Tree => app.selected_tree = app.selected_tree.saturating_sub(1),
            Focus::Inspector => app.selected_inspector = app.selected_inspector.saturating_sub(1),
        },
        KeyCode::Down => match app.focus {
            Focus::Tree => {
                app.selected_tree =
                    (app.selected_tree + 1).min(app.visible_rows().len().saturating_sub(1))
            }
            Focus::Inspector => {
                app.selected_inspector =
                    (app.selected_inspector + 1).min(app.inspector_rows().len().saturating_sub(1))
            }
        },
        KeyCode::Left if app.focus == Focus::Tree => collapse_or_parent(app),
        KeyCode::Right if app.focus == Focus::Tree => {
            app.collapsed.remove(&app.selected_path());
        }
        KeyCode::Enter if app.focus == Focus::Inspector => activate_inspector(app),
        KeyCode::Enter if app.focus == Focus::Tree => {
            app.focus = Focus::Inspector;
            app.selected_inspector = 0;
        }
        KeyCode::Char('a') | KeyCode::Char('A') => open_class_picker(app, true),
        KeyCode::F(2) => open_rename(app),
        KeyCode::Char('d') | KeyCode::Char('D') => duplicate_selected(app),
        KeyCode::Char('m') | KeyCode::Char('M') => open_move_picker(app),
        KeyCode::Delete if app.focus == Focus::Inspector => delete_inspector(app),
        KeyCode::Delete => open_delete(app),
        _ => {}
    }
    None
}

fn collapse_or_parent(app: &mut App) {
    let path = app.selected_path();
    let row = &app.visible_rows()[app.selected_tree];
    if row.has_children && !app.collapsed.contains(&path) {
        app.collapsed.insert(path);
    } else if !path.is_empty() {
        let parent = &path[..path.len() - 1];
        if let Some(index) = app.visible_rows().iter().position(|row| row.path == parent) {
            app.selected_tree = index;
        }
    }
}

fn open_class_picker(app: &mut App, add: bool) {
    let path = app.selected_path();
    if add && !app.model().can_add_children(&path) {
        app.error("rproj-managed source mounts cannot contain template children.");
        return;
    }
    if !add && !app.model().can_restructure(&path) {
        app.error("This instance's class is managed by rproj.");
        return;
    }
    let at_root = if add {
        path.is_empty()
    } else {
        path.len() == 1
    };
    let options = app
        .metadata
        .classes(at_root)
        .into_iter()
        .map(|class| PickOption {
            detail: if class.service {
                "service".into()
            } else {
                "instance".into()
            },
            label: class.name,
            service: class.service,
        })
        .collect();
    app.modal = Some(picker(
        if add { "Add instance" } else { "Change class" },
        options,
        if add {
            PickAction::AddClass(path)
        } else {
            PickAction::ChangeClass(path)
        },
    ));
}

fn open_rename(app: &mut App) {
    let path = app.selected_path();
    if !app.model().can_restructure(&path) {
        app.error("This instance's name is managed by rproj.");
        return;
    }
    let text = path.last().cloned().unwrap_or_default();
    app.modal = Some(input(
        "Rename instance",
        "Instance name",
        text,
        InputAction::Rename(path),
    ));
}

fn duplicate_selected(app: &mut App) {
    let path = app.selected_path();
    if !app.model().can_restructure(&path) {
        app.error("This instance is managed by rproj.");
        return;
    }
    match app.model_mut().duplicate(&path) {
        Ok(next) => select_path(app, &next),
        Err(error) => app.error(error),
    }
}

fn open_move_picker(app: &mut App) {
    let path = app.selected_path();
    if !app.model().can_restructure(&path) {
        app.error("This instance is managed by rproj.");
        return;
    }
    let options = app
        .model()
        .rows()
        .into_iter()
        .filter(|row| !row.path.starts_with(&path) && app.model().can_add_children(&row.path))
        .map(|row| PickOption {
            label: if row.path.is_empty() {
                "DataModel".into()
            } else {
                row.path.join("/")
            },
            detail: row.class_name,
            service: false,
        })
        .collect();
    app.modal = Some(picker("Move under", options, PickAction::Move(path)));
}

fn open_delete(app: &mut App) {
    let path = app.selected_path();
    if !app.model().can_restructure(&path) {
        app.error("This instance is managed by rproj.");
        return;
    }
    let name = path.last().cloned().unwrap_or_else(|| "DataModel".into());
    app.modal = Some(confirm(
        format!("Delete `{name}` and all of its children?"),
        ConfirmAction::Delete(path),
    ));
}

fn activate_inspector(app: &mut App) {
    let Some(row) = app.inspector_rows().get(app.selected_inspector).cloned() else {
        return;
    };
    let path = app.selected_path();
    match row {
        InspectorRow::Class => open_class_picker(app, false),
        InspectorRow::IgnoreUnknown => {
            let current = app
                .model()
                .node(&path)
                .and_then(|n| n.get("$ignoreUnknownInstances"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if let Err(error) = app.model_mut().set_ignore_unknown(&path, !current) {
                app.error(error);
            }
        }
        InspectorRow::Property(name, Some(kind)) => open_value_editor(app, path, name, kind, false),
        InspectorRow::Property(_, None) => {
            app.error("This value is preserved but can only be edited in Advanced JSON mode.")
        }
        InspectorRow::AddProperty => open_property_picker(app, path),
        InspectorRow::Attribute(name, Some(kind)) => open_value_editor(app, path, name, kind, true),
        InspectorRow::Attribute(_, None) => app.error(
            "This attribute type is preserved but can only be edited in Advanced JSON mode.",
        ),
        InspectorRow::AddAttribute => {
            app.modal = Some(input(
                "Add attribute",
                "Attribute name",
                "",
                InputAction::AttributeName(path),
            ))
        }
        InspectorRow::Setting(key, _, SettingKind::Bool) => {
            let current = app
                .model()
                .value()
                .get(key)
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if let Err(error) = app.model_mut().set_setting(key, Some(json!(!current))) {
                app.error(error);
            }
        }
        InspectorRow::Setting(key, _, kind) => {
            let text = setting_text(app.model().value().get(key));
            app.modal = Some(input(
                format!("Edit {key}"),
                "Leave empty to remove this setting",
                text,
                InputAction::Setting(key, kind),
            ));
        }
    }
}

fn delete_inspector(app: &mut App) {
    let Some(row) = app.inspector_rows().get(app.selected_inspector).cloned() else {
        return;
    };
    let path = app.selected_path();
    let result = match row {
        InspectorRow::Property(name, _) => app.model_mut().remove_property(&path, &name),
        InspectorRow::Attribute(name, _) => app.model_mut().remove_attribute(&path, &name),
        InspectorRow::Setting(key, _, _) => app.model_mut().set_setting(key, None),
        _ => {
            app.error("This field cannot be removed.");
            return;
        }
    };
    if let Err(error) = result {
        app.error(error);
    }
    app.clamp_selection();
}

fn open_property_picker(app: &mut App, path: Vec<String>) {
    let class = app.model().class_name(&path);
    let existing = app
        .model()
        .node(&path)
        .and_then(|n| n.get("$properties"))
        .and_then(Value::as_object);
    let options = app
        .metadata
        .properties(&class)
        .into_iter()
        .filter(|property| existing.is_none_or(|e| !e.contains_key(&property.name)))
        .map(|property| PickOption {
            label: property.name,
            detail: property.kind.label().into(),
            service: false,
        })
        .collect();
    app.modal = Some(picker(
        format!("Add {class} property"),
        options,
        PickAction::AddProperty(path),
    ));
}

fn open_value_editor(
    app: &mut App,
    path: Vec<String>,
    name: String,
    kind: ValueKind,
    attribute: bool,
) {
    if let ValueKind::Enum(items) = &kind {
        let options = items
            .iter()
            .map(|item| PickOption {
                label: item.clone(),
                detail: "enum item".into(),
                service: false,
            })
            .collect();
        app.modal = Some(picker(
            format!("Set {name}"),
            options,
            PickAction::EnumProperty(path, name, kind),
        ));
        return;
    }
    let current = if attribute {
        app.model()
            .node(&path)
            .and_then(|n| n.get("$properties"))
            .and_then(|p| p.get("Attributes"))
            .and_then(|a| a.get(&name))
            .and_then(explicit_inner)
    } else {
        app.model()
            .node(&path)
            .and_then(|n| n.get("$properties"))
            .and_then(|p| p.get(&name))
    };
    app.modal = Some(input(
        format!("Set {name}"),
        kind.label(),
        value_text(current),
        if attribute {
            InputAction::AttributeValue(path, name, kind)
        } else {
            InputAction::Property(path, name, kind)
        },
    ));
}

fn input(
    title: impl Into<String>,
    hint: impl Into<String>,
    text: impl Into<String>,
    action: InputAction,
) -> Modal {
    Modal::Input {
        title: title.into(),
        hint: hint.into(),
        state: InputState::new(text),
        action,
    }
}

fn picker(title: impl Into<String>, options: Vec<PickOption>, action: PickAction) -> Modal {
    let items = options
        .into_iter()
        .map(|option| PickerItem {
            label: option.label.clone(),
            detail: option.detail.clone(),
            value: option,
        })
        .collect();
    Modal::Picker {
        title: title.into(),
        state: PickerState::new(items),
        action,
    }
}

fn confirm(prompt: impl Into<String>, action: ConfirmAction) -> Modal {
    Modal::Confirm {
        state: ConfirmState::new(prompt),
        action,
    }
}

fn handle_modal(app: &mut App, key: KeyEvent) -> Option<Request> {
    let modal = app.modal.take().expect("checked by caller");
    match modal {
        Modal::Error { message, scroll } => {
            if !matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                scroll.set(match key.code {
                    KeyCode::Up => scroll.get().saturating_sub(1),
                    KeyCode::Down => scroll.get().saturating_add(1),
                    KeyCode::PageUp => scroll.get().saturating_sub(10),
                    KeyCode::PageDown => scroll.get().saturating_add(10),
                    KeyCode::Home => 0,
                    KeyCode::End => u16::MAX,
                    _ => scroll.get(),
                });
                app.modal = Some(Modal::Error { message, scroll });
            }
            None
        }
        Modal::Help => {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                None
            } else {
                app.modal = Some(Modal::Help);
                None
            }
        }
        Modal::Confirm { state, action } => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => match action {
                ConfirmAction::Reset => Some(Request::Reset),
                ConfirmAction::Exit => Some(Request::Cancel),
                ConfirmAction::Delete(path) => {
                    if let Err(error) = app.model_mut().delete(&path) {
                        app.error(error);
                    }
                    app.selected_tree = app.selected_tree.saturating_sub(1);
                    app.clamp_selection();
                    None
                }
            },
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => None,
            _ => {
                app.modal = Some(Modal::Confirm { state, action });
                None
            }
        },
        Modal::Input {
            title,
            hint,
            mut state,
            action,
        } => match key.code {
            KeyCode::Esc => None,
            KeyCode::Enter => match apply_input(app, action.clone(), state.text()) {
                Ok(()) => None,
                Err(message) => {
                    crate::diagnostics::event(
                        "template.input.rejected",
                        message.lines().next().unwrap_or("invalid input"),
                    );
                    state.error = Some(message);
                    app.modal = Some(Modal::Input {
                        title,
                        hint,
                        state,
                        action,
                    });
                    None
                }
            },
            _ => {
                state.handle_key(key);
                app.modal = Some(Modal::Input {
                    title,
                    hint,
                    state,
                    action,
                });
                None
            }
        },
        Modal::Picker {
            title,
            mut state,
            action,
        } => match key.code {
            KeyCode::Esc => None,
            KeyCode::Enter => {
                let picked = state.selected_value().cloned();
                if let Some(picked) = picked {
                    if let Err(error) = apply_pick(app, action, picked) {
                        app.error(error);
                    }
                } else {
                    app.modal = Some(Modal::Picker {
                        title,
                        state,
                        action,
                    });
                }
                None
            }
            _ => {
                state.handle_key(key);
                app.modal = Some(Modal::Picker {
                    title,
                    state,
                    action,
                });
                None
            }
        },
    }
}

fn apply_input(app: &mut App, action: InputAction, text: &str) -> std::result::Result<(), String> {
    let kind = match &action {
        InputAction::Rename(_) => "rename",
        InputAction::Property(_, _, _) => "property",
        InputAction::AttributeName(_) => "attribute name",
        InputAction::AttributeValue(_, _, _) => "attribute value",
        InputAction::Setting(_, _) => "setting",
    };
    crate::diagnostics::event(
        "template.input",
        format!("{kind}; {} characters; value omitted", text.chars().count()),
    );
    match action {
        InputAction::Rename(path) => app
            .model_mut()
            .rename(&path, text)
            .map(|next| select_path(app, &next))
            .map_err(|e| e.to_string()),
        InputAction::Property(path, name, kind) => kind.parse(text).and_then(|value| {
            app.model_mut()
                .set_property(&path, &name, value)
                .map_err(|e| e.to_string())
        }),
        InputAction::AttributeName(path) => {
            let name = text.trim();
            if name.is_empty() {
                return Err("attribute names cannot be empty".into());
            }
            let options = attribute_kinds()
                .into_iter()
                .map(|(label, kind)| PickOption {
                    label: label.into(),
                    detail: kind.label().into(),
                    service: false,
                })
                .collect();
            app.modal = Some(picker(
                format!("Type for {name}"),
                options,
                PickAction::AttributeKind(path, name.into()),
            ));
            Ok(())
        }
        InputAction::AttributeValue(path, name, kind) => {
            let value = kind.parse(text)?;
            set_attribute(app.model_mut(), &path, &name, attribute_value(&kind, value))
                .map_err(|e| e.to_string())
        }
        InputAction::Setting(key, kind) => {
            let value = parse_setting(kind, text)?;
            app.model_mut()
                .set_setting(key, value)
                .map_err(|e| e.to_string())
        }
    }
}

fn apply_pick(app: &mut App, action: PickAction, picked: PickOption) -> Result<()> {
    crate::diagnostics::event("template.choice", &picked.label);
    match action {
        PickAction::AddClass(parent) => {
            let path = app
                .model_mut()
                .add(&parent, &picked.label, picked.service)?;
            select_path(app, &path);
        }
        PickAction::ChangeClass(path) => {
            let next = app
                .model_mut()
                .change_class(&path, &picked.label, picked.service)?;
            select_path(app, &next);
        }
        PickAction::AddProperty(path) => {
            let class = app.model().class_name(&path);
            let property = app
                .metadata
                .property(&class, &picked.label)
                .context("property metadata disappeared")?;
            let name = property.name.clone();
            if let ValueKind::Enum(_) = property.kind {
                open_value_editor(app, path, name, property.kind, false);
            } else {
                app.model_mut()
                    .set_property(&path, &name, property.kind.default_value())?;
                open_value_editor(app, path, name, property.kind, false);
            }
        }
        PickAction::Move(path) => {
            let destination = if picked.label == "DataModel" {
                Vec::new()
            } else {
                picked.label.split('/').map(ToString::to_string).collect()
            };
            let next = app.model_mut().move_to(&path, &destination)?;
            select_path(app, &next);
        }
        PickAction::EnumProperty(path, name, kind) => {
            let value = kind.parse(&picked.label).map_err(anyhow::Error::msg)?;
            app.model_mut().set_property(&path, &name, value)?;
        }
        PickAction::AttributeKind(path, name) => {
            let kind = attribute_kinds()
                .into_iter()
                .find(|(label, _)| *label == picked.label)
                .map(|(_, kind)| kind)
                .context("attribute type disappeared")?;
            open_value_editor(app, path, name, kind, true);
        }
    }
    Ok(())
}

fn set_attribute(model: &mut EditorModel, path: &[String], name: &str, value: Value) -> Result<()> {
    let mut attributes = model
        .node(path)
        .and_then(|n| n.get("$properties"))
        .and_then(|p| p.get("Attributes"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    attributes.insert(name.into(), value);
    model.set_property(path, "Attributes", Value::Object(attributes))
}

fn attribute_value(kind: &ValueKind, value: Value) -> Value {
    let label = match kind {
        ValueKind::Bool => "Bool",
        ValueKind::String => "String",
        ValueKind::Integer | ValueKind::Number => "Float64",
        ValueKind::BrickColor => return value,
        ValueKind::Color3 => "Color3",
        ValueKind::Vector2 => "Vector2",
        ValueKind::Vector3 => "Vector3",
        ValueKind::UDim | ValueKind::UDim2 | ValueKind::Rect => return value,
        ValueKind::CFrame => {
            return json!({"CFrame": {
                "position": [value[0], value[1], value[2]],
                "orientation": [[value[3], value[4], value[5]], [value[6], value[7], value[8]], [value[9], value[10], value[11]]]
            }});
        }
        ValueKind::NumberRange => "NumberRange",
        ValueKind::Enum(_) => "String",
    };
    json!({ label: value })
}

fn explicit_inner(value: &Value) -> Option<&Value> {
    value.as_object().and_then(|object| {
        if object.len() == 1 {
            object.values().next()
        } else {
            None
        }
    })
}

fn attribute_kind(value: &Value) -> Option<ValueKind> {
    let object = value.as_object()?;
    let (kind, _) = object.iter().next()?;
    if object.len() != 1 {
        return None;
    }
    match kind.as_str() {
        "Bool" => Some(ValueKind::Bool),
        "String" => Some(ValueKind::String),
        "Float64" | "Float32" | "Int32" | "Int64" => Some(ValueKind::Number),
        "BrickColor" => Some(ValueKind::BrickColor),
        "Color3" => Some(ValueKind::Color3),
        "Vector2" => Some(ValueKind::Vector2),
        "Vector3" => Some(ValueKind::Vector3),
        "UDim" => Some(ValueKind::UDim),
        "UDim2" => Some(ValueKind::UDim2),
        "CFrame" => Some(ValueKind::CFrame),
        "NumberRange" => Some(ValueKind::NumberRange),
        "Rect" => Some(ValueKind::Rect),
        _ => None,
    }
}

fn attribute_kinds() -> Vec<(&'static str, ValueKind)> {
    vec![
        ("Boolean", ValueKind::Bool),
        ("String", ValueKind::String),
        ("Number", ValueKind::Number),
        ("BrickColor", ValueKind::BrickColor),
        ("Color3", ValueKind::Color3),
        ("Vector2", ValueKind::Vector2),
        ("Vector3", ValueKind::Vector3),
        ("UDim", ValueKind::UDim),
        ("UDim2", ValueKind::UDim2),
        ("CFrame", ValueKind::CFrame),
        ("NumberRange", ValueKind::NumberRange),
        ("Rect", ValueKind::Rect),
    ]
}

fn parse_setting(kind: SettingKind, text: &str) -> std::result::Result<Option<Value>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value = match kind {
        SettingKind::Text => json!(text),
        SettingKind::Port => {
            let port = text
                .parse::<u16>()
                .map_err(|_| "enter a port from 0 to 65535")?;
            json!(port)
        }
        SettingKind::Integer => {
            let number = text
                .parse::<u64>()
                .map_err(|_| "enter a non-negative integer")?;
            json!(number)
        }
        SettingKind::IntegerList => {
            let values: Result<Vec<_>, _> =
                text.split(',').map(|v| v.trim().parse::<u64>()).collect();
            json!(values.map_err(|_| "enter comma-separated non-negative integers")?)
        }
        SettingKind::TextList => json!(
            text.split(',')
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        ),
        SettingKind::Bool => unreachable!("booleans toggle directly"),
    };
    Ok(Some(value))
}

fn parse_explorer_json(text: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(text).context("invalid JSON")?;
    representable(&value)?;
    rojo::validate_template_structure(&value)?;
    Ok(value)
}

fn select_path(app: &mut App, path: &[String]) {
    if let Some(index) = app.visible_rows().iter().position(|row| row.path == path) {
        app.selected_tree = index;
        app.selected_inspector = 0;
    }
}
fn value_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Array(v)) => v
            .iter()
            .map(|value| value_text(Some(value)))
            .collect::<Vec<_>>()
            .join(", "),
        Some(Value::Object(v)) if v.len() == 1 => value_text(v.values().next()),
        Some(Value::Object(v)) if v.contains_key("position") && v.contains_key("orientation") => {
            format!(
                "{}, {}",
                value_text(v.get("position")),
                value_text(v.get("orientation"))
            )
        }
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
fn setting_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .map(|v| {
                v.as_str()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| v.to_string())
            })
            .collect::<Vec<_>>()
            .join(", "),
        value => value_text(value),
    }
}

fn render(frame: &mut ratatui::Frame<'_>, app: &App) {
    let area = frame.area();
    if is_too_small(area) {
        render_too_small(frame, area);
        if let Some(Modal::Help) = app.modal {
            render_modal(frame, &Modal::Help, area);
        }
        return;
    }
    match app.mode {
        Mode::Explorer => render_explorer(frame, app, area),
        Mode::Json => render_json(frame, app, area),
    }
    if let Some(modal) = &app.modal {
        render_modal(frame, modal, area);
    }
}

fn render_explorer(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let outer = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new("rproj project template")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::BOTTOM)),
        outer[0],
    );
    let panes = responsive_panes(outer[1], 45);
    render_tree(frame, app, panes[0]);
    render_inspector(frame, app, panes[1]);
    render_footer(
        frame,
        outer[2],
        &app.status,
        "Arrows navigate  Enter edit  A add  F2 rename  D duplicate  M move  Del delete  Ctrl+E JSON  Ctrl+S save  Ctrl+R reset  ? help",
        app.status.starts_with("Template was not"),
    );
}

fn render_tree(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let rows = app.visible_rows();
    let items: Vec<_> = rows
        .iter()
        .map(|row| {
            let branch = if row.has_children {
                if app.collapsed.contains(&row.path) {
                    ">"
                } else {
                    "v"
                }
            } else {
                " "
            };
            let lock = if row.locked { " [locked]" } else { "" };
            ListItem::new(format!(
                "{}{} {} ({}){}",
                "  ".repeat(row.depth),
                branch,
                row.name,
                row.class_name,
                lock
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.selected_tree));
    let style = if app.focus == Focus::Tree {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Black).bg(Color::DarkGray)
    };
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(" Explorer ")
                    .borders(Borders::ALL)
                    .border_style(if app.focus == Focus::Tree {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(style),
        area,
        &mut state,
    );
}

fn render_inspector(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let path = app.selected_path();
    let node = app.model().node(&path).expect("selected node exists");
    let incompatible = app.model().incompatible_properties(&path, &app.metadata);
    let items: Vec<_> = app
        .inspector_rows()
        .iter()
        .map(|row| {
            ListItem::new(match row {
                InspectorRow::Class => format!(
                    "Class: {}{}",
                    app.model().class_name(&path),
                    if path.is_empty() || app.visible_rows()[app.selected_tree].locked {
                        " [locked]"
                    } else {
                        ""
                    }
                ),
                InspectorRow::IgnoreUnknown => format!(
                    "Ignore unknown instances: {}",
                    node.get("$ignoreUnknownInstances")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                ),
                InspectorRow::Property(name, kind) => {
                    let value = value_text(node.get("$properties").and_then(|p| p.get(name)));
                    format!(
                        "{}{}: {}",
                        if incompatible.contains(name) {
                            "! "
                        } else {
                            ""
                        },
                        name,
                        if kind.is_some() {
                            value
                        } else {
                            format!("{value} [JSON only]")
                        }
                    )
                }
                InspectorRow::AddProperty => "+ Add property".into(),
                InspectorRow::Attribute(name, kind) => format!(
                    "@ {name}{}",
                    if kind.is_some() { "" } else { " [JSON only]" }
                ),
                InspectorRow::AddAttribute => "+ Add attribute".into(),
                InspectorRow::Setting(key, label, _) => {
                    format!("{label}: {}", setting_text(app.model().value().get(key)))
                }
            })
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.selected_inspector));
    let title = if path.is_empty() {
        " Inspector / project settings ".into()
    } else {
        format!(" Inspector: {} ", path.join("/"))
    };
    let style = if app.focus == Focus::Inspector {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::Black).bg(Color::DarkGray)
    };
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(if app.focus == Focus::Inspector {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    }),
            )
            .highlight_style(style),
        area,
        &mut state,
    );
}

fn render_json(frame: &mut ratatui::Frame<'_>, app: &App, area: Rect) {
    let layout = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(6),
        Constraint::Length(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new("Advanced JSON")
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(Block::default().borders(Borders::BOTTOM)),
        layout[0],
    );
    let (row, column) = app.json.cursor();
    let height = layout[1].height.saturating_sub(2) as usize;
    let scroll = row.saturating_sub(height.saturating_sub(1));
    let lines: Vec<_> = app
        .json
        .lines()
        .iter()
        .enumerate()
        .skip(scroll)
        .take(height)
        .map(|(index, line)| {
            Line::from(vec![
                Span::styled(
                    format!("{:>4} ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(line),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(
            Block::default()
                .title(" default.project.json ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.json_error.is_some() {
                    Color::Red
                } else {
                    Color::Cyan
                })),
        ),
        layout[1],
    );
    if row >= scroll && row - scroll < height {
        let display_column = app.json.lines()[row]
            .chars()
            .take(column)
            .collect::<String>();
        frame.set_cursor_position((
            layout[1].x + 6 + UnicodeWidthStr::width(display_column.as_str()) as u16,
            layout[1].y + 1 + (row - scroll) as u16,
        ));
    }
    let message = app.json_error.as_deref().unwrap_or(&app.status);
    render_footer(
        frame,
        layout[2],
        message,
        "Esc apply/return  Ctrl+S validate and save  Ctrl+Z/Y undo/redo  Ctrl+R restore  ? help",
        app.json_error.is_some(),
    );
}

fn render_modal(frame: &mut ratatui::Frame<'_>, modal: &Modal, area: Rect) {
    let popup = centered(
        area,
        72.min(area.width.saturating_sub(4)),
        match modal {
            Modal::Help => 14,
            Modal::Picker { .. } => 18,
            _ => 9,
        },
    );
    frame.render_widget(Clear, popup);
    match modal {
        Modal::Error { message, scroll } => {
            let areas = Layout::vertical([Constraint::Min(3), Constraint::Length(2)]).split(area);
            let lines = crate::tui::wrap_lines(message, areas[0].width.saturating_sub(2) as usize);
            let max = lines.len().saturating_sub(areas[0].height.saturating_sub(2) as usize).min(u16::MAX as usize) as u16;
            scroll.set(scroll.get().min(max));
            frame.render_widget(Clear, area);
            frame.render_widget(Paragraph::new(lines.join("\n")).scroll((scroll.get(), 0)).block(Block::bordered().title(" Template error ")), areas[0]);
            frame.render_widget(Paragraph::new("Arrows / Page Up / Page Down scroll   Home / End\nEnter or Esc returns to the draft"), areas[1]);
        }
        Modal::Help => frame.render_widget(Paragraph::new("Navigation\n  Up/Down select; Left/Right collapse and expand; Tab changes pane\n\nEditing\n  Enter edits; A adds; F2 renames; D duplicates; M moves; Delete removes\n  Ctrl+E opens Advanced JSON; Ctrl+Z/Y undo and redo\n\nFile\n  Ctrl+S validates and saves; Ctrl+R restores built-in; Esc exits\n\nNo command launches an external editor.").wrap(Wrap { trim: false }).block(Block::default().title(" Help ").borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan))), popup),
        Modal::Confirm { state, .. } => render_confirm(frame, area, state),
        Modal::Input { title, hint, state, .. } => render_input(frame, area, title, hint, state),
        Modal::Picker { title, state, .. } => render_picker(frame, area, title, state),
    }
}

fn render_too_small(frame: &mut ratatui::Frame<'_>, area: Rect) {
    frame.render_widget(Paragraph::new("rproj project template\n\nTerminal is too small. Resize to at least 60 x 16.\n\n? help   Esc exit").alignment(ratatui::layout::Alignment::Center).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow))), area);
}

#[cfg(test)]
#[path = "../../tests/common/mod.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common;

    #[test]
    fn editor_pty_driver() {
        let Some(path) = std::env::var_os("RPROJ_EDITOR_TEST_PATH") else {
            return;
        };
        let path = std::path::PathBuf::from(path);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            serde_json::to_string_pretty(&crate::steps::rojo::builtin_project_template()).unwrap()
        });
        super::run(
            text,
            |value| crate::config::project_template::save_to(value, &path).map(|_| ()),
            || crate::config::project_template::reset_at(&path).map(|_| ()),
        )
        .unwrap();
        println!("Editor returned");
    }

    fn pty_editor(path: &std::path::Path) -> common::Session {
        common::Session::start_program(
            &std::env::current_exe().unwrap(),
            path.parent().unwrap(),
            &[
                "project_editor::app::tests::editor_pty_driver",
                "--exact",
                "--nocapture",
            ],
            &[("RPROJ_EDITOR_TEST_PATH", path.to_str().unwrap())],
        )
    }

    #[test]
    fn pty_repeated_save_stays_open_then_exit_and_reset_return() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        let mut session = pty_editor(&path);
        session.wait_for("rproj project template");
        session.send("\x13");
        session.wait_for("Template saved.");
        let saved = std::fs::read(&path).unwrap();
        session.send("\x13");
        session.send("?");
        session.wait_for("Navigation");
        session.send(common::ESC);
        session.wait_for("rproj project template");
        session.send(common::ESC);
        session.wait_for("Editor returned");
        assert_eq!(session.finish().code, 0);
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        let mut session = pty_editor(&path);
        session.wait_for("rproj project template");
        session.send("\x12");
        session.wait_for("Restore the built-in");
        session.send(common::ENTER);
        session.wait_for("Editor returned");
        assert_eq!(session.finish().code, 0);
        assert!(!path.exists());
    }

    #[test]
    fn pty_malformed_template_repair_saves_in_json_mode() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        std::fs::write(&path, "").unwrap();
        let mut session = pty_editor(&path);
        session.wait_for("Advanced JSON");
        let text =
            serde_json::to_string_pretty(&crate::steps::rojo::builtin_project_template()).unwrap();
        session.send(&format!("\x1b[200~{text}\x1b[201~"));
        session.wait_for("StarterPlayerScripts");
        session.send("\x13");
        session.wait_for("Template saved.");
        assert!(session.text().contains("Advanced JSON"));
        session.send("\x03");
        session.wait_for("Editor returned");
        assert_eq!(session.finish().code, 0);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(&path).unwrap()).unwrap(),
            crate::steps::rojo::builtin_project_template()
        );
    }

    #[test]
    fn json_baseline_and_atomic_failure_keep_the_saved_document() {
        let value = crate::steps::rojo::builtin_project_template();
        let mut app = super::App::from_text(serde_json::to_string_pretty(&value).unwrap());
        app.model_mut().add(&[], "Folder", false).unwrap();
        app.enter_json();
        app.json = super::TextBuffer::new(&serde_json::to_string_pretty(&value).unwrap());
        assert!(
            !app.dirty(),
            "JSON matching disk is clean even if the Explorer checkpoint differs"
        );
        let root = tempfile::tempdir().unwrap();
        let blocked = root.path().join("template.json");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("owned"), "preserve").unwrap();
        let draft = app.model().value().clone();
        assert!(!super::validate_save(
            &mut app,
            draft.clone(),
            &mut |value| crate::config::project_template::save_to(value, &blocked).map(|_| ())
        ));
        assert_eq!(app.model().value(), &draft);
        assert_eq!(
            std::fs::read_to_string(blocked.join("owned")).unwrap(),
            "preserve"
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
    #[test]
    fn compound_attribute_values_round_trip_through_the_input() {
        for (kind, input) in [
            (super::ValueKind::UDim, "0.5, 12"),
            (super::ValueKind::UDim2, "0.5, 12, 1, -24"),
            (super::ValueKind::Rect, "1, 2, 10, 20"),
            (
                super::ValueKind::CFrame,
                "1, 2, 3, 1, 0, 0, 0, 1, 0, 0, 0, 1",
            ),
        ] {
            let value = super::attribute_value(&kind, kind.parse(input).unwrap());
            assert!(kind.accepts(&value), "{value}");
            let text = super::value_text(Some(&value));
            assert_eq!(
                super::attribute_value(&kind, kind.parse(&text).unwrap()),
                value
            );
        }
    }

    #[test]
    #[ignore = "requires a real Rojo binary on PATH"]
    fn guided_compound_values_pass_real_rojo_validation() {
        let mut template = crate::steps::rojo::builtin_project_template();
        let mut attributes = serde_json::Map::new();
        for (name, kind) in [
            ("Position", super::ValueKind::CFrame),
            ("Layout", super::ValueKind::UDim2),
            ("Padding", super::ValueKind::UDim),
            ("Bounds", super::ValueKind::Rect),
        ] {
            attributes.insert(
                name.into(),
                super::attribute_value(&kind, kind.default_value()),
            );
        }
        template["tree"]["Workspace"]["Probe"] = serde_json::json!({
            "$className": "Part", "$properties": {"Attributes": attributes}
        });
        template["tree"]["StarterGui"] = serde_json::json!({
            "$className": "StarterGui", "Screen": {"$className": "ScreenGui",
                "Image": {"$className": "ImageLabel", "$properties": {
                    "Size": super::ValueKind::UDim2.parse("1, 0, 1, 0").unwrap(),
                    "SliceCenter": super::ValueKind::Rect.parse("0, 0, 10, 10").unwrap()
                }, "Layout": {"$className": "UIListLayout", "$properties": {
                    "Padding": super::ValueKind::UDim.parse("0, 8").unwrap()
                }}}
            }
        });
        crate::steps::rojo::validate_template_with_rojo(&template).unwrap();
    }
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn rendered(width: u16, height: u16) -> String {
        let text = serde_json::to_string_pretty(&rojo::builtin_project_template()).unwrap();
        let app = App::from_text(text);
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn wide_and_narrow_layouts_render_the_editor() {
        assert!(rendered(120, 30).contains("Explorer"));
        assert!(rendered(80, 24).contains("Inspector"));
    }

    #[test]
    fn tiny_layout_requests_a_resize() {
        assert!(rendered(40, 10).contains("too small"));
    }

    #[test]
    fn corrupt_json_opens_repair_mode() {
        let app = App::from_text("not json".into());
        assert_eq!(app.mode, Mode::Json);
        assert!(app.json_error.unwrap().contains("invalid JSON"));
    }

    #[test]
    fn protected_damage_also_opens_repair_mode() {
        let mut value = rojo::builtin_project_template();
        value["tree"]["$className"] = json!("Folder");
        let app = App::from_text(serde_json::to_string_pretty(&value).unwrap());
        assert_eq!(app.mode, Mode::Json);
        assert!(app.json_error.unwrap().contains("DataModel"));
    }

    #[test]
    fn applying_valid_json_returns_to_explorer_as_one_draft() {
        let text = serde_json::to_string_pretty(&rojo::builtin_project_template()).unwrap();
        let mut app = App::from_text(text);
        app.enter_json();
        app.json.end();
        app.leave_json();
        assert_eq!(app.mode, Mode::Explorer);
        assert!(app.model.is_some());
    }

    #[test]
    fn unsaved_exit_and_reset_require_confirmation() {
        let text = serde_json::to_string_pretty(&rojo::builtin_project_template()).unwrap();
        let mut app = App::from_text(text);
        app.model_mut().add(&[], "Folder", false).unwrap();

        assert!(handle_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)).is_none());
        assert!(matches!(app.modal, Some(Modal::Confirm { .. })));
        assert!(matches!(
            handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Request::Cancel)
        ));

        assert!(
            handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)
            )
            .is_none()
        );
        assert!(matches!(
            handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Request::Reset)
        ));
    }

    #[test]
    fn failed_save_keeps_the_draft_and_success_returns_the_candidate() {
        let text = serde_json::to_string_pretty(&rojo::builtin_project_template()).unwrap();
        let mut app = App::from_text(text);
        app.model_mut().add(&[], "Folder", false).unwrap();
        let candidate = app.model().value().clone();

        let mut reject = |_: &Value| anyhow::bail!("invalid property");
        assert!(!validate_save(&mut app, candidate.clone(), &mut reject));
        assert!(app.model().value()["tree"].get("Folder").is_some());
        assert!(app.status.contains("invalid property"));

        let mut accept = |_: &Value| Ok(());
        assert!(validate_save(&mut app, candidate.clone(), &mut accept));
        assert!(!app.dirty());
        assert_eq!(app.model().value(), &candidate);
        app.model_mut().undo();
        assert!(app.dirty());
        app.model_mut().redo();
        assert!(!app.dirty());
    }

    #[test]
    fn common_attributes_round_trip_through_explicit_rojo_values() {
        assert_eq!(
            attribute_kind(&json!({"Vector3": [1, 2, 3]})),
            Some(ValueKind::Vector3)
        );
        assert_eq!(
            attribute_value(&ValueKind::Bool, json!(true)),
            json!({"Bool": true})
        );
    }

    #[test]
    fn setting_parsers_distinguish_numbers_and_lists() {
        assert_eq!(
            parse_setting(SettingKind::Port, "34872").unwrap(),
            Some(json!(34872))
        );
        assert_eq!(
            parse_setting(SettingKind::IntegerList, "1, 2").unwrap(),
            Some(json!([1, 2]))
        );
        assert_eq!(
            parse_setting(SettingKind::TextList, "a, b").unwrap(),
            Some(json!(["a", "b"]))
        );
        assert_eq!(parse_setting(SettingKind::Text, "").unwrap(), None);
    }
}
