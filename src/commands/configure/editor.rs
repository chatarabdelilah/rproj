use super::*;
use crate::tui::{self, ConfirmState, InputState, PickerItem, PickerState, TerminalSession};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};

enum Modal {
    Choice(PickerState<Value>),
    Integer(InputState),
    Save,
    Discard,
}

struct Editor {
    project: std::path::PathBuf,
    tool: Option<&'static ConfigurableTool>,
    tools: PickerState<usize>,
    current: Vec<Option<Value>>,
    changes: Vec<Option<Value>>,
    baseline: Option<String>,
    selected: usize,
    modal: Option<Modal>,
    status: String,
    scroll: u16,
}

impl Editor {
    fn new(project: &Path) -> Self {
        Self {
            project: project.to_owned(),
            tool: None,
            tools: PickerState::new(
                CONFIGURABLE_TOOLS
                    .iter()
                    .enumerate()
                    .map(|(index, tool)| PickerItem {
                        label: tool.display_name.into(),
                        detail: tool.summary.into(),
                        value: index,
                    })
                    .collect(),
            ),
            current: vec![],
            changes: vec![],
            baseline: None,
            selected: 0,
            modal: None,
            status: String::new(),
            scroll: 0,
        }
    }

    fn path(&self) -> std::path::PathBuf {
        self.project
            .join(target_description(&self.tool.unwrap().target))
    }

    fn load(&mut self, index: usize) -> Result<()> {
        let tool = &CONFIGURABLE_TOOLS[index];
        let current = current_values(&self.project, tool)?;
        let path = self.project.join(target_description(&tool.target));
        let baseline = if path.exists() {
            Some(fs::read_to_string(path)?)
        } else {
            None
        };
        self.changes = vec![None; current.len()];
        self.current = current;
        self.baseline = baseline;
        self.tool = Some(tool);
        self.selected = 0;
        self.scroll = 0;
        self.status.clear();
        Ok(())
    }

    fn dirty(&self) -> bool {
        self.changes.iter().any(Option::is_some)
    }

    fn value(&self, index: usize) -> Option<&Value> {
        self.changes[index]
            .as_ref()
            .or(self.current[index].as_ref())
    }

    fn edit(&mut self) {
        let setting = &self.tool.unwrap().settings[self.selected];
        let current = self.value(self.selected);
        self.modal = Some(match &setting.kind {
            SettingKind::Integer { default } => Modal::Integer(InputState::new(
                current
                    .and_then(Value::as_i64)
                    .unwrap_or(*default)
                    .to_string(),
            )),
            kind => {
                let (values, default): (Vec<PickerItem<Value>>, Value) = match kind {
                    SettingKind::Bool { default } => (
                        vec![false, true]
                            .into_iter()
                            .map(|value| PickerItem {
                                label: value.to_string(),
                                detail: String::new(),
                                value: json!(value),
                            })
                            .collect(),
                        json!(default),
                    ),
                    SettingKind::Choice { default, options } => (
                        options
                            .iter()
                            .map(|option| PickerItem {
                                label: option.value.into(),
                                detail: option.explanation.into(),
                                value: json!(option.value),
                            })
                            .collect(),
                        json!(default),
                    ),
                    _ => unreachable!(),
                };
                let mut picker = PickerState::new(values);
                picker.selected = picker
                    .items
                    .iter()
                    .position(|item| Some(&item.value) == current)
                    .or_else(|| picker.items.iter().position(|item| item.value == default))
                    .unwrap_or(0);
                Modal::Choice(picker)
            }
        });
    }

    fn set(&mut self, value: Value) {
        self.changes[self.selected] =
            (self.current[self.selected].as_ref() != Some(&value)).then_some(value);
    }

