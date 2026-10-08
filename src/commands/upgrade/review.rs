use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{UpgradePlan, diff};
use crate::tui::{self, ACCENT, ConfirmState, TerminalSession};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    Apply,
    Cancel { home: bool },
}

#[derive(Default)]
struct Scroll {
    vertical: usize,
    horizontal: usize,
}

struct Review<'a> {
    project: &'a Path,
    plan: &'a UpgradePlan,
    documents: Vec<Vec<diff::Row>>,
    scrolls: Vec<Scroll>,
    selected: usize,
    detail: bool,
    help: bool,
    confirm: bool,
    small: bool,
    height: usize,
}

impl<'a> Review<'a> {
    fn new(project: &'a Path, plan: &'a UpgradePlan) -> Self {
        let mut documents: Vec<_> = plan
            .rewrites
            .iter()
            .map(|rewrite| {
                let mut rows = vec![
                    diff::Row::meta(rewrite.reason),
                    diff::Row::meta("LF unless marked [CRLF]/[no newline]; tabs shown as \\t"),
                    diff::Row::meta(""),
                ];
                rows.extend(diff::rows(
                    plan.originals[&rewrite.relative].as_deref(),
                    &rewrite.contents,
                    &rewrite.relative,
                ));
                rows
            })
            .collect();
        if !plan.skipped.is_empty() {
            documents.push(
                plan.skipped
                    .iter()
                    .map(|note| diff::Row::meta(*note))
                    .collect(),
            );
        }
        let scrolls = documents.iter().map(|_| Scroll::default()).collect();
        Self {
            project,
            plan,
            documents,
            scrolls,
            selected: 0,
            detail: false,
            help: false,
            confirm: false,
            small: false,
            height: 1,
        }
    }

