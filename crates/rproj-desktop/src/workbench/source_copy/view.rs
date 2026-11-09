use super::*;
use gpui_kit::component::button::*;
use gpui_kit::prelude::FluentBuilder;

impl Workbench {
    pub(in super::super) fn source_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .h_flex()
            .flex_wrap()
            .gap_2()
            .px_6()
            .py_2()
            .child(
                Button::new("copy-source")
                    .label("Copy source…")
                    .icon(gpui_kit::assets::IconName::Copy)
                    .disabled(self.project.is_none())
                    .on_click(cx.listener(|this, _, window, cx| this.edit_source(window, cx))),
            )
            .when(self.source.owner.is_some(), |v| {
                v.child(
                    Button::new("source-resume")
                        .ghost()
                        .label(format!("Resume Copy Source for {}", self.source.name()))
                        .on_click(cx.listener(|this, _, window, cx| this.show_source(window, cx))),
                )
            })
            .into_any_element()
    }

    pub(in super::super) fn render_source(
        &self,
        narrow: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let source = &self.source;
        let revision = source.revision;
        let partial = source
            .snapshot
            .as_ref()
            .is_some_and(|p| !p.raw.skipped.is_empty());
        let mut page = div().id("source-review").role(Role::Group)
            .aria_label("Captured Copy Source review").test_support().track_focus(&source.focus)
            .size_full().overflow_y_scroll().track_scroll(&source.page_scroll)
            .child(div().v_flex().gap_3().p_6()
                .child(div().text_lg().child(format!("Copy Source for {}", source.name())))
                .child(div().id("source-owner").overflow_x_scroll().child(source.owner.as_ref().map(|p| p.display().to_string()).unwrap_or_default()))
                .child("Captured snapshot · recursive src scope. Refresh explicitly collects newer content. Selecting a file only navigates the preview.")
                .child(div().id("source-status").role(Role::Status).aria_label(source.status.clone()).test_support().child(source.status.clone()))
                .child(div().h_flex().gap_2().flex_wrap()
                    .child(Button::new("source-refresh").label("Refresh").disabled(source.busy())
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(project) = this.source.owner.clone() { this.prepare_source(project, cx); }
                        })))
                    .child(Button::new("source-copy").primary().icon(gpui_kit::assets::IconName::Copy)
                        .label(if partial { "Copy included files" } else { "Copy source" }).disabled(!source.copyable())
                        .on_click(cx.listener(move |this, _, _, cx| this.copy_source_snapshot(revision, cx))))));
        if let Some(snapshot) = &source.snapshot {
            if partial {
                page = page.child(div().px_6().py_2().child(format!("Partial collection: {} path(s) skipped. Copy includes only the listed readable files.", snapshot.raw.skipped.len())));
                // Skips are bounded by the collector and virtualized like included files.
                let skipped = snapshot.clone();
                let count = skipped.raw.skipped.len();
                page = page.child(
                    div()
                        .id("source-skips")
                        .role(Role::Group)
                        .test_support()
                        .aria_label("Skipped paths and reasons; Home and End navigate")
                        .track_focus(&source.skips_focus)
                        .border_1()
                        .border_color(cx.theme().border)
                        .focus_visible({
                            let ring = cx.theme().ring;
                            move |v| v.border_color(ring)
                        })
                        .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                            if !this.source.skips_focus.is_focused(window) {
                                return;
                            }
                            let top = this.source.skips_scroll.0.borrow().base_handle.offset().y;
                            let index = ((-top / px(48.)) as usize).min(count - 1);
                            let next = match event.keystroke.key.as_str() {
                                "home" => 0,
                                "end" => count - 1,
                                "down" => (index + 1).min(count - 1),
                                "up" => index.saturating_sub(1),
                                _ => return,
                            };
                            this.source
                                .skips_scroll
                                .scroll_to_item(next, ScrollStrategy::Top);
                            cx.stop_propagation();
                            cx.notify();
                        }))
                        .child(
                            uniform_list(
                                "source-skip-rows",
                                skipped.raw.skipped.len(),
                                move |range, _, _| {
                                    range
                                        .map(|i| {
                                            div()
                                                .id(SharedString::from(format!("source-skip-{i}")))
                                                .h(px(48.))
                                                .px_6()
                                                .overflow_x_scroll()
                                                .v_flex()
                                                .child(skipped.raw.skipped[i].path.clone())
                                                .child(skipped.raw.skipped[i].reason.clone())
                                        })
                                        .collect::<Vec<_>>()
                                },
                            )
                            .track_scroll(&source.skips_scroll)
                            .size_full(),
                        )
                        .h(px(96.))
                        .w_full(),
                );
            }
            if !snapshot.raw.files.is_empty() {
                let files = self.source_files(cx);
                let preview = self.source_preview(snapshot.clone(), cx);
                page = page.child(
                    div()
                        .id(if narrow {
                            "source-stacked"
                        } else {
                            "source-adjacent"
                        })
                        .role(Role::Group)
                        .aria_label("Included files and complete combined text")
                        .test_support()
                        .flex()
                        .when(narrow, |v| v.flex_col())
                        .gap_3()
                        .px_6()
                        .pb_4()
                        .w_full()
                        .child(
                            div()
                                .min_w_0()
                                .when(narrow, |v| v.w_full().h(px(128.)))
                                .when(!narrow, |v| v.w(px(180.)).flex_none().h(px(300.)))
                                .child(files),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .h(px(300.))
                                .when(narrow, |v| v.w_full())
                                .when(!narrow, |v| v.flex_1())
                                .child(preview),
                        ),
                );
            }
        }
        page.child(div().px_6().pb_4().text_sm().child("Preview annotations (LF, CRLF, escaped controls and continuation rows) are not copied. Original text, path headers and separators are copied exactly. Limits: 8 MiB payload, 10,000 visited entries, 100,000 preview rows and 16 MiB annotated text. Copy requested means submitted to the platform clipboard, not verified by the OS.")).into_any_element()
    }

    fn source_files(&self, cx: &mut Context<Self>) -> AnyElement {
        let count = self
            .source
            .snapshot
            .as_ref()
            .map_or(0, |p| p.raw.files.len());
        let ring = cx.theme().ring;
        div()
            .id("source-files")
            .role(Role::Group)
            .aria_label("Included files; arrow keys select and navigate preview")
            .test_support()
            .track_focus(&self.source.files_focus)
            .size_full()
            .border_1()
            .border_color(cx.theme().border)
            .focus_visible(move |v| v.border_color(ring))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !this.source.files_focus.contains_focused(window, cx) || count == 0 {
                    return;
                }
                let index = match event.keystroke.key.as_str() {
                    "down" => (this.source.selected + 1).min(count - 1),
                    "up" => this.source.selected.saturating_sub(1),
                    "home" => 0,
                    "end" => count - 1,
                    _ => return,
                };
                this.select_source_file(index, cx);
                cx.stop_propagation();
            }))
            .child(
                uniform_list(
                    "source-file-rows",
                    count,
                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                        range
                            .map(|index| {
                                let path = this.source.snapshot.as_ref().unwrap().raw.files[index]
                                    .path
                                    .clone();
                                Button::new(SharedString::from(format!("source-file-{index}")))
                                    .ghost()
                                    .tab_stop(false)
                                    .w_full()
                                    .h(px(40.))
                                    .selected(this.source.selected == index)
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "source-file-path-{index}"
                                            )))
                                            .overflow_x_scroll()
                                            .child(path),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.source.files_focus.focus(window, cx);
                                        this.select_source_file(index, cx);
                                    }))
                            })
                            .collect::<Vec<_>>()
                    }),
                )
                .track_scroll(&self.source.files_scroll)
                .size_full(),
            )
            .into_any_element()
    }

    fn select_source_file(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(snapshot) = &self.source.snapshot
            && let Some(&row) = snapshot.file_rows.get(index)
        {
            self.source.selected = index;
            self.source.rows.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            });
            self.source
                .files_scroll
                .scroll_to_item(index, ScrollStrategy::Nearest);
            cx.notify();
        }
    }

    fn source_preview(&self, snapshot: Arc<Prepared>, cx: &mut Context<Self>) -> AnyElement {
        let keys = self.source.rows.clone();
        let count = snapshot.rows.len();
        let focus = self.source.preview_focus.clone();
        let clicked = focus.clone();
        let ring = cx.theme().ring;
        div()
            .id("source-preview")
            .role(Role::Group)
            .aria_label(
                "Complete read-only source snapshot; arrows, Page Up/Down, Home and End scroll",
            )
            .test_support()
            .track_focus(&focus)
            .size_full()
            .border_1()
            .border_color(cx.theme().border)
            .focus_visible(move |v| v.border_color(ring))
            .overflow_hidden()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                clicked.focus(window, cx)
            })
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                if !focus.is_focused(window) || count == 0 {
                    return;
                }
                let i = keys.logical_scroll_top().item_ix;
                let next = match event.keystroke.key.as_str() {
                    "down" => i + 1,
                    "up" => i.saturating_sub(1),
                    "pagedown" => i + 10,
                    "pageup" => i.saturating_sub(10),
                    "home" => 0,
                    "end" => count - 1,
                    _ => return,
                };
                keys.scroll_to(ListOffset {
                    item_ix: next.min(count - 1),
                    offset_in_item: px(0.),
                });
                cx.stop_propagation();
                window.refresh();
            })
            .child(
                list(self.source.rows.clone(), move |index, _, _| {
                    let row = &snapshot.rows[index];
                    div()
                        .id(SharedString::from(format!("source-row-{index}")))
                        .role(Role::Group)
                        .aria_label(format!("{} {}", row.line, row.text))
                        .test_support()
                        .h_flex()
                        .items_start()
                        .px_2()
                        .py_1()
                        .gap_2()
                        .text_sm()
                        .child(div().w(px(56.)).flex_none().child(format!(
                            "{}{}",
                            row.line,
                            if row.continuation { " ↳" } else { "" }
                        )))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .whitespace_normal()
                                .child(row.text.clone()),
                        )
                        .into_any_element()
                })
                .size_full(),
            )
            .into_any_element()
    }
}
