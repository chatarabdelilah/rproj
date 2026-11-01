use super::*;
use gpui_kit::component::{button::*, input::Input};
use gpui_kit::prelude::FluentBuilder;
use rproj_core::{FileState, project_discovery::Source};

fn source_label(source: Source) -> &'static str {
    match source {
        Source::ProjectsFolder => "Projects folder",
        Source::LaunchFolder => "Launch folder",
        Source::Both => "Projects folder and launch folder",
    }
}
fn file_label(state: FileState) -> &'static str {
    match state {
        FileState::Missing => "Absent",
        FileState::Ready => "Readable",
        FileState::Invalid => "Malformed",
        FileState::Unreadable => "Unreadable",
    }
}

impl Workbench {
    pub(in super::super) fn browser_empty_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let focus = self.browser.empty_focus.clone();
        div()
            .id("browse-empty-focus")
            .role(Role::Group)
            .aria_label("Browse projects")
            .test_support()
            .track_focus(&focus)
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, w, cx| {
                if focus.is_focused(w) && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    this.show_projects(true, w, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                Button::new("browse-empty")
                    .label("Browse projects…")
                    .icon(IconName::FolderOpen)
                    .on_click(cx.listener(|this, _, w, cx| this.show_projects(true, w, cx))),
            )
            .into_any_element()
    }
    pub(in super::super) fn render_projects(
        &self,
        w: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let narrow =
            w.viewport_size().width - theme::SIDEBAR_WIDTH.to_pixels(w.rem_size()) < px(600.);
        let b = &self.browser;
        let mut body = div()
            .id("projects-browser")
            .role(Role::Group)
            .aria_label("Browse projects")
            .test_support()
            .track_focus(&b.focus)
            .size_full()
            .min_w_0()
            .min_h_0()
            .v_flex()
            .gap_2()
            .p(theme::PANEL_PADDING)
            .overflow_y_scroll()
            .on_key_down(cx.listener(|this, key: &KeyDownEvent, w, cx| {
                if key.keystroke.key == "escape" {
                    this.back_from_projects(w, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("projects-back")
                            .label("Back")
                            .on_click(cx.listener(|this, _, w, cx| this.back_from_projects(w, cx))),
                    )
                    .child(
                        Button::new("projects-refresh")
                            .label("Refresh projects")
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_projects(cx))),
                    ),
            );
        if let Some(root) = b.listing.as_ref().and_then(|s| s.root.as_ref()) {
            body = body.child(
                div()
                    .id("projects-root")
                    .role(Role::Group)
                    .aria_label(root.display().to_string())
                    .test_support()
                    .overflow_x_scroll()
                    .whitespace_nowrap()
                    .child(format!("Projects folder · {}", root.display())),
            );
        }
        if let Some(scan) = &b.listing {
            for warning in &scan.found.warnings {
                body = body.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().warning)
                        .child(warning.clone()),
                );
            }
        }
        if b.scan.active.is_some() {
            body = body.child(if b.listing.is_some() {
                "Refreshing projects…"
            } else {
                "Discovering projects…"
            });
        }
        if let Some(message) = &b.notice {
            body = body.child(
                div()
                    .id("projects-notice")
                    .role(Role::Alert)
                    .aria_label(message.clone())
                    .test_support()
                    .child(message.clone()),
            );
        }