    fn key(&mut self, key: KeyEvent) -> Option<Decision> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(Decision::Cancel { home: true });
        }
        if self.help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                self.help = false;
            }
            return None;
        }
        if key.code == KeyCode::Char('?') {
            self.help = true;
            return None;
        }
        if self.confirm {
            match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('n' | 'N') => self.confirm = false,
                KeyCode::Char('y' | 'Y') if !self.small => return Some(Decision::Apply),
                _ => {}
            }
            return None;
        }
        if key.code == KeyCode::Esc {
            return Some(Decision::Cancel { home: false });
        }
        if self.small {
            return None;
        }
        let scroll = &mut self.scrolls[self.selected];
        let last = self.documents[self.selected].len().saturating_sub(1);
        let page = self.height.max(1);
        match key.code {
            KeyCode::Char('a' | 'A') => {
                self.confirm = true;
                crate::diagnostics::event("prompt", "Apply upgrade changes?");
            }
            KeyCode::Tab | KeyCode::BackTab => self.detail = !self.detail,
            KeyCode::Enter => self.detail = true,
            KeyCode::Up if !self.detail => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down if !self.detail => {
                self.selected = (self.selected + 1).min(self.documents.len() - 1)
            }
            KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) || self.detail => {
                scroll.vertical = 0
            }
            KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) || self.detail => {
                scroll.vertical = last.saturating_sub(page - 1)
            }
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = self.documents.len() - 1,
            KeyCode::Up => scroll.vertical = scroll.vertical.saturating_sub(1),
            KeyCode::Down => scroll.vertical = (scroll.vertical + 1).min(last),
            KeyCode::PageUp => scroll.vertical = scroll.vertical.saturating_sub(page),
            KeyCode::PageDown => scroll.vertical = scroll.vertical.saturating_add(page).min(last),
            KeyCode::Left => scroll.horizontal = scroll.horizontal.saturating_sub(8),
            KeyCode::Right => {
                let widest = self.documents[self.selected]
                    .iter()
                    .map(|row| row.text.width())
                    .max()
                    .unwrap_or(0);
                scroll.horizontal = scroll
                    .horizontal
                    .saturating_add(8)
                    .min(widest.saturating_sub(1));
            }
            _ => {}
        }
        None
    }

    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let area = frame.area();
        self.small = tui::is_too_small(area);
        if self.small {
            frame.render_widget(Paragraph::new("Upgrade review\n\nResize to at least 60 x 16 to review or apply.\n\n? help   Esc cancel   Ctrl+C exit")
                .wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL)), area);
        } else {
            let outer = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);
            frame.render_widget(
                Paragraph::new(format!(
                    "Upgrade review — {}\n{} file changes; review first, then Apply all.",
                    diff::visible(&self.project.display().to_string()),
                    self.plan.rewrites.len()
                ))
                .block(Block::default().borders(Borders::BOTTOM)),
                outer[0],
            );
            let panes = tui::responsive_panes(outer[1], 35);
            let items = self
                .plan
                .rewrites
                .iter()
                .map(|rewrite| {
                    ListItem::new(format!(
                        "{} {}",
                        if rewrite.creating { "create" } else { "update" },
                        rewrite.relative
                    ))
                })
                .chain((!self.plan.skipped.is_empty()).then(|| {
                    ListItem::new(format!("Skipped files ({})", self.plan.skipped.len()))
                }));
            let mut state = ListState::default().with_selected(Some(self.selected));
            frame.render_stateful_widget(
                List::new(items)
                    .block(
                        Block::default()
                            .title(" Files ")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(if self.detail {
                                tui::MUTED
                            } else {
                                ACCENT
                            })),
                    )
                    .highlight_style(tui::selected_style(!self.detail)),
                panes[0],
                &mut state,
            );
            self.height = usize::from(panes[1].height.saturating_sub(2)).max(1);
            let scroll = &self.scrolls[self.selected];
            let lines: Vec<_> = self.documents[self.selected]
                .iter()
                .skip(scroll.vertical)
                .take(self.height)
                .map(|row| {
                    let color = match row.kind {
                        diff::Kind::Added => Color::Green,
                        diff::Kind::Removed => Color::Red,
                        diff::Kind::Context => Color::White,
                        diff::Kind::Meta => tui::WARNING,
                    };
                    Line::styled(
                        crop(
                            &row.text,
                            scroll.horizontal,
                            usize::from(panes[1].width.saturating_sub(2)),
                        ),
                        Style::default().fg(color),
                    )
                })
                .collect();
            let title = self
                .plan
                .rewrites
                .get(self.selected)
                .map(|rewrite| rewrite.relative.as_str())
                .unwrap_or("Skipped files");
            frame.render_widget(
                Paragraph::new(lines).block(
                    Block::default()
                        .title(format!(" {title} — diff "))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(if self.detail {
                            ACCENT
                        } else {
                            tui::MUTED
                        })),
                ),
                panes[1],
            );
            tui::render_footer(
                frame,
                outer[2],
                &format!(
                    "Item {}/{} • diff row {}/{} • column {}",
                    self.selected + 1,
                    self.documents.len(),
                    scroll.vertical + 1,
                    self.documents[self.selected].len(),
                    scroll.horizontal + 1
                ),
                "Tab focus  PgUp/PgDn scroll  ←/→ pan  A Apply all  Esc cancel  ? help",
                false,
            );
        }
        if self.confirm {
            tui::render_confirm_default_no(
                frame,
                area,
                &ConfirmState::new(format!(
                    "Apply these changes?\nUpdate/create all {} reviewed files in {}.\n\n{}",
                    self.plan.rewrites.len(),
                    diff::visible(&self.project.display().to_string()),
                    if self.small {
                        "Apply is blocked until the terminal is at least 60 x 16."
                    } else {
                        "No files have been written. Y applies all; Enter returns to review."
                    }
                )),
            );
        }
        if self.help {
            let popup = tui::centered(area, area.width.min(88), area.height.min(18));
            frame.render_widget(Clear, popup);
            frame.render_widget(Paragraph::new("Upgrade review\n\nUp/Down, Home/End select files; Tab switches focus.\nIn the diff, Up/Down, Home/End and PgUp/PgDn scroll.\nLeft/Right pan long lines; Ctrl+Home/End jump in the diff.\n\n+ added, - removed, space unchanged context.\nCRLF and missing final newlines are marked; controls escaped.\nSkipped files are read-only warnings, outside Apply all.\n\nA opens Apply all confirmation. Enter means No; Y approves.\nEsc cancels a dialog, then Upgrade; Ctrl+C leaves for Home.\nBelow 60 x 16, Apply is blocked; Help and cancel still work.\n\n? or Esc closes Help.")
                .wrap(Wrap { trim: false }).block(Block::default().title(" Help ").borders(Borders::ALL)), popup);
        }
    }
}

