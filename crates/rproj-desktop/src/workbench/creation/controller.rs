use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};
use rproj_core::creation::PackageWorkflow;

impl Workbench {
    pub(in super::super) fn load_creation_context(&mut self, cx: &mut Context<Self>) {
        if self.creation.loading || self.creation_running() {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        self.creation.context_id = self.creation.context_id.wrapping_add(1);
        let id = self.creation.context_id;
        self.creation.loading = true;
        self.creation.prepared = None;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = client.creation_context().map_err(|e| {
                format!(
                    "New Project is unavailable: {e:#}. Use a matching portable CLI and Recheck."
                )
            });
            let _ = sender.send(Event::CreationContext(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_creation_context(
        &mut self,
        id: u64,
        result: Result<CreationContext, String>,
    ) {
        if id != self.creation.context_id {
            return;
        }
        self.creation.loading = false;
        self.creation.prepared = None;
        match result {
            Ok(context) => {
                self.creation.context = Some(context);
                self.creation.error = None;
            }
            Err(error) => {
                self.creation.context = None;
                self.creation.error = Some(error);
            }
        }
    }
    pub(in super::super) fn show_creation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.creation.draft.is_none() {
            self.creation.draft = Some(CreationDraft::new(String::new()));
            self.creation.step = Step::Project;
        }
        self.creation.visible = true;
        self.focus_creation(window, cx);
        cx.notify();
    }
    pub(in super::super) fn creation_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| {
            matches!(op, Operation::PrepareCreation | Operation::CreateProject)
        })
    }
    pub(in super::super) fn focus_creation(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.creation.step == Step::Project {
            self.creation_inputs
                .name
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        } else {
            self.creation_inputs.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn mutate_creation(
        &mut self,
        change: impl FnOnce(&mut CreationDraft) -> anyhow::Result<()>,
        cx: &mut Context<Self>,
    ) {
        if self.creation_running() || self.creation.result.is_some() {
            return;
        }
        let Some(draft) = self.creation.draft.as_mut() else {
            return;
        };
        match change(draft) {
            Ok(()) => self.creation.changed(),
            Err(error) => self.creation.error = Some(error.to_string()),
        }
        cx.notify();
    }
    pub(in super::super) fn confirm_workflow(
        &mut self,
        workflow: PackageWorkflow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = &self.creation.draft else {
            return;
        };
        if draft.graph.package_workflow == workflow || self.creation_running() {
            return;
        }
        let revision = self.creation.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let owner = owner.clone();
            dialog.confirm().title("Change dependency workflow?")
                .child("Package choices and optional file exclusions will be recomputed. Testing stays selected; resolve any incompatible runner explicitly in Capabilities.")
                .button_props(dialog::DialogButtonProps::default().ok_text("Change workflow").cancel_text("Keep workflow"))
                .on_ok(move |_, _, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if this.creation.revision == revision {
                            this.mutate_creation(|draft| { draft.set_workflow(workflow); Ok(()) }, cx);
                        }
                    }); true
                })
        });
    }
    pub(in super::super) fn creation_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some() {
            return;
        }
        if self.creation.step == Step::Review {
            self.prepare_creation(cx);
            return;
        }
        if matches!(self.creation.step, Step::Project | Step::Capabilities)
            && let Err(error) = self.creation.request()
        {
            self.creation.error = Some(error);
            cx.notify();
            return;
        }
        self.creation.step = Step::ALL[self.creation.step.index() + 1];
        if self.creation.step == Step::Review {
            self.prepare_creation(cx);
        }
        self.focus_creation(window, cx);
        cx.notify();
    }
    fn creation_operation(
        &mut self,
        operation: Operation,
    ) -> Option<(u64, Cancellation, WorkbenchClient)> {
        if self.running.is_some() || self.creation.teardown_uncertain {
            return None;
        }
        let client = self.client.clone()?;
        let destination = self.creation.destination()?;
        self.pending_switch = None;
        let cancellation = Cancellation::default();
        let id = self.operations.begin(operation, destination.clone());
        self.activity.set_origin(Some(format!(
            "{} · {}",
            destination.display(),
            operation_label(operation)
        )));
        self.running = Some((operation, cancellation.clone()));
        Some((id, cancellation, client))
    }
    fn prepare_creation(&mut self, cx: &mut Context<Self>) {
        let request = match self.creation.request() {
            Ok(request) => request,
            Err(error) => {
                self.creation.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some((id, cancellation, client)) = self.creation_operation(Operation::PrepareCreation)
        else {
            return;
        };
        let revision = self.creation.revision;
        self.creation.error = None;
        self.creation.prepared = None;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .prepare_creation(request, &cancellation, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::Prepared(id, revision, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_preparation(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<PreparationOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let outcome = match &result {
            Ok(PreparationOutcome::Prepared(_)) => Ok(OperationOutcome::Succeeded),
            Ok(PreparationOutcome::Cancelled) => Ok(OperationOutcome::Cancelled),
            Err(error) => Err(error.clone()),
        };
        self.finish_creation_operation(id, &outcome);
        if revision == self.creation.revision {
            match &result {
                Ok(PreparationOutcome::Prepared(prepared)) => {
                    self.creation.draft = Some(prepared.plan.draft.clone());
                    self.creation.prepared = Some((revision, *prepared.clone()));
                    self.creation.error = None;
                }
                Ok(PreparationOutcome::Cancelled) => {
                    self.creation.error =
                        Some("Preparation cancelled. No destination was created.".into())
                }
                Err(error) => {
                    self.creation.error = Some(format!(
                        "Review could not be prepared: {error}. Correct the issue and prepare again; Recheck if configuration changed."
                    ))
                }
            }
        }
        self.finish_requested_close(result.is_ok(), cx);
    }
    pub(in super::super) fn confirm_creation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.creation.result.is_some() {
            return;
        }
        let Some(prepared) = self.creation.fresh_plan().cloned() else {
            return;
        };
        let revision = self.creation.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let owner = owner.clone();
            let prepared = prepared.clone();
            dialog.confirm().title("Create this project?")
                .child(format!("Destination: {}. rproj will initialize Git and run the reviewed tool and dependency operations. Failure or cancellation can leave partial files.", prepared.plan.destination.display()))
                .button_props(dialog::DialogButtonProps::default().ok_text("Create project").cancel_text("Keep reviewing"))
                .on_ok(move |_, _, cx| {
                    let _ = owner.update(cx, |this, cx| {
                        if this.creation.revision == revision && this.creation.fresh_plan() == Some(&prepared) {
                            this.execute_creation(prepared.clone(), cx);
                        } else { this.creation.error = Some("The reviewed choices changed. Prepare Review again.".into()); cx.notify(); }
                    }); true
                })
        });
    }
    fn execute_creation(&mut self, prepared: PreparedCreation, cx: &mut Context<Self>) {
        let Some((id, cancellation, client)) = self.creation_operation(Operation::CreateProject)
        else {
            return;
        };
        self.creation.prepared = None;
        self.creation.error = None;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .create_project(prepared, &cancellation, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::Created(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_creation_result(
        &mut self,
        id: u64,
        result: Result<CreationOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let outcome = match &result {
            Ok(result) => match result.status {
                CreationStatus::Succeeded => Ok(OperationOutcome::Succeeded),
                CreationStatus::Failed => Err(result.message.clone().unwrap_or_else(|| {
                    "Creation failed; inspect the destination for partial files.".into()
                })),
                CreationStatus::Cancelled => Ok(OperationOutcome::Cancelled),
            },
            Err(error) => Err(format!(
                "Creation outcome is uncertain; partial files may exist. {error}"
            )),
        };
        self.finish_creation_operation(id, &outcome);
        if let Ok(result) = &result {
            self.activity.append(&format!(
                "Destination: {} · {:?}. {}",
                result.destination.display(),
                result.claim,
                result.message.as_deref().unwrap_or("")
            ));
        }
        self.creation.result = Some(result.clone());
        self.creation.dirty = false;
        self.finish_requested_close(result.is_ok(), cx);
    }
    fn finish_creation_operation(&mut self, id: u64, result: &Result<OperationOutcome, String>) {
        self.operations.finish(id, result);
        self.running = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
    }
    fn finish_requested_close(&mut self, teardown_confirmed: bool, cx: &mut Context<Self>) {
        if self.creation.close_pending {
            self.creation.close_pending = false;
            if teardown_confirmed {
                self.close_after_teardown(cx);
            } else {
                self.creation.teardown_uncertain = true;
                self.operations.expanded = true;
                self.creation.error = Some("The window stayed open because worker teardown could not be confirmed. Inspect Activity and the destination; partial files may exist.".into());
            }
        }
        cx.notify();
    }
    fn close_after_teardown(&self, cx: &mut Context<Self>) {
        let handle = self.window;
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        });
    }
    fn reset_creation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.creation = CreationSession {
            context: self.creation.context.take(),
            context_id: self.creation.context_id,
            loading: self.creation.loading,
            ..Default::default()
        };
        self.creation_inputs
            .name
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.creation_inputs
            .setup
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.creation_inputs
            .query
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.focus.focus(window, cx);
        cx.notify();
    }
    pub(in super::super) fn discard_creation(
        &mut self,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.creation_running() || self.creation.close_dialog || self.creation.teardown_uncertain
        {
            return;
        }
        if !self.creation.dirty {
            self.reset_creation(window, cx);
            if closing {
                self.close_after_teardown(cx);
            }
            return;
        }
        self.creation.close_dialog = true;
        let revision = self.creation.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let discard = owner.clone(); let closed = owner.clone();
            dialog.title("Discard the project draft?").child("The unsubmitted choices are session-only. Existing projects and detached Watch stay unchanged.")
                .footer(DialogFooter::new()
                    .child(Button::new("keep-draft").primary().label("Keep draft").on_click(move |_, window, cx| { window.close_dialog(cx); let _ = keep.update(cx, |this, _| this.creation.close_dialog = false); }))
                    .child(Button::new("discard-draft").label("Discard draft").on_click(move |_, window, cx| {
                        let _ = discard.update(cx, |this, cx| {
                            if this.creation.revision == revision && !this.creation_running() {
                                this.reset_creation(window, cx);
                                if closing { this.close_after_teardown(cx); }
                                cx.notify();
                            }
                        }); window.close_dialog(cx);
                    })))
                .on_close(move |_, _, cx| { let _ = closed.update(cx, |this, _| this.creation.close_dialog = false); })
        });
    }
    pub(in super::super) fn request_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.creation.teardown_uncertain {
            self.operations.expanded = true;
            cx.notify();
            return false;
        }
        if self.creation.close_pending || self.creation.close_dialog {
            return false;
        }
        if self.creation_running() {
            self.creation.close_dialog = true;
            let destination = self
                .operations
                .latest
                .as_ref()
                .unwrap()
                .project
                .display()
                .to_string();
            let id = self.operations.latest.as_ref().unwrap().id;
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx, move |dialog, _, _| {
                let cancel = owner.clone(); let closed = owner.clone(); let keep = owner.clone();
                dialog.title("Creation is still running")
                    .child(format!("Destination: {destination}. Cancellation may leave partial files. The window closes only after the worker and its children exit. Detached Watch stays running."))
                    .footer(DialogFooter::new()
                        .child(Button::new("keep-open").primary().label("Keep open").on_click(move |_, window, cx| {
                            let _ = keep.update(cx, |this, _| this.creation.close_dialog = false);
                            window.close_dialog(cx);
                        }))
                        .child(Button::new("cancel-creation-close").label("Cancel creation and close").on_click(move |_, window, cx| {
                            let _ = cancel.update(cx, |this, cx| {
                                this.creation.close_dialog = false;
                                if this.operations.accepts(id) && this.creation_running() {
                                    this.creation.close_pending = true;
                                    if let Some((_, token)) = &this.running { token.cancel(); }
                                    this.operations.cancel();
                                } else { this.creation.error = Some("Creation finished while this dialog was open. Review its result before closing.".into()); }
                                cx.notify();
                            }); window.close_dialog(cx);
                        })))
                    .on_close(move |_, _, cx| { let _ = closed.update(cx, |this, _| this.creation.close_dialog = false); })
            });
            false
        } else if self.creation.dirty {
            self.discard_creation(true, window, cx);
            false
        } else {
            true
        }
    }
}
