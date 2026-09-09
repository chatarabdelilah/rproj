use crate::catalog::{
    tool_catalog::{self, ToolKind},
    wally_packages,
};
use crate::catalog_view::{CatalogDetail, CatalogEntry, CatalogSection, lookup, sections};
use crate::tui::{self, ACCENT, MUTED, TerminalSession, wrap_lines};
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Padding, Paragraph, Wrap},
};
use std::cell::Cell;
use std::io::{self, IsTerminal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogExit {
    Back,
    Quit,
}

#[derive(Clone, Debug)]
enum Node {
    Group {
        label: String,
        children: Vec<Node>,
    },
    Entry(CatalogEntry),
    Page {
        label: String,
        detail: CatalogDetail,
    },
}

impl Node {
    fn label(&self) -> &str {
        match self {
            Self::Group { label, .. } | Self::Page { label, .. } => label,
            Self::Entry(entry) => &entry.key,
        }
    }
    fn detail(&self) -> Option<CatalogDetail> {
        match self {
            Self::Entry(entry) => lookup(&entry.key),
            Self::Page { detail, .. } => Some(detail.clone()),
            _ => None,
        }
    }
    fn matches(&self, query: &str) -> bool {
        self.label().to_lowercase().contains(query)
            || match self {
                Self::Entry(entry) => {
                    entry.description.to_lowercase().contains(query)
                        || entry.badge.to_lowercase().contains(query)
                }
                _ => false,
            }
    }
}

fn group(label: &str, mut children: Vec<Node>) -> Node {
    children.sort_by_cached_key(|node| node.label().to_lowercase());
    Node::Group {
        label: label.into(),
        children,
    }
}

fn tree() -> Vec<Node> {
    sections()
        .into_iter()
        .enumerate()
        .map(|(index, section)| match section {
            CatalogSection::Entries { entries, .. } if index == 0 => group(
                "Packages",
                wally_packages::Category::ALL
                    .iter()
                    .map(|category| {
                        group(
                            category.label(),
                            entries
                                .iter()
                                .filter(|entry| {
                                    wally_packages::find(&entry.key)
                                        .is_some_and(|p| p.category == *category)
                                })
                                .cloned()
                                .map(Node::Entry)
                                .collect(),
                        )
                    })
                    .collect(),
            ),
            CatalogSection::Entries { entries, .. } if index == 1 => {
                let mut groups = std::collections::BTreeMap::<&str, Vec<Node>>::new();
                for entry in entries {
                    let tool = tool_catalog::find(&entry.key).expect("catalog tool");
                    let label = match tool.kind {
                        ToolKind::SystemApp { .. } => "System Apps",
                        ToolKind::RokitTool { .. } => "CLI Tools",
                        ToolKind::BlenderAddon { .. } => "Blender Add-ons",
                        ToolKind::VsCodeExtension { .. } if entry.key.starts_with("theme-") => {
                            "Themes & Icons"
                        }
                        ToolKind::VsCodeExtension { .. } => "Extensions",
                        _ => "Studio Plugins",
                    };
                    groups.entry(label).or_default().push(Node::Entry(entry));
                }
                let vscode = group(
                    "VS Code",
                    ["Extensions", "Themes & Icons"]
                        .into_iter()
                        .map(|name| group(name, groups.remove(name).unwrap_or_default()))
                        .collect(),
                );
                let mut children: Vec<_> = groups
                    .into_iter()
                    .map(|(name, nodes)| group(name, nodes))
                    .collect();
                children.push(vscode);
                group("Tools", children)
            }
            CatalogSection::Entries { label, entries } => {
                group(&label, entries.into_iter().map(Node::Entry).collect())
            }
            CatalogSection::Page { label, detail } => Node::Page { label, detail },
        })
        .collect()
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Location {
    path: Vec<usize>,
    query: String,
    selected: usize,
    detail_focus: bool,
    scroll: u16,
    offset: Cell<usize>,
}

pub struct CatalogApp {
    roots: Vec<Node>,
    location: Location,
    history: Vec<Location>,
    help: bool,
    detail_limit: Cell<u16>,
}

impl CatalogApp {
    pub fn new() -> Self {
        Self {
            roots: tree(),
            location: Location::default(),
            history: Vec::new(),
            help: false,
            detail_limit: Cell::new(0),
        }
    }
    fn node(&self, path: &[usize]) -> &Node {
        let mut nodes = &self.roots;
        for (depth, index) in path.iter().enumerate() {
            let node = &nodes[*index];
            if depth + 1 == path.len() {
                return node;
            }
            if let Node::Group { children, .. } = node {
                nodes = children;
            }
        }
        unreachable!("paths are constructed from the catalog")
    }
    fn children(&self) -> &[Node] {
        if self.location.path.is_empty() {
            &self.roots
        } else if let Node::Group { children, .. } = self.node(&self.location.path) {
            children
        } else {
            &[]
        }
    }
    fn visible(&self) -> Vec<Vec<usize>> {
        fn collect(nodes: &[Node], parent: &[usize], query: &str, output: &mut Vec<Vec<usize>>) {
            for (index, node) in nodes.iter().enumerate() {
                let mut path = parent.to_vec();
                path.push(index);
                if query.is_empty() || node.matches(query) {
                    output.push(path.clone());
                }
                if !query.is_empty()
                    && let Node::Group { children, .. } = node
                {
                    collect(children, &path, query, output);
                }
            }
        }
        let mut result = Vec::new();
        collect(
            self.children(),
            &self.location.path,
            &self.location.query.to_lowercase(),
            &mut result,
        );
        result
    }
    fn selected_path(&self) -> Option<Vec<usize>> {
        self.visible().get(self.location.selected).cloned()
    }
    fn detail(&self) -> Option<CatalogDetail> {
        self.selected_path()
            .and_then(|path| self.node(&path).detail())
    }
    fn breadcrumb(&self, path: &[usize]) -> String {
        (1..=path.len())
            .map(|n| self.node(&path[..n]).label())
            .collect::<Vec<_>>()
            .join(" / ")
    }
    fn open(&mut self) {
        let Some(path) = self.selected_path() else {
            return;
        };
        crate::diagnostics::event("catalog.open", "selected catalog node; filter omitted");
        crate::diagnostics::event("catalog.choice", self.node(&path).label());
        if matches!(self.node(&path), Node::Group { .. }) {
            self.history.push(self.location.clone());
            self.location = Location {
                path,
                ..Location::default()
            };
        } else {
            self.location.detail_focus = true;
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
            KeyCode::Esc => {
                if self.location.detail_focus {
                    self.location.detail_focus = false;
                } else if let Some(location) = self.history.pop() {
                    self.location = location;
                } else {
                    return Some(CatalogExit::Back);
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                if self.detail().is_some() {
                    self.location.detail_focus = !self.location.detail_focus;
                }
            }
            KeyCode::Enter => self.open(),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => {
                let up = matches!(key.code, KeyCode::Up | KeyCode::PageUp);
                let amount = if matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
                    10
                } else {
                    1
                };
                if self.location.detail_focus {
                    let limit = self.detail_limit.get();
                    self.location.scroll = match key.code {
                        KeyCode::Home => 0,
                        KeyCode::End => limit,
                        _ if up => self.location.scroll.min(limit).saturating_sub(amount),
                        _ => self.location.scroll.saturating_add(amount).min(limit),
                    };
                } else {
                    let end = self.visible().len().saturating_sub(1);
                    self.location.selected = match key.code {
                        KeyCode::Home => 0,
                        KeyCode::End => end,
                        _ if up => self.location.selected.saturating_sub(amount as usize),
                        _ => (self.location.selected + amount as usize).min(end),
                    };
                    self.location.scroll = 0;
                }
            }
            KeyCode::Backspace if !self.location.detail_focus => {
                self.location.query.pop();
                self.location.selected = 0;
                self.location.scroll = 0;
            }
            KeyCode::Char(ch)
                if !self.location.detail_focus
                    && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.location.query.push(ch);
                self.location.selected = 0;
                self.location.scroll = 0;
            }
            _ => {}
        }
        None
    }
    pub fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        if self.help {
            frame.render_widget(Paragraph::new("Catalog help\n\nType to filter this group and descendants.\nEnter opens a group or focuses details.\nTab changes focus. Arrows and Page Up/Down scroll.\nHome/End go to the first/last item or detail line.\nEsc returns to the previous view. Ctrl+C leaves Catalog.\n\n? or Esc closes Help.").wrap(Wrap { trim: false }).block(Block::bordered().title(" Help ").padding(Padding::uniform(1))), area);
            return;
        }
        if tui::is_too_small(area) {
            frame.render_widget(
                Paragraph::new(
                    "rproj catalog\nResize to at least 60 x 16.\n? help  Esc back  Ctrl+C close",
                )
                .wrap(Wrap { trim: false }),
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
            Paragraph::new(format!(
                "rproj catalog\n{}",
                if self.location.path.is_empty() {
                    "Sections".into()
                } else {
                    self.breadcrumb(&self.location.path)
                }
            ))
            .style(tui::title_style()),
            outer[0],
        );
        let panes = tui::responsive_panes(outer[1], 38);
        self.render_list(frame, panes[0]);
        self.render_detail(frame, panes[1]);
        tui::render_footer(
            frame,
            outer[2],
            &format!("Filter: {}", self.location.query),
            "Type to filter  Arrows navigate  Enter open  Tab focus  Esc back  ? help",
            false,
        );
    }
    fn render_list(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let paths = self.visible();
        let width = area.width.saturating_sub(4) as usize;
        let items = paths
            .iter()
            .map(|path| {
                let node = self.node(path);
                let purpose = match node {
                    Node::Entry(entry) => crate::catalog_view::first_sentence(&entry.description),
                    Node::Group { .. } => "Open group",
                    Node::Page { .. } => "Read topic",
                };
                let second = if self.location.query.is_empty() {
                    purpose.into()
                } else {
                    self.breadcrumb(&path[..path.len() - 1])
                };
                ListItem::new(vec![
                    Line::from(Span::styled(
                        truncate(node.label(), width),
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                    Line::from(Span::styled(
                        truncate(&second, width),
                        Style::default().fg(MUTED),
                    )),
                ])
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default()
            .with_offset(self.location.offset.get())
            .with_selected(if paths.is_empty() {
                None
            } else {
                Some(self.location.selected)
            });
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::bordered()
                        .title(if self.location.path.is_empty() {
                            " Sections "
                        } else {
                            " Entries "
                        })
                        .padding(Padding::horizontal(1))
                        .border_style(Style::default().fg(if self.location.detail_focus {
                            MUTED
                        } else {
                            ACCENT
                        })),
                )
                .highlight_style(tui::selected_style(!self.location.detail_focus)),
            area,
            &mut state,
        );
        self.location.offset.set(state.offset());
    }
    fn render_detail(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let detail = self.detail().unwrap_or_else(|| CatalogDetail {
            title: "Overview".into(),
            body: self
                .selected_path()
                .map(|path| match self.node(&path) {
                    Node::Group { children, .. } => children
                        .iter()
                        .map(|node| node.label())
                        .collect::<Vec<_>>()
                        .join("\n"),
                    _ => String::new(),
                })
                .unwrap_or_else(|| "No matches.".into()),
        });
        let lines = wrap_lines(&detail.body, area.width.saturating_sub(4) as usize);
        let max = lines
            .len()
            .saturating_sub(area.height.saturating_sub(4) as usize)
            .min(u16::MAX as usize) as u16;
        self.detail_limit.set(max);
        let text: Vec<Line<'_>> = lines
            .into_iter()
            .map(|line| {
                if line.starts_with("## ") {
                    Line::from(Span::styled(
                        line.trim_start_matches("## ").to_owned(),
                        tui::title_style(),
                    ))
                } else {
                    Line::from(line)
                }
            })
            .collect();
        frame.render_widget(
            Paragraph::new(text)
                .scroll((self.location.scroll.min(max), 0))
                .block(
                    Block::bordered()
                        .title(format!(" {} ", detail.title))
                        .padding(Padding::uniform(1))
                        .border_style(Style::default().fg(if self.location.detail_focus {
                            ACCENT
                        } else {
                            MUTED
                        })),
                ),
            area,
        );
    }
}

fn truncate(text: &str, width: usize) -> String {
    use unicode_width::UnicodeWidthChar;
    if unicode_width::UnicodeWidthStr::width(text) <= width {
        return text.into();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let size = ch.width().unwrap_or(0);
        if used + size >= width {
            break;
        }
        out.push(ch);
        used += size;
    }
    if width > 0 {
        out.push('…');
    }
    out
}

pub fn run() -> Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        anyhow::bail!("Catalog requires an interactive terminal");
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
    fn press(app: &mut CatalogApp, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    #[test]
    fn grouping_preserves_every_catalog_entry_and_root_search_finds_descendants() {
        let mut app = CatalogApp::new();
        for section in sections() {
            if let CatalogSection::Entries { entries, .. } = section {
                for entry in entries {
                    app.location.query = entry.key.clone();
                    assert!(
                        app.visible()
                            .iter()
                            .any(|path| app.node(path).label() == entry.key),
                        "{}",
                        entry.key
                    );
                }
            }
        }
    }
    #[test]
    fn back_restores_filter_selection_and_detail_scroll() {
        let mut app = CatalogApp::new();
        app.location.query = "Tools".into();
        let before = app.location.clone();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.location, before);
        app.location.query = "promise".into();
        app.detail_limit.set(20);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::PageDown);
        assert!(app.location.detail_focus);
        assert_eq!(app.location.scroll, 10);
        press(&mut app, KeyCode::Esc);
        assert!(!app.location.detail_focus);
        assert_eq!(app.location.query, "promise");
    }
    #[test]
    fn layouts_help_search_and_long_details_render() {
        for (width, height) in [(120, 30), (280, 70), (80, 24), (40, 10)] {
            let mut app = CatalogApp::new();
            app.location.query = "react".into();
            let mut terminal =
                ratatui::Terminal::new(ratatui::backend::TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| app.render(frame)).unwrap();
            app.help = true;
            terminal.draw(|frame| app.render(frame)).unwrap();
            let text: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect();
            assert!(text.contains("Help") || text.contains("help"));
        }
        assert_eq!(truncate("世界abc", 5), "世界…");
        assert_eq!(wrap_lines("  abc\nxyz", 4), ["  ab", "c", "xyz"]);
    }

    #[test]
    fn grouped_entries_are_sorted_and_end_scroll_can_move_back_up() {
        fn check(nodes: &[Node]) {
            for node in nodes {
                if let Node::Group { children, .. } = node {
                    let labels: Vec<_> =
                        children.iter().map(|n| n.label().to_lowercase()).collect();
                    assert!(labels.windows(2).all(|pair| pair[0] <= pair[1]));
                    check(children);
                }
            }
        }
        let mut app = CatalogApp::new();
        check(&app.roots);
        app.location.query = "reactRoblox".into();
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| app.render(frame)).unwrap();
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::End);
        let end = app.location.scroll;
        assert!(end > 0);
        press(&mut app, KeyCode::Up);
        assert_eq!(app.location.scroll, end - 1);
        press(&mut app, KeyCode::Esc);
        assert!(!app.location.detail_focus);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.location.scroll, end - 1);
    }
}
