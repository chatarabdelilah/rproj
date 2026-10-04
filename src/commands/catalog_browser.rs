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
        description: String,
        children: Vec<Node>,
    },
    Entry(CatalogEntry),
}

impl Node {
    fn label(&self) -> &str {
        match self {
            Self::Group { label, .. } => label,
            Self::Entry(entry) => &entry.key,
        }
    }
    fn detail(&self) -> Option<CatalogDetail> {
        match self {
            Self::Entry(entry) => lookup(&entry.key),
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
    let (label, description) = label.split_once(" - ").unwrap_or((label, ""));
    Node::Group {
        label: label.into(),
        description: description.into(),
        children,
    }
}

fn tree() -> Vec<Node> {
    sections()
        .into_iter()
        .enumerate()
        .map(|(index, section)| match section {
            CatalogSection::Entries { label, entries } if index == 0 => group(
                &label,
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
            CatalogSection::Entries { label, entries } if index == 1 => {
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
                group(
                    &format!(
                        "Tools - {}",
                        label
                            .split_once(" - ")
                            .map(|(_, detail)| detail)
                            .unwrap_or("")
                    ),
                    children,
                )
            }
            CatalogSection::Entries { label, entries } => {
                group(&label, entries.into_iter().map(Node::Entry).collect())
            }
        })
        .collect()
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
struct Location {
    path: Vec<usize>,
    query: String,
    selected: usize,
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
                if let Some(location) = self.history.pop() {
                    self.location = location;
                } else {
                    return Some(CatalogExit::Back);
                }
            }
            KeyCode::Enter => self.open(),
            KeyCode::PageUp => self.location.scroll = self.location.scroll.saturating_sub(10),
            KeyCode::PageDown => {
                self.location.scroll = self
                    .location
                    .scroll
                    .saturating_add(10)
                    .min(self.detail_limit.get())
            }
            KeyCode::Home | KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.location.scroll = if key.code == KeyCode::Home {
                    0
                } else {
                    self.detail_limit.get()
                };
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End => {
                let end = self.visible().len().saturating_sub(1);
                self.location.selected = match key.code {
                    KeyCode::Home => 0,
                    KeyCode::End => end,
                    KeyCode::Up => self.location.selected.saturating_sub(1),
                    _ => (self.location.selected + 1).min(end),
                };
                self.location.scroll = 0;
            }
            KeyCode::Backspace => {
                self.location.query.pop();
                self.location.selected = 0;
                self.location.scroll = 0;
            }
            KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.location.query.push(ch);
                self.location.selected = 0;
                self.location.scroll = 0;
            }
            _ => {}
        }
        None
    }
    pub fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;
        let amount = 3;
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.location.scroll = self.location.scroll.saturating_sub(amount)
            }
            MouseEventKind::ScrollDown => {
                self.location.scroll = self
                    .location
                    .scroll
                    .saturating_add(amount)
                    .min(self.detail_limit.get())
            }
            _ => {}
        }
    }
    pub fn render(&self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        if self.help {
            frame.render_widget(Paragraph::new("Catalog help\n\nType to filter this group and descendants.\nEnter opens a group. Entries are read-only.\nArrows navigate. Page Up/Down scroll details.\nHome/End select items. Ctrl+Home/End scroll details.\nEsc returns to the previous view. Ctrl+C leaves Catalog.\n\n? or Esc closes Help.").wrap(Wrap { trim: false }).block(Block::bordered().title(" Help ").padding(Padding::uniform(1))), area);
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
            "Type filter  Arrows navigate  Enter group  PgUp/PgDn details  Esc back  ? help",
            false,
        );
    }
    fn render_list(&self, frame: &mut ratatui::Frame<'_>, area: Rect) {
        let paths = self.visible();
        let width = area.width.saturating_sub(4) as usize;
        let items = paths
            .iter()
            .map(|path| {
                ListItem::new(Span::styled(
                    truncate(self.node(path).label(), width),
                    Style::default().add_modifier(Modifier::BOLD),
                ))
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
                        .border_style(Style::default().fg(ACCENT)),
                )
                .highlight_style(tui::selected_style(true)),
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
                    Node::Group {
                        description,
                        children,
                        ..
                    } => format!(
                        "{}\n\n{}",
                        description,
                        children
                            .iter()
                            .map(|node| node.label())
                            .collect::<Vec<_>>()
                            .join("\n")
                    ),
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
                        .border_style(Style::default().fg(MUTED)),
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
    terminal.set_mouse_capture(true)?;
    let mut app = CatalogApp::new();
    loop {
        terminal.draw(|frame| app.render(frame))?;
        match terminal.read_event()? {
            Event::Key(key) if key.kind != crossterm::event::KeyEventKind::Release => {
                if app.handle_key(key).is_some() {
                    return Ok(());
                }
            }
            Event::Mouse(mouse) => app.handle_mouse(mouse),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_use_names_and_enter_on_an_entry_does_not_change_navigation() {
        let mut app = CatalogApp::new();
        assert!(app.roots.iter().all(|node| !node.label().contains(" - ")));
        let capabilities = app
            .roots
            .iter()
            .find(|node| node.label() == "Capabilities")
            .unwrap();
        assert!(
            matches!(capabilities, Node::Group { description, .. } if description.contains("what a project"))
        );
        app.location.query = "promise".into();
        let before = app.location.clone();
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.location, before);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.location, before);
    }
    fn press(app: &mut CatalogApp, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }
    #[test]
    fn grouping_preserves_every_catalog_entry_and_root_search_finds_descendants() {
        let mut app = CatalogApp::new();
        for section in sections() {
            let CatalogSection::Entries { entries, .. } = section;
            {
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
        // Scrolling does not move focus away from the list.
        assert_eq!(app.location.scroll, 10);
        press(&mut app, KeyCode::Esc);

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
        assert_eq!(wrap_lines("abcde fg", 5), ["abcde", "fg"]);
        assert_eq!(wrap_lines("abcde ", 5), ["abcde"]);
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
        app.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL));
        let end = app.location.scroll;
        assert!(end > 0);
        press(&mut app, KeyCode::PageUp);
        assert_eq!(app.location.scroll, end.saturating_sub(10));
        press(&mut app, KeyCode::Esc);

        press(&mut app, KeyCode::Tab);
        assert_eq!(app.location.scroll, end.saturating_sub(10));
    }

    fn resize_and_render(
        app: &CatalogApp,
        terminal: &mut ratatui::Terminal<ratatui::backend::TestBackend>,
        width: u16,
        height: u16,
    ) -> String {
        terminal.backend_mut().resize(width, height);
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
    fn resize_recovery_preserves_search_selection_and_detail_scroll() {
        let mut app = CatalogApp::new();
        for ch in "react".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        let index = app
            .visible()
            .iter()
            .position(|path| app.node(path).label() == "reactRoblox")
            .unwrap();
        assert!(index > 0, "exercise a non-default selection");
        for _ in 0..index {
            press(&mut app, KeyCode::Down);
        }
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).unwrap();
        resize_and_render(&app, &mut terminal, 120, 30);
        press(&mut app, KeyCode::PageDown);
        assert!(app.location.scroll > 0);
        let before = app.location.clone();
        let selected = app.selected_path();
        resize_and_render(&app, &mut terminal, 120, 30);
        let baseline = terminal.backend().buffer().clone();

        for (width, height) in [(80, 24), (60, 16), (40, 10), (120, 30)] {
            let screen = resize_and_render(&app, &mut terminal, width, height);
            assert_eq!(app.location.query, before.query);
            assert_eq!(app.location.selected, before.selected);
            assert_eq!(app.selected_path(), selected);
            assert_eq!(app.location.scroll, before.scroll);
            if width < 60 {
                assert!(screen.contains("Resize to at least 60 x 16."));
                press(&mut app, KeyCode::Char('?'));
                assert!(resize_and_render(&app, &mut terminal, width, height).contains("Help"));
                press(&mut app, KeyCode::Esc);
            } else {
                assert!(screen.contains("Filter: react"));
                assert!(screen.contains(" reactRoblox "));
            }
        }
        let restored = resize_and_render(&app, &mut terminal, 120, 30);
        // The list offset can adjust to keep the selection visible in the smaller viewport.
        let details = tui::responsive_panes(Rect::new(0, 3, 120, 24), 38)[1];
        for y in details.y..details.bottom() {
            for x in details.x..details.right() {
                assert_eq!(terminal.backend().buffer()[(x, y)], baseline[(x, y)]);
            }
        }

        press(&mut app, KeyCode::PageUp);
        assert_eq!(app.location.scroll, before.scroll.saturating_sub(10));
        assert_ne!(resize_and_render(&app, &mut terminal, 120, 30), restored);
        press(&mut app, KeyCode::PageDown);
        assert_eq!(resize_and_render(&app, &mut terminal, 120, 30), restored);
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.location.scroll, before.scroll.saturating_sub(3));
        assert_eq!(app.selected_path(), selected);
    }

    #[test]
    fn resize_recovery_keeps_scrolled_list_selection_visible_and_back_state() {
        let mut app = CatalogApp::new();
        app.roots = vec![group(
            "Resize fixture",
            (0..40)
                .map(|index| group(&format!("Match {index:02}"), vec![]))
                .collect(),
        )];
        press(&mut app, KeyCode::Enter);
        for ch in "Match".chars() {
            press(&mut app, KeyCode::Char(ch));
        }
        press(&mut app, KeyCode::End);
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 30)).unwrap();
        resize_and_render(&app, &mut terminal, 120, 30);
        assert!(app.location.offset.get() > 0);
        let selected = app.selected_path();
        let path = app.location.path.clone();

        for (width, height) in [(80, 24), (60, 16), (40, 10), (120, 30)] {
            let screen = resize_and_render(&app, &mut terminal, width, height);
            assert_eq!(app.location.query, "Match");
            assert_eq!(app.location.path, path);
            assert_eq!(app.location.selected, 39);
            assert_eq!(app.selected_path(), selected);
            assert!(app.location.offset.get() > 0);
            if width >= 60 {
                assert!(
                    screen.contains("Match 39"),
                    "selected row must stay visible"
                );
                assert!(!screen.contains("Match 00"), "list must remain scrolled");
            }
        }

        let before = app.location.clone();
        press(&mut app, KeyCode::Enter);
        resize_and_render(&app, &mut terminal, 40, 10);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.location, before);
        assert!(resize_and_render(&app, &mut terminal, 120, 30).contains("Match 39"));
        press(&mut app, KeyCode::Up);
        assert!(resize_and_render(&app, &mut terminal, 120, 30).contains("Match 38"));
        assert_eq!(app.location.selected, 38);
    }

    #[test]
    fn mouse_wheel_scrolls_the_selected_details() {
        let mut app = CatalogApp::new();
        app.detail_limit.set(20);
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.location.scroll, 3);
        app.handle_mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.location.scroll, 0);
    }
}