        let focus = b.rows_focus.clone();
        let mut rows = div()
            .id("projects-list")
            .role(Role::Group)
            .aria_label("Project previews; use arrow keys, Home or End to select")
            .test_support()
            .track_focus(&focus)
            .v_flex()
            .min_w_0()
            .min_h_0()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .p_2()
            .focus_visible({
                let ring = cx.theme().ring;
                move |style| style.border_color(ring)
            })
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, w, cx| {
                if this.browser.query.read(cx).focus_handle(cx).is_focused(w) {
                    return;
                }
                let n = this.browser.indices.len();
                if n == 0 {
                    return;
                }
                let at = (0..n)
                    .find(|&i| {
                        this.browser.at(i).map(|p| &p.path) == this.browser.selected.as_ref()
                    })
                    .unwrap_or(0);
                let ix = match event.keystroke.key.as_str() {
                    "up" => at.saturating_sub(1),
                    "down" => (at + 1).min(n - 1),
                    "home" => 0,
                    "end" => n - 1,
                    _ => return,
                };
                let path = this.browser.at(ix).unwrap().path.clone();
                this.browser.rows_focus.focus(w, cx);
                this.browser
                    .scroll
                    .scroll_to_item(ix, ScrollStrategy::Center);
                this.browser.notice = None;
                this.select_project_preview(Some(path), cx);
                cx.stop_propagation();
            }))
            .child(Input::new(&b.query))
            .child(format!("{} projects", b.indices.len()));
        if b.indices.is_empty() {
            rows = rows.child(if b.listing.is_none() {
                "Waiting for discovery…"
            } else if b
                .listing
                .as_ref()
                .is_some_and(|s| s.found.entries.is_empty())
            {
                "No recognized projects were found."
            } else {
                "No projects match this search."
            });
        } else {
            let list = uniform_list(
                "projects-rows",
                b.indices.len(),
                cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                    range
                        .filter_map(|ix| {
                            this.browser.at(ix).map(|p| {
                                let path = p.path.clone();
                                Button::new(SharedString::from(format!("project-row-{ix}")))
                                    .ghost()
                                    .w_full()
                                    .h(px(56.))
                                    .selected(this.browser.selected.as_ref() == Some(&path))
                                    .child(
                                        div()
                                            .v_flex()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .child(p.name.clone())
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .truncate()
                                                    .child(path.display().to_string()),
                                            ),
                                    )
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        this.browser.notice = None;
                                        this.browser.rows_focus.focus(w, cx);
                                        this.select_project_preview(Some(path.clone()), cx);
                                    }))
                            })
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(&b.scroll)
            .h_full()
            .w_full();
            rows = rows.child(
                div()
                    .h(if narrow { px(170.) } else { px(280.) })
                    .child(list),
            );
        }
        let mut detail = div()
            .id("projects-detail")
            .role(Role::Group)
            .aria_label("Project preview")
            .test_support()
            .min_w_0()
            .v_flex()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Project preview"),
            );
        if let Some(path) = &b.selected {
            if let Some(entry) = b
                .listing
                .as_ref()
                .and_then(|s| s.found.entries.iter().find(|p| &p.path == path))
            {
                detail = detail
                    .child(div().text_lg().child(entry.name.clone()))
                    .child(source_label(entry.source));
            }
            detail = detail.child(
                div()
                    .id("projects-preview-path")
                    .role(Role::Group)
                    .aria_label(path.display().to_string())
                    .test_support()
                    .overflow_x_scroll()
                    .whitespace_nowrap()
                    .child(path.display().to_string()),
            );
            match &b.detail {
                Some(Ok(snapshot)) => {
                    detail = detail
                        .child(format!(
                            "rproj.toml · {}",
                            file_label(snapshot.files.rproj.state)
                        ))
                        .child(format!(
                            "default.project.json · {}",
                            file_label(snapshot.files.rojo.state)
                        ));
                    for diagnostic in &snapshot.diagnostics {
                        detail = detail.child(div().text_sm().child(format!(
                            "{} · {}",
                            diagnostic.path.display(),
                            diagnostic.message
                        )));
                    }
                }
                Some(Err(message)) => {
                    detail = detail.child(
                        div()
                            .id("projects-preview-error")
                            .role(Role::Alert)
                            .aria_label(message.clone())
                            .test_support()
                            .text_color(cx.theme().danger)
                            .child(message.clone()),
                    )
                }
                None => detail = detail.child("Reading preview…"),
            }
        } else {
            detail = detail.child("Select a project to inspect its files.");
        }
        detail = detail.child(
            Button::new("projects-open")
                .primary()
                .label(if b.opening.active.is_some() {
                    "Checking folder…"
                } else {
                    "Open project"
                })
                .disabled(!matches!(&b.detail, Some(Ok(_))) || b.opening.active.is_some())
                .on_click(cx.listener(|this, _, _, cx| this.open_previewed_project(cx))),
        );
        let panes = if narrow {
            div().id("projects-stacked").v_flex()
        } else {
            div().id("projects-adjacent").h_flex().items_start()
        }
        .test_support()
        .gap_3()
        .min_w_0();
        body.child(
            panes
                .child(rows.when(!narrow, |r| r.flex_1()))
                .child(detail.when(!narrow, |d| d.flex_1())),
        )
        .into_any_element()
    }
}