fn crop(text: &str, offset: usize, width: usize) -> String {
    let mut column = 0;
    let mut result = String::new();
    for ch in text.chars() {
        let size = ch.width().unwrap_or(0);
        if column >= offset && column + size <= offset.saturating_add(width) {
            result.push(ch);
        }
        column += size;
        if column > offset.saturating_add(width) {
            break;
        }
    }
    result
}

pub(super) fn run(
    terminal: &mut TerminalSession,
    project: &Path,
    plan: &UpgradePlan,
) -> Result<Decision> {
    let mut review = Review::new(project, plan);
    crate::diagnostics::event("screen", "Upgrade review");
    loop {
        crate::interrupt::check()?;
        terminal.draw(|frame| review.draw(frame))?;
        if let Some(Event::Key(key)) = terminal.poll_event(Duration::from_millis(150))?
            && key.kind != KeyEventKind::Release
            && let Some(decision) = review.key(key)
        {
            return Ok(decision);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Rewrite;
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    use crate::test_common as common;

    #[test]
    fn borrowed_review_terminal_driver() {
        let Some(project) = std::env::var_os("RPROJ_UPGRADE_SELECTED_PROJECT") else {
            return;
        };
        let project = std::path::PathBuf::from(project);
        let mut terminal = TerminalSession::enter().unwrap();
        match super::super::open_in(&mut terminal, &project).unwrap() {
            super::super::ReviewOutcome::Apply(prepared) => {
                terminal.suspend().unwrap();
                assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
                prepared.execute().unwrap();
                terminal.resume().unwrap();
            }
            super::super::ReviewOutcome::Cancelled { .. } => {}
            super::super::ReviewOutcome::UpToDate(_) => panic!("expected reviewed changes"),
        }
        drop(terminal);
        assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
        println!("Upgrade terminal restored");
    }

    #[test]
    fn borrowed_terminal_reviews_and_writes_selected_project_when_launched_elsewhere() {
        for apply in [false, true] {
            let selected = common::TempProject::new("selected-upgrade");
            selected.write("default.project.json", "{}\n");
            selected.write("rproj.toml", "package_workflow='none'\n");
            selected.write("src/custom.luau", "return 'keep'\n");
            let launch = common::TempProject::new("launch-upgrade");
            launch.write("rproj.toml", "broken = [");
            let selected_before = common::project_snapshot(selected.path());
            let launch_before = common::project_snapshot(launch.path());
            let mut session = common::Session::start_program(
                &std::env::current_exe().unwrap(),
                launch.path(),
                &[
                    "--exact",
                    "commands::upgrade::review::tests::borrowed_review_terminal_driver",
                    "--nocapture",
                    "--test-threads=1",
                ],
                &[
                    (
                        "RPROJ_UPGRADE_SELECTED_PROJECT",
                        selected.path().to_str().unwrap(),
                    ),
                    ("RPROJ_NO_LOG", "1"),
                ],
            );
            session.wait_for("Upgrade review");
            session.wait_for("selected-upgrade");
            if apply {
                session.send("a");
                session.wait_for("Apply these changes?");
                session.send("y");
            } else {
                session.send(common::ESC);
            }
            let outcome = session.finish();
            assert_eq!(outcome.code, 0, "{}", outcome.text);
            outcome.assert_contains("Upgrade terminal restored");
            assert_eq!(common::project_snapshot(launch.path()), launch_before);
            if apply {
                assert!(selected.exists(".gitignore"));
                assert!(selected.exists(".luaurc"));
                assert_eq!(selected.read("src/custom.luau"), "return 'keep'\n");
            } else {
                assert_eq!(common::project_snapshot(selected.path()), selected_before);
            }
        }
    }

    fn plan() -> UpgradePlan {
        let contents = (0..70)
            .map(|i| format!("row{i:02} 東京 {}\n", "long ".repeat(24)))
            .collect::<String>();
        UpgradePlan {
            rewrites: vec![
                Rewrite {
                    relative: "selene.toml".into(),
                    contents: contents.clone(),
                    reason: "Update managed settings",
                    creating: false,
                },
                Rewrite {
                    relative: "new.json".into(),
                    contents,
                    reason: "Create required configuration",
                    creating: true,
                },
            ],
            originals: [
                ("selene.toml".into(), Some("old\r\n".into())),
                ("new.json".into(), None),
            ]
            .into(),
            skipped: vec![".luaurc exists but couldn't be parsed"],
        }
    }
    fn press(review: &mut Review<'_>, code: KeyCode) -> Option<Decision> {
        review.key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    fn draw(review: &mut Review<'_>, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| review.draw(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn default_no_requires_explicit_approval_and_cancel_is_distinct() {
        let plan = plan();
        let mut review = Review::new(Path::new("project"), &plan);
        draw(&mut review, 120, 30);
        press(&mut review, KeyCode::Char('a'));
        assert!(draw(&mut review, 120, 30).contains("[No]"));
        assert_eq!(press(&mut review, KeyCode::Enter), None);
        assert!(!review.confirm);
        assert_eq!(press(&mut review, KeyCode::Char('y')), None);
        press(&mut review, KeyCode::Char('a'));
        assert_eq!(
            press(&mut review, KeyCode::Char('y')),
            Some(Decision::Apply)
        );
        review.confirm = false;
        assert_eq!(
            press(&mut review, KeyCode::Esc),
            Some(Decision::Cancel { home: false })
        );
        assert_eq!(
            review.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(Decision::Cancel { home: true })
        );
    }

    #[test]
    fn resize_help_and_file_changes_preserve_each_scroll_position() {
        let plan = plan();
        let mut review = Review::new(Path::new("東京 project"), &plan);
        draw(&mut review, 120, 30);
        press(&mut review, KeyCode::Down);
        press(&mut review, KeyCode::Tab);
        press(&mut review, KeyCode::PageDown);
        press(&mut review, KeyCode::Right);
        let expected = draw(&mut review, 120, 30);
        for (width, height) in [(80, 24), (60, 16), (40, 10)] {
            draw(&mut review, width, height);
            press(&mut review, KeyCode::Char('?'));
            assert!(draw(&mut review, width, height).contains("Help"));
            press(&mut review, KeyCode::Esc);
            if width < 60 {
                press(&mut review, KeyCode::Char('a'));
                press(&mut review, KeyCode::Down);
                assert!(!review.confirm);
            }
        }
        assert_eq!(draw(&mut review, 120, 30), expected);
        press(&mut review, KeyCode::Down);
        assert_ne!(draw(&mut review, 120, 30), expected);
        press(&mut review, KeyCode::Tab);
        press(&mut review, KeyCode::Up);
        press(&mut review, KeyCode::Down);
        assert_eq!(review.selected, 1);
        assert_eq!(review.scrolls[1].horizontal, 8);
    }

    #[test]
    fn pending_confirmation_cannot_apply_below_minimum_size() {
        let plan = plan();
        let mut review = Review::new(Path::new("project"), &plan);
        draw(&mut review, 120, 30);
        press(&mut review, KeyCode::Char('a'));
        assert!(draw(&mut review, 40, 10).contains("[No]"));
        assert_eq!(press(&mut review, KeyCode::Char('y')), None);
        assert!(review.confirm);
        press(&mut review, KeyCode::Enter);
        assert!(!review.confirm);
        assert_eq!(
            press(&mut review, KeyCode::Esc),
            Some(Decision::Cancel { home: false })
        );
    }

    #[test]
    fn warnings_and_unicode_long_lines_are_readable() {
        let plan = plan();
        let mut review = Review::new(Path::new("project"), &plan);
        press(&mut review, KeyCode::End);
        let output = draw(&mut review, 120, 30);
        assert!(output.contains(".luaurc exists but couldn't be parsed"));
        assert_eq!(crop("a東京bc", 1, 4), "東京");
        assert_eq!(crop("a東京bc", 5, 2), "bc");
        assert_eq!(crop("", 0, 10), "");
    }
}
