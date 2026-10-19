use super::*;
use gpui_kit::component::button::*;
use gpui_kit::prelude::FluentBuilder;
use rproj_core::upgrade::diff::Kind;

fn writes_label(writes: WriteState) -> &'static str {
    match writes {
        WriteState::NotStarted => "No configuration writes started",
        WriteState::MayHaveBegun => {
            "Changes may have started; inspect the project for partial changes"
        }
        WriteState::Completed => "All proposed replacements completed",
        WriteState::Unknown => "File changes could not be verified; inspect the project",
    }
}

impl Workbench {
    pub(in super::super) fn upgrade_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let available = self.client.is_some() && matches!(self.upgrade.support, Some(Ok(())));
        div().v_flex().gap_2().px_6().py_2()
            .child(div().h_flex().flex_wrap().gap_2()
                .child(Button::new("review-upgrade").icon(IconName::FileText).label("Review configuration changes…").disabled(!available || self.project.is_none() || self.running.is_some() || self.teardown_uncertain()).on_click(cx.listener(|this, _, window, cx| this.edit_upgrade(window, cx))))
                .when(self.upgrade.project.is_some(), |view| view.child(Button::new("resume-upgrade").label(format!("Resume review for {}", self.upgrade.name())).on_click(cx.listener(|this, _, window, cx| this.show_upgrade(window, cx))))))
            .child(div().text_sm().text_color(cx.theme().muted_foreground).child("rproj upgrade refreshes generated configuration. Package/tool version updates are separate."))
            .when(!available, |view| view.child(div().text_sm().child(self.upgrade.support.as_ref().and_then(|r| r.as_ref().err()).cloned().unwrap_or_else(|| "Checking Upgrade support in the bundled CLI…".into()))))
            .into_any_element()
    }
    pub(in super::super) fn render_upgrade(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.upgrade;
        let busy = self.running.is_some() || self.teardown_uncertain();
        let captured_path = div()
            .id("upgrade-captured-path")
            .role(Role::Group)
            .aria_label(
                state
                    .project
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            )
            .test_support()
            .min_w_0()
            .flex_none()
            .px_6()
            .py_2()
            .overflow_x_scroll()
            .whitespace_nowrap()
            .text_sm()
            .child(format!(
                "Captured project: {}",
                state
                    .project
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));
        let mut body = div().v_flex().gap_3().p_6()
            .child("Review generated-configuration changes. Apply updates every proposed file; safely skipped files stay unchanged. No package/tool versions are updated.");
        if state.loading {
            body = body.child(
                div()
                    .id("upgrade-loading")
                    .role(Role::Status)
                    .aria_label("Preparing captured review…")
                    .test_support()
                    .child("Preparing captured review…"),
            );
        }
        if let Some(error) = &state.error {
            body = body.child(
                div()
                    .id("upgrade-error")
                    .role(Role::Alert)
                    .aria_label(error.clone())
                    .test_support()
                    .text_color(cx.theme().danger)
                    .child(error.clone()),
            );
        }
        if let Some(result) = &state.result {
            let message = match result {
                Ok(r) => format!(
                    "{}. {}. {} Refresh review explicitly to inspect current or remaining changes.",
                    match r.status {
                        UpgradeStatus::Succeeded => "Configuration changes applied",
                        UpgradeStatus::Failed => "Configuration changes failed",
                        UpgradeStatus::Cancelled => "Configuration changes cancelled",
                        UpgradeStatus::Uncertain => "Configuration outcome unconfirmed",
                    },
                    writes_label(r.writes),
                    r.message.as_deref().unwrap_or("")
                ),
                Err(error) => format!(
                    "Worker teardown is unconfirmed: {error}. The window stays open; inspect possible partial changes."
                ),
            };
            body = body.child(
                div()
                    .id("upgrade-result")
                    .role(Role::Status)
                    .aria_label(message.clone())
                    .test_support()
                    .child(message),
            );
        }
        if let Some(prepared) = &state.prepared {
            let review = &prepared.review;
            body = body.child(
                div()
                    .id("upgrade-counts")
                    .role(Role::Group)
                    .aria_label(format!(
                        "{} proposed files; {} skipped",
                        review.files.len(),
                        review.skipped.len()
                    ))
                    .test_support()
                    .child(format!(
                        "{} proposed files · {} safely skipped",
                        review.files.len(),
                        review.skipped.len()
                    )),
            );
            if review.files.is_empty() && state.result.is_none() {
                body = body.child(if review.skipped.is_empty() { "Already up to date: no generated-configuration changes needed." } else { "No applicable changes. Skipped files remain unresolved; this does not mean the whole project is up to date." });
            }
            let files = div()
                .id("upgrade-files")
                .role(Role::Group)
                .aria_label("Proposed and skipped configuration files")
                .test_support()
                .v_flex()
                .gap_2()
                .min_w_0()
                .overflow_y_scroll()
                .children(
                    review
                        .files
                        .iter()
                        .map(|file| (&file.relative, &file.reason, false))
                        .chain(
                            review
                                .skipped
                                .iter()
                                .map(|file| (&file.relative, &file.reason, true)),
                        )
                        .enumerate()
                        .map(|(index, (relative, reason, skipped))| {
                            Button::new(SharedString::from(format!("upgrade-file-{index}")))
                                .label(format!(
                                    "{}{}",
                                    relative,
                                    if skipped { " · Skipped" } else { "" }
                                ))
                                .tooltip(reason.clone())
                                .selected(state.selected == index)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.upgrade.selected = index;
                                    this.upgrade_list.reset(this.upgrade.row_count());
                                    cx.notify();
                                }))
                        }),
                );
            let details = if let Some(file) = review.files.get(state.selected) {
                div().v_flex().size_full().min_w_0().min_h_0().gap_2()
                    .child(div().text_sm().child(format!("{} · {}", if file.creating { "New file" } else { "Changed file" }, file.reason)))
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Old / new line · + added · − removed · LF unless [CRLF] or [no newline]; controls escaped"))
                    .child(self.upgrade_diff(cx)).into_any_element()
            } else if let Some(file) = review
                .skipped
                .get(state.selected.saturating_sub(review.files.len()))
            {
                div()
                    .id("upgrade-skipped")
                    .role(Role::Group)
                    .aria_label(file.reason.clone())
                    .test_support()
                    .min_w_0()
                    .flex_1()
                    .child(format!(
                        "Skipped: {}. {}. No proposed replacement or diff.",
                        file.relative, file.reason
                    ))
                    .into_any_element()
            } else {
                div().child("No proposed file selected").into_any_element()
            };
            body = body.child(
                div()
                    .id(if stacked {
                        "upgrade-stacked"
                    } else {
                        "upgrade-adjacent"
                    })
                    .role(Role::Group)
                    .aria_label(if stacked {
                        "Stacked file list and diff"
                    } else {
                        "Adjacent file list and diff"
                    })
                    .test_support()
                    .flex()
                    .gap_3()
                    .min_w_0()
                    .when(stacked, |view| view.flex_col())
                    .child(if stacked {
                        files.max_h(px(160.)).into_any_element()
                    } else {
                        files
                            .w(px(220.))
                            .flex_none()
                            .max_h(px(360.))
                            .into_any_element()
                    })
                    .child(div().flex_1().min_w_0().h(px(320.)).child(details)),
            );
        }
        if state.watch_conflict(self.watch.as_ref()) {
            body = body.child(div().id("upgrade-watch-conflict").role(Role::Status).aria_label("Watch on this captured project blocks Apply; use the separate Stop control and wait for acknowledgement").test_support().child("Watch on this captured project blocks Apply. Use the separate global Stop control and wait for acknowledgement; Upgrade never stops or switches Watch."));
        }
        div()
            .id("upgrade-flow")
            .role(Role::Group)
            .aria_label("Generated configuration review")
            .test_support()
            .track_focus(&self.upgrade_focus)
            .v_flex()
            .size_full()
            .min_h_0()
            .child(captured_path)
            .child(
                div()
                    .id("upgrade-scroll")
                    .role(Role::Group)
                    .aria_label("Scrollable Upgrade review")
                    .test_support()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(body),
            )
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .flex_none()
                    .gap_2()
                    .px_6()
                    .py_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("upgrade-back")
                            .ghost()
                            .label("Back to project")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.upgrade.visible = false;
                                this.focus.focus(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("upgrade-refresh")
                            .label("Refresh review…")
                            .disabled(
                                busy || state.loading || !matches!(state.support, Some(Ok(()))),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                if let Some(project) = this.upgrade.project.clone() {
                                    this.confirm_upgrade_refresh(project, window, cx);
                                }
                            })),
                    )
                    .child(
                        Button::new("upgrade-wrap")
                            .label(if state.unwrapped {
                                "Wrap long lines"
                            } else {
                                "Unwrap long lines"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.upgrade.unwrapped = !this.upgrade.unwrapped;
                                let offset = this.upgrade_list.logical_scroll_top();
                                this.upgrade_list.reset(this.upgrade.row_count());
                                this.upgrade_list.scroll_to(offset);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("upgrade-apply")
                            .primary()
                            .label("Apply all proposed changes…")
                            .disabled(
                                busy || state.ready().is_none_or(|p| p.files.is_empty())
                                    || state.watch_conflict(self.watch.as_ref()),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm_upgrade_apply(window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }
    fn upgrade_diff(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(rows) = self
            .upgrade
            .prepared
            .as_ref()
            .and_then(|p| p.documents.get(self.upgrade.selected))
            .cloned()
        else {
            return div().into_any_element();
        };
        let unwrapped = self.upgrade.unwrapped;
        let focus = self.upgrade_diff_focus.clone();
        let state = self.upgrade_list.clone();
        let ring = cx.theme().ring;
        div()
            .id("upgrade-diff")
            .role(Role::Group)
            .aria_label("Read-only unified diff; arrow keys and Page Up/Down scroll")
            .test_support()
            .track_focus(&focus)
            .border_1()
            .border_color(cx.theme().border)
            .focus_visible(move |style| style.border_color(ring))
            .w_full()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.upgrade_diff_focus.focus(window, cx)),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !this.upgrade_diff_focus.is_focused(window) {
                    return;
                }
                let count = this.upgrade.row_count();
                if count == 0 {
                    return;
                }
                let index = this.upgrade_list.logical_scroll_top().item_ix;
                let next = match event.keystroke.key.as_str() {
                    "down" => index.saturating_add(1),
                    "up" => index.saturating_sub(1),
                    "pagedown" => index.saturating_add(10),
                    "pageup" => index.saturating_sub(10),
                    "home" => 0,
                    "end" => count - 1,
                    _ => return,
                };
                // Logical offsets also reach rows not yet measured by the viewport.
                this.upgrade_list.scroll_to(ListOffset {
                    item_ix: next.min(count - 1),
                    offset_in_item: px(0.),
                });
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                list(state, move |index, _, cx| {
                    let Some(row) = rows.get(index) else {
                        return div().into_any_element();
                    };
                    let old = row
                        .old_line
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "·".into());
                    let new = row
                        .new_line
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "·".into());
                    let color = match row.kind {
                        Kind::Removed => cx.theme().danger,
                        Kind::Added => cx.theme().primary,
                        Kind::Meta => cx.theme().muted_foreground,
                        Kind::Context => cx.theme().foreground,
                    };
                    div()
                        .id(SharedString::from(format!("upgrade-row-{index}")))
                        .role(Role::Group)
                        .aria_label(format!("{old} / {new} {}", row.text))
                        .test_support()
                        .h_flex()
                        .items_start()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .text_sm()
                        .text_color(color)
                        .child(div().w(px(72.)).flex_none().child(format!("{old} / {new}")))
                        .child(
                            div()
                                .id(("upgrade-row-text", index))
                                .min_w_0()
                                .flex_1()
                                .whitespace_normal()
                                .when(unwrapped, |view| {
                                    view.whitespace_nowrap().overflow_x_scroll()
                                })
                                .child(row.text.clone()),
                        )
                        .into_any_element()
                })
                .size_full(),
            )
            .into_any_element()
    }
}
