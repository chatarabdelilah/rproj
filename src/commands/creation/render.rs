use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::{
    super::new,
    model::{Draft, Modal},
};
use crate::tui;

pub fn draw(frame: &mut Frame<'_>, draft: &Draft, destination: &str) {
    let area = frame.area();
    if tui::is_too_small(area) {
        frame.render_widget(
            Paragraph::new("Resize to at least 60x16\n? Help   Esc Back   Ctrl+C Exit"),
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
            Paragraph::new(format!(
                "rproj {} | New Project: {}",
                env!("CARGO_PKG_VERSION"),
                draft.name
            ))
            .style(tui::title_style()),
            rows[0],
        );
        let panes = tui::responsive_panes(rows[1], 40);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", draft.title()))
            .border_style(Style::default().fg(if draft.details_focus {
                tui::MUTED
            } else {
                tui::ACCENT
            }));
        let inner = block.inner(panes[0]);
        frame.render_widget(block, panes[0]);
        let list_rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
        frame.render_widget(
            Paragraph::new(format!("Filter: {}", draft.picker.query.text())),
            list_rows[0],
        );
        let items = draft
            .picker
            .filtered()
            .iter()
            .map(|p| {
                let mark = if draft.multi() {
                    if draft.checked.contains(&p.value) {
                        "[x] "
                    } else {
                        "[ ] "
                    }
                } else {
                    ""
                };
                ListItem::new(format!("{mark}{}", p.label))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default().with_selected(if items.is_empty() {
            None
        } else {
            Some(draft.picker.selected)
        });
        frame.render_stateful_widget(
            List::new(items).highlight_style(tui::selected_style(!draft.details_focus)),
            list_rows[1],
            &mut state,
        );

        let selected = draft
            .picker
            .filtered()
            .get(draft.picker.selected)
            .map(|p| p.detail.as_str())
            .unwrap_or("");
        let plan = draft.graph.plan(&draft.apps, &draft.extensions);
        let mut details = format!("{selected}\n\nDestination: {destination}\n");
        if !draft.status.is_empty() {
            details.push_str(&format!("{}\n", draft.status));
        }
        if let Some(name) = &draft.save_setup {
            details.push_str(&format!("Save setup on Create: {name}\n"));
        }
        details.push('\n');
        details.push_str(&new::summary_lines(&draft.name, &draft.graph, &plan).join("\n"));
        frame.render_widget(
            Paragraph::new(details)
                .wrap(Wrap { trim: false })
                .scroll((draft.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Details / planned files ")
                        .border_style(Style::default().fg(if draft.details_focus {
                            tui::ACCENT
                        } else {
                            tui::MUTED
                        })),
                ),
            panes[1],
        );
        tui::render_footer(
            frame,
            rows[2],
            &draft.status,
            if draft.multi() {
                "Space toggle  Enter continue  Tab focus  PgUp/Dn details  Esc back  ? help"
            } else {
                "Enter select  Tab focus  PgUp/Dn details  Esc back  ? help"
            },
            !draft.status.is_empty(),
        );
    }
    match &draft.modal {
        Some(Modal::Name(input)) => {
            tui::render_input(frame, area, "Project name", "One folder name", input)
        }
        Some(Modal::Setup(input)) => tui::render_input(
            frame,
            area,
            "Save named setup",
            "New setup name; leave empty to disable saving",
            input,
        ),
        Some(Modal::Discard(confirm) | Modal::Create(confirm)) => {
            tui::render_confirm(frame, area, confirm)
        }
        Some(Modal::Help) => {
            let popup = tui::centered(area, 76, 16);
            frame.render_widget(Clear, popup);
            frame.render_widget(Paragraph::new("New Project\n\nArrows navigate; type to filter. Space toggles multiple choices.\nEnter accepts the current step. Tab switches to details; arrows or Page Up/Down scroll.\n\nReview can revise answers, omit optional files, rename, and save a new setup. Required files cannot be removed.\n\nEsc goes back or cancels a revision. Ctrl+C exits without creating anything. Only confirmed Create writes a project.\n\nEsc or ? closes Help.")
                .wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).title(" Help ")), popup);
        }
        None => {}
    }
}
