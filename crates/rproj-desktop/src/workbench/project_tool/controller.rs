use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

impl Workbench {
    pub(in super::super) fn project_tool_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| {
            matches!(
                op,
                Operation::PrepareProjectTool | Operation::SetupProjectTool
            )
        })
    }
    pub(in super::super) fn probe_project_tool(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::ProjectToolSupport(
                client.project_tool_support().map_err(|e| format!("{e:#}")),
            ));
        });
    }
    fn focus_project_tool(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_tool.visible {
            self.project_tool_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn show_project_tool(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.browser.leave();
        self.source.visible = false;
        self.library.visible = false;
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.settings.visible = false;
        self.creation.visible = false;
        self.composition.visible = false;
        self.upgrade.visible = false;
        self.project_tool.visible = true;
        self.test_args.visible = false;
        self.focus_project_tool(window, cx);
        cx.notify();
    }
    fn finish_project_tool_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.project_tool.dialog_open = false;
        self.project_tool.close_pending = false;
        self.focus_project_tool(window, cx);
        cx.notify();
    }
    pub(in super::super) fn edit_project_tool(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.project_tool.project.as_ref() == Some(&project) {
            self.show_project_tool(window, cx);
        } else if self.running.is_none()
            && !self.teardown_uncertain()
            && !self.project_tool.dialog_open
        {
            if self.project_tool.project.is_none() {
                self.capture_project_tool(project, window, cx);
                return;
            }
            self.project_tool.dialog_open = true;
            let revision = self.project_tool.revision;
            let previous = self.project_tool.project.clone();
            let name = self.project_tool.name();
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx, move |dialog, _, _| {
                let keep = owner.clone(); let replace = owner.clone(); let dismissed = owner.clone();
                let project = project.clone(); let previous = previous.clone();
                dialog.title("Keep the captured tool review?")
                    .child(format!("The review for {name} stays attached to its original project. Replace it with {}?", project.display()))
                    .footer(DialogFooter::new()
                        .child(Button::new("project-tool-keep-review").primary().label("Keep review").on_click(move |_, w, cx| {
                            w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx));
                        }))
                        .child(Button::new("project-tool-replace").label("Replace review").on_click(move |_, w, cx| {
                            w.close_dialog(cx); let _ = replace.update(cx, |this, cx| {
                                if this.project_tool.revision == revision && this.project_tool.project == previous && this.running.is_none() && !this.teardown_uncertain() {
                                    this.capture_project_tool(project.clone(), w, cx);
                                }
                                this.finish_project_tool_dialog(w, cx);
                            });
                        })))
                    .on_close(move |_, w, cx| { let _ = dismissed.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx)); })
            });
        }
    }
    fn capture_project_tool(
        &mut self,
        project: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let support = self.project_tool.support.take();
        let revision = self.project_tool.revision.wrapping_add(1);
        self.project_tool = Session {
            project: Some(project),
            tool: "asphalt".into(),
            revision,
            support,
            ..Default::default()
        };
        self.project_tool_query
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.show_project_tool(window, cx);
    }
    fn project_tool_operation(&mut self, operation: Operation) -> Option<(u64, WorkbenchClient)> {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.project_tool.support, Some(Ok(())))
        {
            return None;
        }
        let client = self.client.clone()?;
        let project = self.project_tool.project.clone()?;
        let id = self.operations.begin(operation, project.clone());
        self.pending_switch = None;
        self.activity.set_origin(Some(format!(
            "{} · {}",
            project.display(),
            operation_label(operation)
        )));
        self.running = Some((operation, Cancellation::default()));
        self.project_tool.loading = true;
        self.project_tool.completion_confirmed = false;
        Some((id, client))
    }
    fn project_tool_sink(&self, id: u64) -> impl Fn(OperationEvent) + Send + Sync + 'static {
        let sender = self.sender.clone();
        let dropped = self.dropped.clone();
        move |event| {
            if sender.try_send(Event::Progress(id, event)).is_err() {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    pub(super) fn prepare_project_tool(&mut self, cx: &mut Context<Self>) {
        if self.project_tool.dialog_open {
            return;
        }
        let Some((id, client)) = self.project_tool_operation(Operation::PrepareProjectTool) else {
            return;
        };
        self.project_tool.revision = self.project_tool.revision.wrapping_add(1);
        self.project_tool.plan = None;
        self.project_tool.result = None;
        self.project_tool.error = None;
        let revision = self.project_tool.revision;
        let project = self.project_tool.project.clone().unwrap();
        let tool = self.project_tool.tool.clone();
        let sender = self.sender.clone();
        let sink = self.project_tool_sink(id);
        std::thread::spawn(move || {
            let result = client
                .prepare_project_tool(&project, &tool, &Cancellation::default(), sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::ProjectToolPrepared(id, revision, result));
        });
        cx.notify();
    }
    pub(super) fn choose_project_tool_again(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.project_tool.dialog_open {
            return;
        }
        self.project_tool.dialog_open = true;
        let revision = self.project_tool.revision;
        let name = self.project_tool.name();
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let choose = owner.clone(); let dismissed = owner.clone();
            dialog.title("Keep this tool review?")
                .child(format!("Choosing another tool replaces the retained review/results for {name}. Existing project files stay unchanged until setup is explicitly confirmed."))
                .footer(DialogFooter::new()
                    .child(Button::new("project-tool-keep-choice").primary().label("Keep review").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx));
                    }))
                    .child(Button::new("project-tool-change-choice").label("Choose tool").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = choose.update(cx, |this, cx| {
                            if this.project_tool.revision == revision && this.running.is_none() && !this.teardown_uncertain() {
                                let tool = this.project_tool.tool.clone(); this.project_tool.choose(&tool);
                            }
                            this.finish_project_tool_dialog(w, cx);
                        });
                    })))
                .on_close(move |_, w, cx| { let _ = dismissed.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx)); })
        });
    }
    fn finish_project_tool_operation(
        &mut self,
        id: u64,
        outcome: &Result<OperationOutcome, String>,
    ) {
        self.operations.finish(id, outcome);
        self.running = None;
        self.project_tool.loading = false;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
    }
    pub(in super::super) fn accept_project_tool_prepared(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<Plan, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || !matches!(self.running, Some((Operation::PrepareProjectTool, _)))
        {
            return;
        }
        let result = result.and_then(|plan| {
            if self.project_tool.revision == revision && plan.tool == self.project_tool.tool {
                Ok(plan)
            } else {
                Err(Failure {
                    message: "Obsolete tool review; explicitly Refresh review.".into(),
                    teardown: false,
                })
            }
        });
        let outcome = result
            .as_ref()
            .map(|_| OperationOutcome::Succeeded)
            .map_err(|e| e.message.clone());
        self.finish_project_tool_operation(id, &outcome);
        match result {
            Ok(plan) => {
                self.project_tool.project = Some(plan.project.clone());
                self.project_tool.plan = Some(plan);
                self.project_tool.completion_confirmed = true;
            }
            Err(error) => {
                self.project_tool.teardown_uncertain |= error.teardown;
                self.project_tool.completion_confirmed = !error.teardown;
                self.project_tool.error = Some(error.message);
            }
        }
        self.finish_project_tool_close(cx);
        cx.notify();
    }
    pub(super) fn confirm_project_tool(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || self.project_tool.dialog_open
            || self.project_tool.watch_conflict(self.watch.as_ref())
        {
            return;
        }
        let Some(plan) = self.project_tool.ready().cloned() else {
            return;
        };
        let revision = self.project_tool.revision;
        let watch = self.watch_generation;
        self.project_tool.dialog_open = true;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let apply = owner.clone(); let dismissed = owner.clone();
            let plan = plan.clone();
            dialog.title("Set up tool?")
                .child(format!("Project: {}. Tool: {}. {} reviewed steps. Downloads, shared cache/trust changes and partial files may remain after failure. Existing pins/files are preserved. No rollback is promised.", plan.project.display(), plan.tool, plan.steps.len()))
                .footer(DialogFooter::new()
                    .child(Button::new("project-tool-keep-reviewing").primary().label("Keep reviewing").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx));
                    }))
                    .child(Button::new("project-tool-confirm").label("Set up tool").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = apply.update(cx, |this, cx| {
                            this.project_tool.dialog_open = false;
                            if this.project_tool.revision == revision && this.watch_generation == watch && this.project_tool.ready() == Some(&plan)
                                && !this.project_tool.watch_conflict(this.watch.as_ref()) { this.execute_project_tool(plan.clone(), cx); }
                            else { this.project_tool.error = Some("Project, review or Watch changed. Refresh review before setup.".into()); }
                            this.focus_project_tool(w, cx); cx.notify();
                        });
                    })))
                .on_close(move |_, w, cx| { let _ = dismissed.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx)); })
        });
    }
    fn execute_project_tool(&mut self, plan: Plan, cx: &mut Context<Self>) {
        let Some((id, client)) = self.project_tool_operation(Operation::SetupProjectTool) else {
            return;
        };
        let sender = self.sender.clone();
        let sink = self.project_tool_sink(id);
        std::thread::spawn(move || {
            let result = client.setup_project_tool(plan, sink).map_err(Failure::from);
            let _ = sender.send(Event::ProjectToolApplied(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_project_tool_applied(
        &mut self,
        id: u64,
        result: Result<Completion, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || !matches!(self.running, Some((Operation::SetupProjectTool, _)))
        {
            return;
        }
        let result = result.and_then(|result| {
            if Some(&result.project) == self.project_tool.project.as_ref()
                && result.tool == self.project_tool.tool
            {
                Ok(result)
            } else {
                Err(Failure {
                    message:
                        "Tool setup completion belongs to another project/tool. Refresh explicitly."
                            .into(),
                    teardown: false,
                })
            }
        });
        let outcome = match &result {
            Ok(r) if r.status == Status::Succeeded => Ok(OperationOutcome::Succeeded),
            Ok(r) => Err(r.message.clone().unwrap_or_else(|| {
                format!(
                    "Tool setup {:?}; inspect partial effects and Refresh review.",
                    r.status
                )
            })),
            Err(e) => Err(e.message.clone()),
        };
        self.finish_project_tool_operation(id, &outcome);
        match result {
            Ok(result) => {
                self.project_tool.completion_confirmed =
                    matches!(result.status, Status::Succeeded | Status::Failed);
                self.project_tool.result = Some(result);
            }
            Err(error) => {
                self.project_tool.teardown_uncertain |= error.teardown;
                self.project_tool.error = Some(error.message);
            }
        }
        self.finish_project_tool_close(cx);
        cx.notify();
    }
    fn finish_project_tool_close(&mut self, cx: &mut Context<Self>) {
        if self.project_tool.close_pending {
            self.project_tool.close_pending = false;
            if self.project_tool.completion_confirmed && !self.project_tool.teardown_uncertain {
                self.continue_close(cx);
            } else {
                self.operations.expanded = true;
            }
        }
    }
    pub(in super::super) fn project_tool_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.project_tool.dialog_open || self.project_tool.teardown_uncertain {
            return Some(false);
        }
        if !self.project_tool_running() {
            return None;
        }
        let id = self.operations.latest.as_ref().unwrap().id;
        let name = self
            .project_tool
            .project
            .as_ref()
            .unwrap()
            .display()
            .to_string();
        self.project_tool.dialog_open = true;
        // A new close decision supersedes an earlier Close after setup request.
        // Completion while this dialog is open must wait for the new decision.
        self.project_tool.close_pending = false;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let close = owner.clone(); let dismissed = owner.clone();
            dialog.title("Project tool setup is running")
                .child(format!("Project: {name}. Setup cannot be cancelled. Partial installations or files may remain. Closing waits for confirmed completion and teardown, then checks other drafts. Watch stays running."))
                .footer(DialogFooter::new()
                    .child(Button::new("project-tool-keep-open").primary().label("Keep open").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx));
                    }))
                    .child(Button::new("project-tool-close-after").label("Close after setup").on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = close.update(cx, |this, cx| {
                            this.project_tool.dialog_open = false;
                            if this.operations.accepts(id) && this.project_tool_running() { this.project_tool.close_pending = true; }
                            else if this.operations.latest.as_ref().is_some_and(|r| r.id == id) && this.project_tool.completion_confirmed && !this.project_tool.teardown_uncertain { this.continue_close(cx); }
                            else { this.project_tool.error = Some("Completion is unconfirmed. Review the result before closing.".into()); }
                            this.focus_project_tool(w, cx); cx.notify();
                        });
                    })))
                .on_close(move |_, w, cx| { let _ = dismissed.update(cx, |this, cx| this.finish_project_tool_dialog(w, cx)); })
        });
        Some(false)
    }
}
