use std::io::{self, IsTerminal};

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};

use crate::catalog_view::{CatalogDetail, CatalogEntry, CatalogSection, lookup, sections};
use crate::tui::{
    ACCENT, MUTED, TerminalSession, centered, is_too_small, render_footer, responsive_panes,
    selected_style, title_style,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogExit {
    Back,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Sections,
    Entries(usize),
    Detail {
        section: usize,
        entry: Option<usize>,
    },
}

pub struct CatalogApp {
    sections: Vec<CatalogSection>,
    level: Level,
    selected: usize,
    query: String,
    help: bool,
}

impl CatalogApp {
    pub fn new() -> Self {
        Self {
            sections: sections(),
            level: Level::Sections,
            selected: 0,
            query: String::new(),
            help: false,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<CatalogExit> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(CatalogExit::Quit);
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?')) {
                self.help = false;
            }
            return None;
        }
        if key.code == KeyCode::Char('?') {
            self.help = true;
            return None;
        }
        match key.code {
            KeyCode::Esc => return self.back(),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.filtered_len().saturating_sub(1));
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.query.push(character);
                self.selected = 0;
            }
            KeyCode::Enter => self.open_selected(),
            _ => {}
        }
        None
    }

    fn back(&mut self) -> Option<CatalogExit> {
        match self.level {
            Level::Sections => Some(CatalogExit::Back),
            Level::Entries(_) => {
                self.level = Level::Sections;
                self.selected = 0;
                self.query.clear();
                None
            }
            Level::Detail {
                section,
                entry: Some(_),
            } => {
                self.level = Level::Entries(section);
                self.selected = 0;
                self.query.clear();
                None
            }
            Level::Detail { .. } => {
                self.level = Level::Sections;
                self.selected = 0;
                self.query.clear();
                None
            }
        }
    }

    fn filtered_section_indices(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.sections
            .iter()
            .enumerate()
            .filter(|(_, section)| section.label().to_lowercase().contains(&query))
            .map(|(index, _)| index)
            .collect()
    }

    fn filtered_entry_indices(&self, section: usize) -> Vec<usize> {
        let query = self.query.to_lowercase();
        let CatalogSection::Entries { entries, .. } = &self.sections[section] else {
            return Vec::new();
        };
        entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                query.is_empty()
                    || entry.key.to_lowercase().contains(&query)
                    || entry.description.to_lowercase().contains(&query)
                    || entry.badge.to_lowercase().contains(&query)
            })
            .map(|(index, _)| index)
            .collect()
    }

    fn filtered_len(&self) -> usize {
        match self.level {
            Level::Sections => self.filtered_section_indices().len(),
            Level::Entries(section) => self.filtered_entry_indices(section).len(),
            Level::Detail { .. } => 1,
        }
    }

    fn open_selected(&mut self) {
        match self.level {
            Level::Sections => {
                let Some(section) = self.filtered_section_indices().get(self.selected).copied()
                else {
                    return;
                };
                self.query.clear();
                self.selected = 0;
                self.level = match self.sections[section] {
                    CatalogSection::Entries { .. } => Level::Entries(section),
                    CatalogSection::Page { .. } => Level::Detail {
                        section,
                        entry: None,
                    },
                };
            }
            Level::Entries(section) => {
                if let Some(entry) = self
                    .filtered_entry_indices(section)
                    .get(self.selected)
                    .copied()
                {
                    self.level = Level::Detail {
                        section,
                        entry: Some(entry),
                    };
                    self.query.clear();
                    self.selected = 0;
                }
            }
            Level::Detail { .. } => {}
        }
    }

    fn selected_detail(&self) -> Option<CatalogDetail> {
        match self.level {
            Level::Sections => None,
            Level::Entries(section) => {
                let entry = self
                    .filtered_entry_indices(section)
                    .get(self.selected)
                    .copied()?;
                let CatalogSection::Entries { entries, .. } = &self.sections[section] else {
                    return None;
                };
                lookup(&entries[entry].key)
            }
            Level::Detail { section, entry } => match (&self.sections[section], entry) {
                (CatalogSection::Entries { entries, .. }, Some(entry)) => {
                    lookup(&entries[entry].key)
                }
                (CatalogSection::Page { detail, .. }, None) => Some(detail.clone()),
                _ => None,
            },
        }
    }

    pub fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        if is_too_small(area) {
            frame.render_widget(
                Paragraph::new("rproj catalog\n\nTerminal is too small. Resize to at least 60 x 16.\n\n? help   Esc exit")
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
            Paragraph::new("rproj catalog")
                .style(title_style())
                .block(Block::default().borders(Borders::BOTTOM)),
            outer[0],
        );
        match self.level {
            Level::Detail { .. } => self.render_detail(frame, outer[1]),
            _ => {
                let panes = responsive_panes(outer[1], 44);
                self.render_list(frame, panes[0]);
                self.render_preview(frame, panes[1]);
            }
        }
        let status = if self.query.is_empty() {
            "Browse rproj's packages, tools, capabilities, files, and workflows.".into()
        } else {
            format!("Filter: {}", self.query)
        };
        render_footer(
            frame,
            outer[2],
            &status,
            "Type to filter  Arrows navigate  Enter open  Esc back  ? help",
            false,
        );
        if self.help {
            let popup = centered(area, 66, 11);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new("Navigation\n  Up/Down select; Enter opens; Esc goes back\n\nSearch\n  Type anywhere to filter; Backspace removes a character\n\nExit\n  Esc from sections returns; Ctrl+C exits rproj")
                    .wrap(Wrap { trim: false })
                    .block(Block::default().title(" Help ").borders(Borders::ALL).border_style(Style::default().fg(ACCENT))),
                popup,
            );
        }
    }

    fn render_list(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let (title, items): (&str, Vec<ListItem<'_>>) = match self.level {
            Level::Sections => (
                " Sections ",
                self.filtered_section_indices()
                    .iter()
                    .map(|index| ListItem::new(self.sections[*index].label()))
                    .collect(),
            ),
            Level::Entries(section) => {
                let CatalogSection::Entries { label, entries } = &self.sections[section] else {
                    return;
                };
                (
                    label,
                    self.filtered_entry_indices(section)
                        .iter()
                        .map(|index| entry_line(&entries[*index]))
                        .collect(),
                )
            }
            Level::Detail { .. } => return,
        };
        let mut state = ListState::default().with_selected(if items.is_empty() {
            None
        } else {
            Some(self.selected.min(items.len() - 1))
        });
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(ACCENT)),
                )
                .highlight_style(selected_style(true)),
            area,
            &mut state,
        );
    }

    fn render_preview(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let (title, body) = match self.selected_detail() {
            Some(detail) => (format!(" {} ", detail.title), detail.body),
            None => (
                " Catalog ".into(),
                "Choose a section to inspect its entries and documentation.".into(),
            ),
        };
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(MUTED))
                .block(Block::default().title(title).borders(Borders::ALL)),
            area,
        );
    }

    fn render_detail(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let detail = self.selected_detail().expect("detail level has detail");
        frame.render_widget(
            Paragraph::new(detail.body)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .title(format!(" {} ", detail.title))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(ACCENT)),
                ),
            area,
        );
    }
}

