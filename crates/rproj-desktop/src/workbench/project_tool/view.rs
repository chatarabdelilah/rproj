use super::*;
use gpui_kit::assets::IconName;
use gpui_kit::component::{button::*, input::Input};
use gpui_kit::prelude::FluentBuilder;

impl Workbench {
    pub(in super::super) fn project_tool_entry(&self, cx: &mut Context<Self>) -> AnyElement {
        let available = self.client.is_some() && matches!(self.project_tool.support, Some(Ok(())));
        div().v_flex().gap_2().px_6().py_2()
            .child(Button::new("project-tool-open").icon(IconName::Wrench).label("Set up project tool…")
                .disabled(!available || self.project.is_none() || self.running.is_some() || self.teardown_uncertain())
                .on_click(cx.listener(|this, _, w, cx| this.edit_project_tool(w, cx))))
            .when(self.project_tool.project.is_some(), |v| v.child(
                Button::new("project-tool-resume").label(format!("Resume tool setup for {}", self.project_tool.name()))
                    .on_click(cx.listener(|this, _, w, cx| this.show_project_tool(w, cx)))))
            .child(div().text_sm().child("Review one tool's project pin, missing files and manual next steps. Requires a Rojo project file; no rproj record or completed machine setup required."))
            .when(!available, |v| v.child(div().text_sm().child(self.project_tool.support.as_ref().and_then(|r| r.as_ref().err()).cloned()
                .unwrap_or_else(|| "Project Tool Setup needs a matching bundled CLI; checking availability…".into()))))
            .into_any_element()
    }
    pub(in super::super) fn render_project_tool(
        &self,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let state = &self.project_tool;
        let busy = self.running.is_some() || self.teardown_uncertain();
        let available = self.client.is_some() && matches!(state.support, Some(Ok(())));
        let captured = format!(
            "{} · {}",
            state
                .project
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            state.tool
        );
        let mut body = div().v_flex().gap_3().min_w_0();
        if let Some(result) = &state.result {
            let status = match result.status {
                Status::Succeeded => {
                    "Completed — inspect individual results and manual next steps."
                }
                Status::Failed => {
                    "Setup failed. Completed installation or file effects may remain. Refresh review before retrying."
                }
                _ => {
                    "Completion is uncertain. Partial effects may remain. Explicitly Refresh review before retrying."
                }
            };
            body = body
                .child(text("project-tool-result", status))
                .child(match result.effects {
                    WriteState::NotStarted => "No setup effects began.",
                    WriteState::Completed => "Reviewed setup effects completed.",
                    _ => "Setup effects may have begun; no rollback is promised.",
                });
            if let Some(message) = &result.message {
                body = body.child(message.clone());
            }
            if let Some(outcome) = &result.outcome {
                for (index, step) in outcome.steps.iter().enumerate() {
                    let label = match step.state {
                        StepState::Completed => "Completed",
                        StepState::AlreadyPresent => "Already present",
                        StepState::ManualAction => "Manual action required",
                        StepState::Failed => "Failed",
                        StepState::Skipped => "Skipped",
                    };
                    body = body.child(text(
                        format!("project-tool-result-{index}"),
                        format!("{label}: {}. {}", step.label, step.detail),
                    ));
                }
            }
        } else if let Some(plan) = &state.plan {
            body = body.child(text(
                "project-tool-review",
                "Review — ordered effects from the captured project",
            ));
            for prerequisite in &plan.prerequisites {
                body = body.child(format!("Prerequisite: {prerequisite}"));
            }
            for (index, step) in plan.steps.iter().enumerate() {
                body = body.child(text(
                    format!("project-tool-step-{index}"),
                    format!("{}. {}", index + 1, step.label()),
                ));
                if let Step::CreateFile { contents, .. } = step {
                    body = body.child(
                        div()
                            .id(SharedString::from(format!("project-tool-file-{index}")))
                            .role(Role::Group)
                            .aria_label(contents.clone())
                            .test_support()
                            .p_3()
                            .rounded_lg()
                            .bg(cx.theme().muted)
                            .child(div().whitespace_normal().child(contents.clone())),
                    );
                }
            }
            for note in &plan.notes {
                body = body.child(note.clone());
            }
            body = body.child("No version is resolved until installation. Existing pins are preserved, not updated. File creation does not prove tool installation succeeded. Credentials and creator IDs are configured manually; setup uploads no assets.");
        } else {
            let query = self.project_tool_query.read(cx).value();
            let matches = choices(&query);
            let list = div()
                .id("project-tool-choices")
                .max_h(px(if stacked { 200. } else { 400. }))
                .overflow_y_scroll()
                .v_flex()
                .gap_2()
                .min_w_0()
                .when(!stacked, |v| v.w(px(230.)).flex_none())
                .child(
                    Input::new(&self.project_tool_query)
                        .id("project-tool-search")
                        .aria_label("Search project tools"),
                )
                .when(matches.is_empty(), |v| {
                    v.child("No matching tools. The selected tool remains unchanged.")
                })
                .children(matches.into_iter().map(|tool| {
                    Button::new(SharedString::from(format!(
                        "project-tool-choice-{}",
                        tool.key
                    )))
                    .label(format!("{} · {}", tool.key, tool.kind.label()))
                    .selected(state.tool == tool.key)
                    .disabled(busy || state.dialog_open)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.running.is_none()
                            && !this.teardown_uncertain()
                            && !this.project_tool.dialog_open
                        {
                            this.project_tool.choose(tool.key);
                            cx.notify();
                        }
                    }))
                }));
            let mut detail = div()
                .id("project-tool-detail")
                .v_flex()
                .gap_3()
                .flex_1()
                .min_w_0();
            if let Some(tool) = tool_catalog::find(&state.tool) {
                detail = detail
                    .child(text(
                        "project-tool-selected",
                        format!("{} · {}", tool.key, scope(tool)),
                    ))
                    .child(tool.description)
                    .child(format!("Source: {}", tool.kind.provider()))
                    .child(
                        Button::new("project-tool-docs")
                            .label("Documentation")
                            .on_click(move |_, _, cx| cx.open_url(tool.docs_url)),
                    );
                if tool.key == "blender" {
                    detail = detail.child("Creates a missing blender/scene.blend using an installed Blender executable. An existing scene is preserved. This does not install Blender.");
                } else if !matches!(tool.kind, ToolKind::RokitTool { .. }) {
                    detail = detail.child("Guidance only in this project. Installation belongs to Machine Setup; this flow will not install this app, extension or plugin.")
                        .child(Button::new("project-tool-machine-setup").label("Machine Setup…").on_click(cx.listener(|this, _, w, cx| this.show_setup(w, cx))));
                } else {
                    detail = detail.child("A missing pin requires Rokit. Review preserves existing pins and files; missing files and external trust/download effects are shown before setup.");
                }
            }
            body = body.child(
                div()
                    .id(if stacked {
                        "project-tool-stacked"
                    } else {
                        "project-tool-adjacent"
                    })
                    .test_support()
                    .flex()
                    .gap_4()
                    .min_w_0()
                    .when(stacked, |v| v.flex_col())
                    .child(list)
                    .child(detail),
            );
        }
        let watch_conflict = state.watch_conflict(self.watch.as_ref());
        div().id("project-tool-screen").test_support().track_focus(&self.project_tool_focus).size_full().min_h_0().v_flex()
            .child(div().id("project-tool-scroll").test_support().flex_1().min_h_0().overflow_y_scroll().p_6().v_flex().gap_3()
                .child(text("project-tool-captured", captured))
                .child("Tool → Review → Results · Captured project; browsing another project never retargets this review.")
                .when(state.loading, |v| v.child(text("project-tool-loading", "Working on captured tool setup…")))
                .when_some(state.error.clone(), |v, error| v.child(div().text_color(cx.theme().danger).child(text("project-tool-error", format!("{error} Refresh review explicitly to retry.")))))
                .when(watch_conflict, |v| v.child(text("project-tool-watch-conflict", "Watch owns this project. Use the separate Stop control and wait for acknowledgement before setup.")))
                .when(!available, |v| v.child("Execution is unavailable. Use a matching desktop/CLI distribution; other workflows remain available."))
                .child(body))
            .child(div().h_flex().flex_wrap().gap_2().p_3().border_t_1().border_color(cx.theme().border)
                .child(Button::new("project-tool-back").label("Choose tool").disabled(busy || state.dialog_open || (state.plan.is_none() && state.result.is_none()))
                    .on_click(cx.listener(|this, _, w, cx| this.choose_project_tool_again(w, cx))))
                .child(Button::new("project-tool-prepare").label(if state.plan.is_some() || state.result.is_some() || state.error.is_some() { "Refresh review" } else { "Review setup" })
                    .disabled(busy || !available || state.dialog_open).on_click(cx.listener(|this, _, _, cx| this.prepare_project_tool(cx))))
                .child(Button::new("project-tool-apply").primary().label("Set up tool…")
                    .disabled(busy || !available || state.ready().is_none() || watch_conflict || state.dialog_open)
                    .on_click(cx.listener(|this, _, w, cx| this.confirm_project_tool(w, cx)))))
            .into_any_element()
    }
}

fn text(id: impl Into<SharedString>, content: impl Into<SharedString>) -> impl IntoElement {
    let content = content.into();
    div()
        .id(id.into())
        .role(Role::Group)
        .aria_label(content.clone())
        .test_support()
        .child(content)
}
