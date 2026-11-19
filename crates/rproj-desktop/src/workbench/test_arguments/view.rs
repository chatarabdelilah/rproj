use super::controller::Intent;
use super::*;
use gpui_kit::component::{button::*, input::Input};
use gpui_kit::prelude::FluentBuilder;

impl Workbench {
    pub(in super::super) fn test_arguments_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let available = self.client.is_some() && matches!(self.test_args.support, Some(Ok(())));
        div().v_flex().gap_2()
            .child(Button::new("edit-test-arguments").label("Run with arguments…")
                .disabled(!available || self.running.is_some() || self.teardown_uncertain())
                .on_click(cx.listener(|this, _, w, cx| this.edit_test_args(w, cx))))
            .when(!available, |v| v.child(self.test_args.support.as_ref().and_then(|r| r.as_ref().err()).cloned()
                .unwrap_or_else(|| "Custom arguments need a matching CLI; checking support. Default Run tests remains independent.".into())))
            .into_any_element()
    }
    pub(in super::super) fn render_test_arguments(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.test_args;
        let available = self.client.is_some() && matches!(state.support, Some(Ok(())));
        let busy = self.running.is_some() || self.teardown_uncertain();
        let owner = state
            .project
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let mut rows = div()
            .id("test-argument-rows")
            .v_flex()
            .gap_3()
            .flex_1()
            .min_w_0()
            .child("Ordered values — no shell parsing or expansion")
            .when(state.rows.is_empty(), |v| {
                v.child("No extra arguments. Add a row to supply one exact value.")
            });
        for (index, row) in state.rows.iter().enumerate() {
            let id = row.id;
            let Some(input) = self.test_args_inputs.rows.get(&id) else {
                continue;
            };
            let owner = cx.entity().downgrade();
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("test-row-{id}")))
                    .v_flex()
                    .gap_2()
                    .min_w_0()
                    .child(format!("Argument {}", index + 1))
                    .child(
                        Input::new(input)
                            .id(SharedString::from(format!("test-input-{id}")))
                            .aria_label(format!("Argument {} value", index + 1))
                            .disabled(state.dialog || self.teardown_uncertain())
                            .on_paste(move |item, w, cx| {
                                let Some(text) = item.text() else {
                                    return false;
                                };
                                owner
                                    .update(cx, |this, cx| this.paste_test_row(id, &text, w, cx))
                                    .unwrap_or(true)
                            }),
                    )
                    .child(
                        div()
                            .h_flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                Button::new(SharedString::from(format!("test-literal-{id}")))
                                    .label("Literal")
                                    .disabled(state.dialog || self.teardown_uncertain())
                                    .selected(row.mode == Mode::Literal)
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        this.change_test_mode(id, Mode::Literal, w, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("test-json-{id}")))
                                    .label("JSON string")
                                    .disabled(state.dialog || self.teardown_uncertain())
                                    .selected(row.mode == Mode::Json)
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        this.change_test_mode(id, Mode::Json, w, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("test-empty-{id}")))
                                    .label("Pass empty argument")
                                    .disabled(state.dialog || self.teardown_uncertain())
                                    .selected(row.empty)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        if let Some(row) =
                                            this.test_args.rows.iter_mut().find(|r| r.id == id)
                                        {
                                            row.empty = !row.empty;
                                            row.edit(row.text.clone());
                                            this.test_args.changed();
                                            cx.notify();
                                        }
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("test-up-{id}")))
                                    .label("Up")
                                    .disabled(index == 0 || state.dialog || self.teardown_uncertain())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.move_test_row(id, true, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("test-down-{id}")))
                                    .label("Down")
                                    .disabled(index + 1 == state.rows.len() || state.dialog || self.teardown_uncertain())
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.move_test_row(id, false, cx)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from(format!("test-remove-{id}")))
                                    .label("Remove")
                                    .disabled(state.dialog || self.teardown_uncertain())
                                    .on_click(cx.listener(move |this, _, w, cx| {
                                        this.remove_test_row(id, w, cx)
                                    })),
                            ),
                    )
                    .when_some(row.value().err(), |v, error| {
                        v.child(div().text_color(cx.theme().danger).child(error))
                            .child(text(format!("test-last-valid-{id}"), format!("Last valid value (not submitted while this row is invalid): {}", serde_json::to_string(&row.last_valid).unwrap())))
                    }),
            );
        }
        rows = rows.child(
            Button::new("test-add-argument")
                .label("Add argument")
                .disabled(state.dialog || self.teardown_uncertain())
                .on_click(cx.listener(|this, _, w, cx| this.add_test_row(w, cx))),
        );
        let mut review = div()
            .id("test-invocation-review")
            .v_flex()
            .gap_3()
            .flex_1()
            .min_w_0()
            .child("Invocation review · exact JSON string values")
            .child("Captured project — browsing never retargets this draft. Values are arguments, not a shell command.");
        if let Some(context) = &state.context {
            review = review.child(match context.runner {
                rproj_core::creation::graph::TestRunner::TestEz => "Runner: TestEZ (Lute)",
                rproj_core::creation::graph::TestRunner::JestRoblox => match context.backend {
                    rproj_core::creation::graph::JestBackend::Studio => {
                        "Runner: Jest Roblox · Recorded backend: Studio"
                    }
                    rproj_core::creation::graph::JestBackend::OpenCloud => {
                        "Runner: Jest Roblox · Recorded backend: Open Cloud"
                    }
                },
            });
            match state
                .values()
                .and_then(|values| context.invocation(&values).map_err(|e| e.to_string()))
            {
                Ok(invocation) => {
                    review = review.child(text("test-program", invocation.program));
                    for (index, arg) in invocation.arguments.iter().enumerate() {
                        review = review.child(text(
                            format!("test-preview-{index}"),
                            serde_json::to_string(arg).unwrap(),
                        ));
                    }
                }
                Err(error) => {
                    review = review.child(text(
                        "test-invocation-invalid",
                        format!("Cannot run: {error}"),
                    ))
                }
            }
            for effect in context.effects() {
                review = review.child(effect_text(effect));
            }
            let key = if context.runner == rproj_core::creation::graph::TestRunner::TestEz {
                "lute"
            } else {
                "jest-roblox"
            };
            if let Some(tool) = rproj_core::catalog::tool_catalog::find(key) {
                let url = tool.docs_url;
                review = review.child(
                    Button::new("test-runner-docs")
                        .label("Runner documentation")
                        .on_click(move |_, _, cx| cx.open_url(url)),
                );
            }
        } else {
            review = review.child(
                "Context has not been loaded. Refresh reads configuration without running tests.",
            );
        }
        if let Some((_, submitted)) = &state.submitted {
            review = review
                .child(text(
                    "test-submitted",
                    format!(
                        "Captured submission: {}",
                        serde_json::to_string(submitted).unwrap()
                    ),
                ))
                .child("Edits made while running belong to the next submission.");
        }
        if let Some(result) = &state.result {
            review = review.child(text(
                "test-result",
                format!(
                    "{:?} · Submission: {} · Runner exit: {}",
                    result.status,
                    match result.accepted {
                        Some(true) => "accepted",
                        Some(false) => "not accepted",
                        None => "unknown",
                    },
                    result
                        .runner_exit
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "not reported".into())
                ),
            ));
        }
        div().id("test-arguments-screen").test_support().track_focus(&self.test_args_focus).size_full().min_h_0().v_flex()
            .child(div().id("test-arguments-scroll").test_support().flex_1().min_h_0().overflow_y_scroll().p_6().v_flex().gap_3()
                .child(text("test-captured", format!("{} · {owner}", state.name())))
                .child(if state.dirty() { "Unsubmitted edits" } else { "No unsubmitted edits" })
                .when(matches!(self.running, Some((Operation::LoadTestContext, _))), |v| v.child(text("test-context-loading", "Reading test context…")))
                .when(!available, |v| v.child("Custom tests unavailable · default Test remains independent."))
                .when_some(state.support.as_ref().and_then(|support| support.as_ref().err()), |v, error| v.child(text("test-support-error", error.clone())))
                .when(!available && self.client.is_some(), |v| v.child(Button::new("test-recheck-support").label("Recheck custom test support").disabled(busy)
                    .on_click(cx.listener(|this, _, _, cx| { this.probe_tests(); cx.notify(); }))))
                .when(state.stale, |v| v.child(text("test-context-stale", "Refresh context explicitly; arguments are retained. Preparation or external edits may have changed configuration.")))
                .when(watch_blocks(self.watch.as_ref()), |v| v.child(text("test-watch-block", "Watch is transitioning or unresponsive. Wait for a confirmed steady state. Testing never stops or switches Watch.")))
                .when_some(state.error.clone(), |v, error| v.child(div().text_color(cx.theme().danger).child(text("test-arguments-error", error))))
                .child(div().id(if stacked { "test-arguments-stacked" } else { "test-arguments-adjacent" }).test_support().flex().gap_4().min_w_0().when(stacked, |v| v.flex_col()).child(rows).child(review)))
            .child(div().h_flex().flex_wrap().gap_2().p_3().border_t_1().border_color(cx.theme().border)
                .child(Button::new("test-refresh-context").label("Refresh context").disabled(busy || !available || state.dialog)
                    .on_click(cx.listener(|this, _, w, cx| this.request_test_intent(Intent::Refresh, w, cx))))
                .child(Button::new("test-run-arguments").primary().label("Run with arguments").disabled(busy || !available || state.dialog || state.ready().is_err() || watch_blocks(self.watch.as_ref()))
                    .on_click(cx.listener(|this, _, _, cx| this.run_test_arguments(cx))))
                .child(Button::new("test-discard-draft").label("Discard draft").disabled(busy || state.dialog)
                    .on_click(cx.listener(|this, _, w, cx| this.request_test_intent(Intent::Discard(false), w, cx)))))
            .into_any_element()
    }
}
fn text(id: impl Into<SharedString>, content: impl Into<SharedString>) -> impl IntoElement {
    let content = content.into();
    div()
        .id(id.into())
        .test_support()
        .role(Role::Group)
        .aria_label(content.clone())
        .whitespace_normal()
        .child(content)
}