fn entry_line(entry: &CatalogEntry) -> ListItem<'_> {
    ListItem::new(format!(
        "{}  {}  [{}]",
        entry.key, entry.description, entry.badge
    ))
}

pub fn run() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!("`rproj info` requires an interactive terminal when no key is supplied");
    }
    let mut terminal = TerminalSession::enter()?;
    let mut app = CatalogApp::new();
    loop {
        terminal.draw(|frame| app.render(frame))?;
        if let Event::Key(key) = terminal.read_event()?
            && key.kind != crossterm::event::KeyEventKind::Release
            && app.handle_key(key).is_some()
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn rendered(app: &CatalogApp, width: u16, height: u16) -> String {
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
    fn wide_narrow_and_tiny_states_render() {
        let app = CatalogApp::new();
        assert!(rendered(&app, 120, 30).contains("Sections"));
        assert!(rendered(&app, 80, 24).contains("Catalog"));
        assert!(rendered(&app, 40, 10).contains("too small"));
    }

    #[test]
    fn filtering_opens_a_detail_and_backtracks() {
        let mut app = CatalogApp::new();
        for character in "Generated".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        for character in "wally.toml".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.level, Level::Detail { .. }));
        assert!(
            app.selected_detail()
                .unwrap()
                .body
                .contains("Wally manifest")
        );
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.level, Level::Entries(_)));
    }
}
