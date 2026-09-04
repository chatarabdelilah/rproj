use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::{ACCENT, ERROR, MUTED, WARNING, centered};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputState {
    text: String,
    cursor: usize,
    pub error: Option<String>,
}

impl InputState {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let cursor = text.chars().count();
        Self {
            text,
            cursor,
            error: None,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(self.text.chars().count()),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = self.text.chars().count(),
            KeyCode::Backspace if self.cursor > 0 => {
                let start = byte_index(&self.text, self.cursor - 1);
                let end = byte_index(&self.text, self.cursor);
                self.text.replace_range(start..end, "");
                self.cursor -= 1;
                self.error = None;
            }
            KeyCode::Delete if self.cursor < self.text.chars().count() => {
                let start = byte_index(&self.text, self.cursor);
                let end = byte_index(&self.text, self.cursor + 1);
                self.text.replace_range(start..end, "");
                self.error = None;
            }
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                let index = byte_index(&self.text, self.cursor);
                self.text.insert(index, character);
                self.cursor += 1;
                self.error = None;
            }
            _ => return false,
        }
        true
    }
}

fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}

#[derive(Clone, Debug)]
pub struct PickerItem<T> {
    pub label: String,
    pub detail: String,
    pub value: T,
}

#[derive(Clone, Debug)]
pub struct PickerState<T> {
    pub items: Vec<PickerItem<T>>,
    pub query: InputState,
    pub selected: usize,
}

impl<T> PickerState<T> {
    pub fn new(items: Vec<PickerItem<T>>) -> Self {
        Self {
            items,
            query: InputState::new(""),
            selected: 0,
        }
    }

    pub fn filtered(&self) -> Vec<&PickerItem<T>> {
        let query = self.query.text().to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.label.to_lowercase().contains(&query)
                    || item.detail.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn selected_value(&self) -> Option<&T> {
        self.filtered().get(self.selected).map(|item| &item.value)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.filtered().len().saturating_sub(1));
            }
            _ if self.query.handle_key(key) => self.selected = 0,
            _ => return false,
        }
        true
    }
}

#[derive(Clone, Debug)]
pub struct ConfirmState {
    pub prompt: String,
}

impl ConfirmState {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }
}

pub fn render_footer(frame: &mut Frame<'_>, area: Rect, status: &str, keys: &str, error: bool) {
    frame.render_widget(
        Paragraph::new(format!("{status}\n{keys}"))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(if error { ERROR } else { MUTED })),
        area,
    );
}

pub fn render_input(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    hint: &str,
    state: &InputState,
) {
    let popup = centered(area, 72.min(area.width.saturating_sub(4)), 9);
    frame.render_widget(Clear, popup);
    let body = format!(
        "{hint}\n\n> {}\n{}",
        state.text(),
        state
            .error
            .as_deref()
            .unwrap_or("Enter confirms; Esc cancels")
    );
    frame.render_widget(
        Paragraph::new(body).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if state.error.is_some() {
                    ERROR
                } else {
                    ACCENT
                })),
        ),
        popup,
    );
    let prefix: String = state.text().chars().take(state.cursor()).collect();
    frame.set_cursor_position((
        (popup.x + 3 + UnicodeWidthStr::width(prefix.as_str()) as u16)
            .min(popup.right().saturating_sub(2)),
        popup.y + 3,
    ));
}

pub fn render_picker<T>(frame: &mut Frame<'_>, area: Rect, title: &str, state: &PickerState<T>) {
    let popup = centered(area, 72.min(area.width.saturating_sub(4)), 18);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT)),
        popup,
    );
    let inner = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .split(popup);
    frame.render_widget(
        Paragraph::new(format!(" Search: {}", state.query.text())),
        inner[0],
    );
    let filtered = state.filtered();
    let start = state.selected.saturating_sub(11);
    let items: Vec<_> = filtered
        .iter()
        .skip(start)
        .take(12)
        .map(|item| ListItem::new(format!("{}  {}", item.label, item.detail)))
        .collect();
    let mut list_state = ListState::default().with_selected(if filtered.is_empty() {
        None
    } else {
        Some(state.selected.min(filtered.len() - 1).saturating_sub(start))
    });
    frame.render_stateful_widget(
        List::new(items).highlight_style(Style::default().fg(Color::Black).bg(ACCENT)),
        inner[1],
        &mut list_state,
    );
    frame.render_widget(
        Paragraph::new("Type to filter; Enter selects; Esc cancels"),
        inner[2],
    );
}

pub fn render_confirm(frame: &mut Frame<'_>, area: Rect, state: &ConfirmState) {
    let popup = centered(area, 72.min(area.width.saturating_sub(4)), 9);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "{}\n\nEnter/Y confirm   Esc/N cancel",
            state.prompt
        ))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(WARNING)),
        ),
        popup,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_edits_unicode_by_character() {
        let mut input = InputState::new("aé");
        input.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        input.handle_key(KeyEvent::new(KeyCode::Char('中'), KeyModifiers::NONE));
        assert_eq!(input.text(), "a中é");
        input.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(input.text(), "aé");
    }

    #[test]
    fn picker_filters_labels_and_details_and_clamps_selection() {
        let mut picker = PickerState::new(vec![
            PickerItem {
                label: "Rojo".into(),
                detail: "tool".into(),
                value: 1,
            },
            PickerItem {
                label: "Asphalt".into(),
                detail: "assets".into(),
                value: 2,
            },
        ]);
        picker.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        picker.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(picker.selected_value(), Some(&2));
        picker.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(picker.selected, 0);
    }
}
