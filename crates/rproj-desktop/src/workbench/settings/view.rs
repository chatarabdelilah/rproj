use super::controller::Replace;
use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::*;
use gpui_kit::prelude::FluentBuilder;

fn value_label(value: Option<&serde_json::Value>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "Unset".into())
}

impl Workbench {
    pub(in super::super) fn settings_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let available = self.client.is_some() && matches!(self.settings.support, Some(Ok(())));
        div().v_flex().gap_2().px_6().py_2()
            .child(Button::new("configure-settings").icon(IconName::Wrench).label("Configure tool settings…")
                .disabled(!available || self.project.is_none() || self.running.is_some() || self.teardown_uncertain())
                .on_click(cx.listener(|this, _, window, cx| this.edit_settings(window, cx))))
            .when(self.settings.project.is_some(), |view| view.child(
                Button::new("resume-settings").label(format!("Resume settings for {}", self.settings.name()))
                    .on_click(cx.listener(|this, _, window, cx| this.show_settings(window, cx)))))
            .child(div().text_sm().child("Edit one tool's project settings. No installation, machine setup or rproj record required."))
            .when(!available, |view| view.child(div().text_sm().child(
                self.settings.support.as_ref().and_then(|r| r.as_ref().err()).cloned()
                    .unwrap_or_else(|| "Checking Configure Tools support in the bundled CLI…".into()))))
            .into_any_element()
    }

    pub(in super::super) fn render_settings(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.settings;
        let busy = self.running.is_some() || self.teardown_uncertain();
        let path = state
            .project
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let tool = tool_settings::find(&state.tool);
        let captured = format!(
            "{} · {} · {}",
            path,
            tool.map_or("", |t| t.display_name),
            tool.map_or("", |t| t.target.relative())
        );
        let mut status = div().v_flex().gap_2().text_sm();
        if state.loading {
            status = status.child(
                div()
                    .id("settings-loading")
                    .role(Role::Status)
                    .aria_label("Working on captured settings…")
                    .test_support()
                    .child("Working on captured settings…"),
            );
        }
        if let Some(error) = &state.error {
            status = status.child(
                div()
                    .id("settings-error")
                    .role(Role::Alert)
                    .aria_label(error.clone())
                    .test_support()
                    .text_color(cx.theme().danger)
                    .child(error.clone()),
            );
        }
        if state.reload_required {
            status = status.child("Reload from file explicitly before retrying. Existing edits are retained; no automatic merge is performed.");
        }
        if let Some(result) = &state.result {
            let message = match result {
                Ok(r) => {
                    let status = match r.status {
                        SettingsStatus::Succeeded => {
                            "Settings saved. The displayed values now match the saved file."
                        }
                        SettingsStatus::Failed => {
                            "Settings were not saved successfully. Your edits are retained."
                        }
                        SettingsStatus::Cancelled | SettingsStatus::Uncertain => {
                            "Save completion is uncertain. Reload from file before retrying."
                        }
                    };
                    let writes = match r.writes {
                        WriteState::NotStarted => "No settings writes began.",
                        WriteState::Completed => "The reviewed file was replaced.",
                        _ => "File changes may have begun; inspect the captured file.",
                    };
                    format!("{status} {writes} {}", r.message.as_deref().unwrap_or(""))
                }
                Err(e) => format!(
                    "Worker teardown unconfirmed: {e}. Closing and further mutations are blocked."
                ),
            };
            status = status.child(
                div()
                    .id("settings-result")
                    .role(Role::Status)
                    .aria_label(message.clone())
                    .test_support()
                    .child(message),
            );
        }
        if state.watch_conflict(self.watch.as_ref()) {
            status = status.child("Watch owns this project. Use the separate Stop control and wait for acknowledgement before saving.");
        }
        let body = if state.reviewing {
            self.settings_review(cx)
        } else {
            let targets = div()
                .id("settings-targets")
                .role(Role::Group)
                .aria_label("Configuration targets")
                .test_support()
                .v_flex()
                .gap_2()
                .min_w_0()
                .when(stacked, |v| v.h_flex().flex_wrap())
                .children(tool_settings::CONFIGURABLE_TOOLS.iter().map(|target| {
                    Button::new(SharedString::from(format!(
                        "settings-target-{}",
                        target.key
                    )))
                    .label(target.display_name)
                    .selected(state.tool == target.key)
                    .disabled(busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.settings.tool != target.key
                            && let Some(project) = this.settings.project.clone()
                        {
                            this.replace_settings(
                                Replace::Load(project, target.key.into()),
                                window,
                                cx,
                            );
                        }
                    }))
                }));
            let fields = div()
                .id("settings-fields")
                .role(Role::Group)
                .aria_label("Settings and current values")
                .test_support()
                .v_flex()
                .gap_4()
                .min_w_0()
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .when_some(tool, |view, tool| {
                    view.child(div().text_sm().child(tool.summary)).child(
                        gpui_kit::base::Link::new("settings-docs")
                            .href(tool.docs_url)
                            .text_color(cx.theme().link)
                            .child("Documentation ↗"),
                    )
                })
                .children(if let Some(loaded) = state.loaded.as_ref() {
                    loaded
                        .context
                        .target()
                        .unwrap()
                        .settings
                        .iter()
                        .enumerate()
                        .map(|(index, setting)| self.settings_field(index, setting, loaded, cx))
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                });
            div()
                .id(if stacked {
                    "settings-stacked"
                } else {
                    "settings-adjacent"
                })
                .role(Role::Group)
                .aria_label(if stacked {
                    "Stacked tool settings"
                } else {
                    "Adjacent tool settings"
                })
                .test_support()
                .flex()
                .gap_4()
                .size_full()
                .min_h_0()
                .min_w_0()
                .when(stacked, |v| v.flex_col())
                .child(targets.when(!stacked, |v| v.w(rems(12.)).flex_none()))
                .child(fields)
                .into_any_element()
        };
        div()
            .id("settings-flow")
            .role(Role::Group)
            .aria_label("Configure tool settings")
            .test_support()
            .track_focus(&self.settings_focus)
            .v_flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(
                div()
                    .id("settings-captured-path")
                    .role(Role::Group)
                    .aria_label(captured.clone())
                    .test_support()
                    .px_6()
                    .py_2()
                    .flex_none()
                    .overflow_x_scroll()
                    .whitespace_nowrap()
                    .text_sm()
                    .child(captured),
            )
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .px_6()
                    .py_2()
                    .flex_none()
                    .child(
                        Button::new("settings-step-settings")
                            .label("1 · Settings")
                            .selected(!state.reviewing)
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings.reviewing = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("settings-step-review")
                            .label("2 · Review")
                            .selected(state.reviewing)
                            .disabled(busy || !state.dirty() || state.reload_required)
                            .on_click(cx.listener(|this, _, _, cx| this.prepare_settings(cx))),
                    ),
            )
            .child(
                div()
                    .id("settings-status")
                    .max_h(rems(7.))
                    .overflow_y_scroll()
                    .px_6()
                    .child(status),
            )
            .child(div().flex_1().min_h_0().min_w_0().px_6().py_2().child(body))
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .px_6()
                    .py_2()
                    .flex_none()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("settings-back")
                            .ghost()
                            .label("Back to project")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.settings.visible = false;
                                this.focus_settings(window, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("settings-reload")
                            .label("Reload from file")
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                if let Some(project) = this.settings.project.clone() {
                                    this.replace_settings(
                                        Replace::Load(project, this.settings.tool.clone()),
                                        window,
                                        cx,
                                    );
                                }
                            })),
                    )
                    .child(
                        Button::new("settings-discard")
                            .label("Discard draft")
                            .disabled(busy || !state.dirty())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.replace_settings(Replace::Discard(false), window, cx)
                            })),
                    )
                    .child(
                        Button::new("settings-save")
                            .primary()
                            .label("Save settings…")
                            .disabled(
                                busy || state.ready().is_none()
                                    || state.watch_conflict(self.watch.as_ref()),
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.confirm_settings_save(window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    fn settings_field(
        &self,
        index: usize,
        setting: &'static tool_settings::SettingSpec,
        loaded: &SettingsLoaded,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let busy = self.running.is_some() || self.teardown_uncertain();
        let editable = !busy && self.settings.editable(index);
        let current = value_label(loaded.draft.current[index].as_ref());
        let pending = loaded.draft.changes[index].as_ref().map(|v| v.to_string());
        let unknown = loaded.draft.current[index]
            .as_ref()
            .is_some_and(|v| !setting.kind.accepts(v));
        let controls = match &setting.kind {
            SettingKind::Bool { .. } => div()
                .h_flex()
                .gap_2()
                .children([true, false].map(|value| {
                    Button::new(SharedString::from(format!(
                        "settings-value-{index}-{value}"
                    )))
                    .label(value.to_string())
                    .selected(loaded.draft.value(index).and_then(|v| v.as_bool()) == Some(value))
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.running.is_none() {
                            this.settings.set(index, serde_json::json!(value));
                            cx.notify();
                        }
                    }))
                }))
                .into_any_element(),
            SettingKind::Choice { options, .. } => div()
                .h_flex()
                .flex_wrap()
                .gap_2()
                .children(options.iter().enumerate().map(|(choice, option)| {
                    Button::new(SharedString::from(format!(
                        "settings-value-{index}-{choice}"
                    )))
                    .label(option.value)
                    .tooltip(option.explanation)
                    .selected(
                        loaded.draft.value(index).and_then(|v| v.as_str()) == Some(option.value),
                    )
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.running.is_none() {
                            this.settings.set(index, serde_json::json!(option.value));
                            cx.notify();
                        }
                    }))
                }))
                .into_any_element(),
            SettingKind::Integer { .. } => {
                Button::new(SharedString::from(format!("settings-integer-{index}")))
                    .label("Edit integer…")
                    .disabled(!editable)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.edit_settings_integer(index, window, cx)
                    }))
                    .into_any_element()
            }
        };
        div()
            .id(SharedString::from(format!("settings-field-{index}")))
            .role(Role::Group)
            .aria_label(format!("{} · Current: {current}", setting.display_key()))
            .test_support()
            .v_flex()
            .gap_2()
            .pb_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(setting.display_key()),
            )
            .child(div().text_sm().child(setting.description))
            .child(div().text_sm().child(format!(
                "Current file: {current} · Suggested default: {}",
                setting.kind.default_value()
            )))
            .when_some(pending, |view, pending| {
                view.child(div().text_sm().child(format!("Pending: {pending}")))
            })
            .when_some(loaded.blocked.get(&index), |view, error| {
                view.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(format!("Blocked: {error}")),
                )
            })
            .when(unknown && !self.settings.consent.contains(&index), |view| {
                view.child(
                    Button::new(SharedString::from(format!("settings-custom-{index}")))
                        .label("Replace unfamiliar value…")
                        .disabled(
                            busy || loaded.blocked.contains_key(&index)
                                || self.settings.reload_required,
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.consent_settings_value(index, window, cx)
                        })),
                )
            })
            .child(controls)
            .child(
                div()
                    .h_flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(SharedString::from(format!("settings-default-{index}")))
                            .label("Use suggested default")
                            .disabled(!editable)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.running.is_none() {
                                    this.settings.set(index, setting.kind.default_value());
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        Button::new(SharedString::from(format!("settings-revert-{index}")))
                            .ghost()
                            .label("Revert change")
                            .disabled(busy || loaded.draft.changes[index].is_none())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.running.is_none()
                                    && let Some(loaded) = this.settings.loaded.as_mut()
                                {
                                    loaded.draft.revert(index);
                                    this.settings.consent.remove(&index);
                                    this.settings.changed();
                                    cx.notify();
                                }
                            })),
                    ),
            )
            .into_any_element()
    }

    fn settings_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(prepared) = &self.settings.prepared else {
            return div().child("Prepare Review again.").into_any_element();
        };
        let review = &prepared.review;
        let tool = review.context.target().unwrap();
        let summary = tool.settings.iter().enumerate().filter_map(|(i, setting)| {
            review.draft.changes[i].as_ref().map(|value| {
                format!(
                    "{}: {} → {value}",
                    setting.display_key(),
                    value_label(review.draft.current[i].as_ref())
                )
            })
        });
        div().v_flex().size_full().min_h_0().min_w_0().gap_2()
            .child(div().id("settings-changes").max_h(rems(5.5)).overflow_y_scroll().v_flex().text_sm()
                .child(if review.context.original.is_none() { "A missing file will be created." } else if review.context.original.as_ref().is_some_and(|s| s.is_empty()) { "The existing empty file will receive these settings." } else { "The existing file will be updated." })
                .child("Review includes writer formatting and legacy-key cleanup. Saving does not install or enable a tool.")
                .children(summary.map(|line| div().child(line))))
            .child(div().text_xs().child("Old / new line · + added · − removed · LF unless [CRLF] or [no newline]; controls escaped"))
            .child(super::super::diff_view::render("settings", prepared.rows.clone(), self.settings_list.clone(), self.settings_diff_focus.clone(), false, cx))
            .into_any_element()
    }
}