    fn save(&mut self) -> Result<()> {
        ensure!(
            self.project.is_dir(),
            "The project directory no longer exists."
        );
        let tool = self.tool.context("No tool selected")?;
        let path = self.path();
        let latest = if path.exists() {
            Some(fs::read_to_string(&path)?)
        } else {
            None
        };
        ensure!(
            latest == self.baseline,
            "The file changed outside this editor. Go back and reopen it before saving."
        );
        let answers: Vec<_> = tool
            .settings
            .iter()
            .zip(&self.changes)
            .filter_map(|(setting, value)| value.clone().map(|value| (setting, value)))
            .collect();
        if answers.is_empty() {
            self.status = "No settings changed.".into();
            return Ok(());
        }
        let merged = match tool.target {
            ConfigTarget::ProjectToml { .. } => {
                checked_toml_merge(latest.as_deref().unwrap_or(""), &answers)?
            }
            ConfigTarget::VsCodeSettings => vscode::merged_settings(
                &self.project,
                &answers
                    .iter()
                    .map(|(setting, value)| (setting.key, value.clone()))
                    .collect::<Vec<_>>(),
            )?,
        };
        let parent = path.parent().context("Settings file has no parent")?;
        fs::create_dir_all(parent)?;
        let mut pending = tempfile::NamedTempFile::new_in(parent)?;
        use std::io::Write;
        pending.write_all(merged.as_bytes())?;
        pending.as_file().sync_all()?;
        pending.persist(&path).map_err(|error| error.error)?;
        for (current, change) in self.current.iter_mut().zip(&mut self.changes) {
            if let Some(value) = change.take() {
                *current = Some(value);
            }
        }
        self.baseline = Some(merged);
        self.status = format!("Saved {}", path.display());
        Ok(())
    }

    fn key(&mut self, key: KeyEvent) -> bool {
        let key = if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
        {
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
        } else {
            key
        };
        if let Some(modal) = self.modal.take() {
            if key.code == KeyCode::Esc {
                return false;
            }
            match modal {
                Modal::Save | Modal::Discard
                    if matches!(
                        key.code,
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter
                    ) => {}
                Modal::Save if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                    if let Err(error) = self.save() {
                        self.status = format!("Save failed: {error:#}");
                    }
                }
                Modal::Discard if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                    self.tool = None;
                    self.changes.clear();
                    self.status = "Changes discarded.".into();
                }
                Modal::Choice(picker) if key.code == KeyCode::Enter => {
                    if let Some(value) = picker.selected_value() {
                        self.set(value.clone());
                    }
                }
                Modal::Choice(mut picker) => {
                    picker.handle_key(key);
                    self.modal = Some(Modal::Choice(picker));
                }
                Modal::Integer(mut input) => {
                    if key.code == KeyCode::Enter {
                        match input.text().parse::<i64>() {
                            Ok(value) => {
                                self.set(json!(value));
                                return false;
                            }
                            Err(_) => {
                                input.error = Some(
                                    "Enter a whole number within the signed 64-bit range.".into(),
                                )
                            }
                        }
                    } else {
                        input.handle_key(key);
                    }
                    self.modal = Some(Modal::Integer(input));
                }
                modal => self.modal = Some(modal),
            }
            return false;
        }
        if key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            if self.dirty() {
                self.modal = Some(Modal::Discard);
            } else if self.tool.is_some() {
                self.tool = None;
            } else {
                return true;
            }
            return false;
        }
        if self.tool.is_none() {
            if key.code == KeyCode::Enter {
                if let Some(index) = self.tools.selected_value().copied()
                    && let Err(error) = self.load(index)
                {
                    self.status = format!("Cannot edit: {error:#}");
                }
            } else {
                self.tools.handle_key(key);
            }
        } else {
            let end = self.current.len();
            match key.code {
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(8),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(8),
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down => self.selected = (self.selected + 1).min(end),
                KeyCode::Home => self.selected = 0,
                KeyCode::End => self.selected = end,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.modal = Some(Modal::Save)
                }
                KeyCode::Enter if self.selected == end => self.modal = Some(Modal::Save),
                KeyCode::Enter => self.edit(),
                _ => {}
            }
        }
        false
    }

    fn review(&self) -> String {
        let Some(tool) = self.tool else {
            return String::new();
        };
        tool.settings
            .iter()
            .enumerate()
            .filter_map(|(index, setting)| {
                self.changes[index].as_ref().map(|value| {
                    format!(
                        "{}: {} -> {}",
                        setting.display_key(),
                        self.current[index]
                            .as_ref()
                            .map(Value::to_string)
                            .unwrap_or_else(|| "unset".into()),
                        value
                    )
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw(&self, frame: &mut Frame<'_>) {
        let area = frame.area();
        if tui::is_too_small(area) {
            frame.render_widget(
                Paragraph::new("Configure Tools\nResize to at least 60x16\nEsc Back"),
                area,
            );
            return;
        }
        let rows = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
        frame.render_widget(
            Paragraph::new(format!("Configure Tools — {}", self.project.display())),
            rows[0],
        );
        let panes = tui::responsive_panes(rows[1], 42);
        if let Some(tool) = self.tool {
            let mut items: Vec<_> = tool
                .settings
                .iter()
                .enumerate()
                .map(|(index, setting)| {
                    ListItem::new(format!(
                        "{}{} = {}",
                        if self.changes[index].is_some() {
                            "* "
                        } else {
                            ""
                        },
                        setting.display_key(),
                        self.value(index)
                            .map(Value::to_string)
                            .unwrap_or_else(|| "not set".into())
                    ))
                })
                .collect();
            items.push(ListItem::new("Save changes"));
            frame.render_stateful_widget(
                List::new(items)
                    .block(Block::bordered().title(tool.display_name))
                    .highlight_style(tui::selected_style(true)),
                panes[0],
                &mut ListState::default().with_selected(Some(self.selected)),
            );
            let description = tool
                .settings
                .get(self.selected)
                .map(|setting| format!("{}\n\n{}", setting.display_key(), setting.description))
                .unwrap_or_else(|| "Review changes before saving.".into());
            frame.render_widget(
                Paragraph::new(format!(
                    "{description}\n\n{}\n\nPending changes\n{}",
                    tool.docs_url,
                    self.review()
                ))
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(Block::bordered().title("Details")),
                panes[1],
            );
        } else {
            tui::render_picker(frame, panes[0], "Tools", &self.tools);
            let detail = self
                .tools
                .selected_value()
                .map(|index| CONFIGURABLE_TOOLS[*index].summary)
                .unwrap_or("No matches");
            frame.render_widget(
                Paragraph::new(detail)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title("Overview")),
                panes[1],
            );
        }
        tui::render_footer(
            frame,
            rows[2],
            &self.status,
            "Arrows select  Enter edit  PgUp/PgDn details  Ctrl+S save  Esc Back",
            self.status.contains("failed") || self.status.contains("Cannot"),
        );
        match &self.modal {
            Some(Modal::Choice(picker)) => tui::render_picker(
                frame,
                area,
                "Choose replacement (Esc keeps current)",
                picker,
            ),
            Some(Modal::Integer(input)) => tui::render_input(
                frame,
                area,
                "Value (Esc keeps current)",
                "Enter accepts",
                input,
            ),
            Some(Modal::Save) => tui::render_confirm_default_no(
                frame,
                area,
                &ConfirmState::new(format!(
                    "Save these changes? ({} settings)",
                    self.changes.iter().flatten().count()
                )),
            ),
            Some(Modal::Discard) => tui::render_confirm_default_no(
                frame,
                area,
                &ConfirmState::new("Discard unsaved changes?"),
            ),
            None => {}
        }
    }
}

pub(crate) fn open_in(terminal: &mut TerminalSession, project: &Path) -> Result<()> {
    let context = crate::commands::projects::ProjectContext::load(project.to_owned());
    if let Err(reason) = context.availability(crate::commands::projects::ProjectAction::Configure) {
        anyhow::bail!("{}: {reason}", project.display());
    }
    let mut editor = Editor::new(project);
    loop {
        let mut small = false;
        terminal.draw(|frame| {
            small = tui::is_too_small(frame.area());
            editor.draw(frame);
        })?;
        if let Event::Key(key) = terminal.read_event()?
            && key.kind != KeyEventKind::Release
            && (!small || key.code == KeyCode::Esc)
            && editor.key(key)
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn stylua(dir: &TempDir, contents: &str) -> Editor {
        fs::write(dir.path().join("stylua.toml"), contents).unwrap();
        let mut editor = Editor::new(dir.path());
        editor
            .load(
                CONFIGURABLE_TOOLS
                    .iter()
                    .position(|tool| tool.key == "stylua")
                    .unwrap(),
            )
            .unwrap();
        editor
    }

    fn setting(editor: &mut Editor, key: &str) {
        editor.selected = editor
            .tool
            .unwrap()
            .settings
            .iter()
            .position(|setting| setting.key == key)
            .unwrap();
    }

    fn press(editor: &mut Editor, code: KeyCode) {
        editor.key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn save_changes_only_the_selected_setting_and_preserves_unknown_values() {
        let dir = TempDir::new().unwrap();
        let mut editor = stylua(
            &dir,
            "column_width = 91\nquote_style = 'FutureStyle'\ncustom = ['keep']\n",
        );
        setting(&mut editor, "column_width");
        editor.set(json!(100));
        editor.save().unwrap();
        let saved = fs::read_to_string(dir.path().join("stylua.toml")).unwrap();
        let table: toml::Value = toml::from_str(&saved).unwrap();
        assert_eq!(table["column_width"].as_integer(), Some(100));
        assert_eq!(table["quote_style"].as_str(), Some("FutureStyle"));
        assert_eq!(table["custom"][0].as_str(), Some("keep"));
        assert!(!editor.dirty());
        editor.save().unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("stylua.toml")).unwrap(),
            saved
        );
    }

    #[test]
    fn cancelled_edits_never_write_and_confirmation_defaults_to_no() {
        let dir = TempDir::new().unwrap();
        let original = "# keep formatting\ncolumn_width=91\nquote_style=['unsupported']\n";
        let mut editor = stylua(&dir, original);
        setting(&mut editor, "quote_style");
        press(&mut editor, KeyCode::Enter);
        press(&mut editor, KeyCode::Esc);
        editor.save().unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("stylua.toml")).unwrap(),
            original
        );
        setting(&mut editor, "column_width");
        editor.set(json!(100));
        editor.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        press(&mut editor, KeyCode::Enter);
        assert!(editor.dirty());
        assert_eq!(
            fs::read_to_string(dir.path().join("stylua.toml")).unwrap(),
            original
        );
        press(&mut editor, KeyCode::Esc);
        press(&mut editor, KeyCode::Enter);
        assert!(editor.dirty());
        press(&mut editor, KeyCode::Esc);
        press(&mut editor, KeyCode::Char('y'));
        assert!(!editor.dirty());
        assert!(editor.tool.is_none());
    }

    #[test]
    fn concurrent_edits_are_not_overwritten() {
        let dir = TempDir::new().unwrap();
        let mut editor = stylua(&dir, "column_width=91\n");
        setting(&mut editor, "column_width");
        editor.set(json!(100));
        fs::write(dir.path().join("stylua.toml"), "column_width=120\n").unwrap();
        assert!(editor.save().unwrap_err().to_string().contains("outside"));
        assert!(editor.dirty());
        assert_eq!(
            fs::read_to_string(dir.path().join("stylua.toml")).unwrap(),
            "column_width=120\n"
        );
    }

    #[test]
    fn json_settings_preserve_unrelated_structured_values() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".vscode")).unwrap();
        let path = dir.path().join(".vscode/settings.json");
        fs::write(
            &path,
            r#"{"editor.formatOnSave":false,"custom":{"items":[1,2]}}"#,
        )
        .unwrap();
        let mut editor = Editor::new(dir.path());
        editor
            .load(
                CONFIGURABLE_TOOLS
                    .iter()
                    .position(|tool| tool.key == "stylua-vscode")
                    .unwrap(),
            )
            .unwrap();
        setting(&mut editor, "editor.formatOnSave");
        editor.set(json!(true));
        editor.save().unwrap();
        let saved: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(saved["editor.formatOnSave"], true);
        assert_eq!(saved["custom"], json!({"items":[1,2]}));
    }

    #[test]
    fn malformed_configuration_is_refused_without_entering_editor() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("stylua.toml"), "[broken").unwrap();
        let mut editor = Editor::new(dir.path());
        assert!(
            editor
                .load(
                    CONFIGURABLE_TOOLS
                        .iter()
                        .position(|tool| tool.key == "stylua")
                        .unwrap()
                )
                .is_err()
        );
        assert!(editor.tool.is_none());
    }

    #[test]
    fn editor_and_modals_render_at_supported_sizes() {
        let dir = TempDir::new().unwrap();
        for (width, height) in [(120, 30), (80, 24), (60, 16), (40, 10)] {
            let mut editor = stylua(&dir, "column_width=91\n");
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| editor.draw(frame)).unwrap();
            setting(&mut editor, "column_width");
            editor.edit();
            terminal.draw(|frame| editor.draw(frame)).unwrap();
            editor.modal = Some(Modal::Save);
            terminal.draw(|frame| editor.draw(frame)).unwrap();
            editor.modal = None;
            editor.tool = None;
            terminal.draw(|frame| editor.draw(frame)).unwrap();
        }
    }
}
