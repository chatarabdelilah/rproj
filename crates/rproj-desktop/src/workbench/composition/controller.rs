use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};
use gpui_kit::prelude::FluentBuilder;

impl Workbench {
    pub(in super::super) fn composition_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| {
            matches!(
                op,
                Operation::PrepareComposition | Operation::ApplyComposition
            )
        })
    }
    pub(in super::super) fn focus_composition(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.composition.visible {
            self.composition_inputs.focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn show_composition(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.browser.leave();
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.creation.visible = false;
        self.settings.visible = false;
        self.upgrade.visible = false;
        self.composition.visible = true;
        self.focus_composition(window, cx);
        cx.notify();
    }
    pub(in super::super) fn edit_composition(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.composition.loading {
            return;
        }
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.composition.project.as_ref() == Some(&project) {
            self.show_composition(window, cx);
        } else if self.composition.dirty || self.composition.result.is_some() {
            self.confirm_composition_reset(Some(project), false, window, cx);
        } else if !self.composition_running() && !self.composition.teardown_uncertain {
            self.load_composition(project, window, cx);
        }
    }
    fn load_composition(&mut self, project: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.composition_running()
            || self.composition.teardown_uncertain
            || self.composition.loading
        {
            return;
        }
        let Some(client) = self.client.clone() else {
            self.composition.error =
                Some("Connect to the matching bundled CLI before editing composition".into());
            cx.notify();
            return;
        };
        let id = self.composition.begin_load(project.clone());
        self.creation.visible = false;
        self.settings.visible = false;
        self.upgrade.visible = false;
        self.composition_inputs
            .query
            .update(cx, |input, cx| input.set_value("", window, cx));
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = client
                .composition_context(&project)
                .and_then(|context| {
                    let snapshot = rproj_core::inspect(&context.project)?;
                    Ok((context, snapshot))
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::CompositionContext(id, Box::new(result)));
        });
        self.focus_composition(window, cx);
        cx.notify();
    }
    pub(in super::super) fn mutate_composition(
        &mut self,
        change: impl FnOnce(&mut CompositionDraft) -> anyhow::Result<()>,
        cx: &mut Context<Self>,
    ) {
        if self.composition_running()
            || self.composition.result.is_some()
            || self.composition.teardown_uncertain
        {
            return;
        }
        let Some(draft) = self.composition.draft.as_mut() else {
            return;
        };
        match change(draft) {
            Ok(()) => self.composition.changed(),
            Err(e) => self.composition.error = Some(e.to_string()),
        }
        cx.notify();
    }
    pub(in super::super) fn composition_next(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.composition.result.is_some() {
            return;
        }
        if let Err(error) = self.composition.ready() {
            self.composition.error = Some(error);
            cx.notify();
            return;
        }
        if self.composition.step != Step::Review {
            self.composition.step = Step::ALL[self.composition.step.index() + 1];
        }
        if self.composition.step == Step::Review {
            self.prepare_composition(cx);
        }
        self.focus_composition(window, cx);
        cx.notify();
    }
    fn composition_operation(
        &mut self,
        operation: Operation,
    ) -> Option<(u64, Cancellation, WorkbenchClient)> {
        if self.running.is_some() || self.teardown_uncertain() {
            return None;
        }
        let project = self.composition.project.clone()?;
        let client = self.client.clone()?;
        let cancellation = Cancellation::default();
        let id = self.operations.begin(operation, project.clone());
        self.pending_switch = None;
        self.activity.set_origin(Some(format!(
            "{} · {}",
            project.display(),
            operation_label(operation)
        )));
        self.running = Some((operation, cancellation.clone()));
        Some((id, cancellation, client))
    }
    fn prepare_composition(&mut self, cx: &mut Context<Self>) {
        let request = match self.composition.request() {
            Ok(request) => request,
            Err(error) => {
                self.composition.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some((id, token, client)) = self.composition_operation(Operation::PrepareComposition)
        else {
            return;
        };
        self.composition.prepared = None;
        self.composition.error = None;
        let revision = self.composition.revision;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .prepare_composition(request, &token, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::CompositionPrepared(id, revision, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_composition_prepared(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<CompositionPreparation, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let teardown_confirmed = result.is_ok();
        if !teardown_confirmed {
            self.composition.teardown_uncertain = true;
        }
        let outcome = match &result {
            Ok(CompositionPreparation::Prepared(_)) => Ok(OperationOutcome::Succeeded),
            Ok(CompositionPreparation::Failed(error)) => Err(error.clone()),
            Ok(CompositionPreparation::Cancelled) => Ok(OperationOutcome::Cancelled),
            Err(e) => Err(e.clone()),
        };
        self.finish_composition_operation(id, &outcome);
        if self.composition.revision == revision {
            match result {
                Ok(CompositionPreparation::Prepared(plan)) => {
                    self.composition.prepared = Some((revision, *plan));
                    self.composition.error = None;
                }
                Ok(CompositionPreparation::Cancelled) => {
                    self.composition.error =
                        Some("Preparation cancelled; preparation made no project changes".into())
                }
                Ok(CompositionPreparation::Failed(e)) => {
                    self.composition.error = Some(format!(
                        "Review could not be prepared: {e}. Changed source files require explicit Reload; missing tools can be installed with rproj setup, then prepare again."
                    ))
                }
                Err(e) => {
                    self.composition.error = Some(format!(
                        "Worker teardown is unconfirmed: {e}. Closing and new operations are disabled because owned processes may still be running. Inspect Activity; this window must stay open."
                    ));
                }
            }
        }
        self.finish_composition_close(teardown_confirmed, cx);
        cx.notify();
    }
    pub(in super::super) fn confirm_composition_apply(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some()
            || self.composition.watch_conflict(self.watch.as_ref())
            || !self.composition.dirty
        {
            return;
        }
        let Some(plan) = self.composition.fresh_plan().cloned() else {
            return;
        };
        let revision = self.composition.revision;
        let watch_generation = self.watch_generation;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let apply = owner.clone(); let keep = owner.clone(); let closed = owner.clone(); let plan = plan.clone();
            dialog.title("Apply packages and capabilities?")
                .child(format!("Project: {}. Apply only the reviewed additions and generated configuration refresh. Downloads may run; failure or cancellation can leave partial changes. No rollback is promised. Watch is never switched.", plan.project.display()))
                .footer(DialogFooter::new()
                    .child(Button::new("composition-keep-reviewing").primary().label("Keep reviewing").on_click(move |_, window, cx| {
                        window.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.focus_composition(window, cx));
                    }))
                    .child(Button::new("composition-confirm-apply").label("Apply additions").on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        let _ = apply.update(cx, |this, cx| {
                            if this.composition.revision == revision && this.watch_generation == watch_generation && !this.composition.watch_conflict(this.watch.as_ref()) && this.composition.fresh_plan() == Some(&plan) {
                                this.apply_composition(plan.clone(), cx);
                            } else { this.composition.prepared = None; this.composition.error = Some("The reviewed draft or Watch state changed; prepare Review again".into()); cx.notify(); }
                            this.focus_composition(window, cx);
                        });
                    })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| this.focus_composition(window, cx)); })
        });
    }
    fn apply_composition(&mut self, plan: CompositionPlan, cx: &mut Context<Self>) {
        let Some((id, token, client)) = self.composition_operation(Operation::ApplyComposition)
        else {
            return;
        };
        self.composition.prepared = None;
        self.composition.error = None;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .apply_composition(plan, &token, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::CompositionApplied(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_composition_result(
        &mut self,
        id: u64,
        result: Result<CompositionOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let owner = self.operations.latest.as_ref().unwrap().project.clone();
        let result = result.and_then(|result| if result.project == owner { Ok(result) } else { Err("Composition result belongs to another project; outcome/teardown are unconfirmed".into()) })
            .map_err(|error| format!("Worker teardown is unconfirmed: {error}. Closing and new operations are disabled because owned processes may still be running. Inspect Activity; this window must stay open."));
        let outcome = match &result {
            Ok(result) => match result.status {
                CompositionStatus::Succeeded => Ok(OperationOutcome::Succeeded),
                CompositionStatus::Cancelled => Ok(OperationOutcome::Cancelled),
                CompositionStatus::Failed | CompositionStatus::Uncertain => {
                    Err(result.message.clone().unwrap_or_else(|| {
                        format!(
                            "Composition {:?}; inspect {} for partial changes",
                            result.status,
                            owner.display()
                        )
                    }))
                }
            },
            Err(error) => Err(format!(
                "Composition outcome/teardown are unconfirmed: {error}"
            )),
        };
        self.activity.append(&format!(
            "Captured project: {}. {}",
            owner.display(),
            result
                .as_ref()
                .ok()
                .map(|r| format!(
                    "{}; {}. {}",
                    outcome_label(r.status),
                    writes_label(r.writes),
                    r.message.as_deref().unwrap_or("")
                ))
                .unwrap_or_else(|| {
                    "Inspect this project for possible partial changes; success was not confirmed"
                        .into()
                })
        ));
        self.finish_composition_operation(id, &outcome);
        self.composition.result = Some(result.clone());
        if result.is_err() {
            self.composition.teardown_uncertain = true;
        }
        self.composition.dirty = false;
        if self.project.as_ref() == Some(&owner) {
            self.inspect();
        }
        self.poll_status();
        self.finish_composition_close(result.is_ok(), cx);
        cx.notify();
    }
    fn finish_composition_operation(&mut self, id: u64, result: &Result<OperationOutcome, String>) {
        self.operations.finish(id, result);
        self.running = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
    }
    pub(in super::super) fn reload_composition(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.composition.project.clone() else {
            return;
        };
        if self.composition.dirty || self.composition.result.is_some() {
            self.confirm_composition_reset(Some(project), false, window, cx);
        } else {
            self.load_composition(project, window, cx);
        }
    }
    pub(in super::super) fn discard_composition(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.composition.loading {
            return;
        }
        if self.composition.dirty {
            self.confirm_composition_reset(None, false, window, cx);
        } else if !self.composition_running() && !self.composition.teardown_uncertain {
            self.composition.reset();
            self.focus.focus(window, cx);
            cx.notify();
        }
    }
    fn confirm_composition_reset(
        &mut self,
        replace: Option<PathBuf>,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.composition.dialog_open
            || self.composition_running()
            || self.composition.teardown_uncertain
        {
            return;
        }
        self.composition.dialog_open = true;
        let revision = self.composition.revision;
        let project = self.composition.project.clone();
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let discard = owner.clone(); let closed = owner.clone(); let replace = replace.clone();
            dialog.title(if replace.is_some() { "Replace the captured composition draft?" } else { "Discard the composition draft?" })
                .child(format!("Draft for {}. Reload/replacement discards unsubmitted additions; it never merges source edits or rolls back partial changes. Activity output stays available.", project.as_ref().map(|p| p.display().to_string()).unwrap_or_default()))
                .when_some(replace.as_ref(), |dialog, target| dialog.child(format!("New capture: {}", target.display())))
                .footer(DialogFooter::new()
                    .child(Button::new("composition-keep-draft").primary().label("Keep draft").on_click(move |_, window, cx| {
                        window.close_dialog(cx); let _ = keep.update(cx, |this, cx| { this.composition.dialog_open = false; this.focus_composition(window, cx); });
                    }))
                    .child(Button::new("composition-discard-draft").label(if replace.is_some() { "Discard and reload" } else { "Discard draft" }).on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        let _ = discard.update(cx, |this, cx| {
                            this.composition.dialog_open = false;
                            if this.composition.revision != revision || this.composition_running() { return; }
                            if let Some(project) = &replace { this.load_composition(project.clone(), window, cx); }
                            else { this.composition.reset(); this.focus.focus(window, cx); if closing && this.request_close(window, cx) { this.close_after_teardown(cx); } cx.notify(); }
                        });
                    })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| { this.composition.dialog_open = false; this.focus_composition(window, cx); }); })
        });
    }
    pub(in super::super) fn composition_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.composition.teardown_uncertain
            || self.composition.close_pending
            || self.composition.dialog_open
        {
            return Some(false);
        }
        if self.composition_running() {
            self.composition.dialog_open = true;
            let id = self.operations.latest.as_ref().unwrap().id;
            let project = self
                .operations
                .latest
                .as_ref()
                .unwrap()
                .project
                .display()
                .to_string();
            let applying = self
                .running
                .as_ref()
                .is_some_and(|(op, _)| *op == Operation::ApplyComposition);
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx, move |dialog, _, _| {
                let keep = owner.clone(); let cancel = owner.clone(); let closed = owner.clone();
                dialog.title(if applying { "Project changes are still running" } else { "Composition preparation is still running" })
                    .child(format!("Project: {project}. {} The window waits for worker/descendant teardown. Detached Watch stays running; other dirty drafts still require discard confirmation.", if applying { "Cancellation may leave partial changes; no rollback is promised." } else { "Preparation makes no project changes." }))
                    .footer(DialogFooter::new()
                        .child(Button::new("composition-keep-open").primary().label("Keep open").on_click(move |_, window, cx| {
                            window.close_dialog(cx); let _ = keep.update(cx, |this, _| this.composition.dialog_open = false);
                        }))
                        .child(Button::new("composition-cancel-close").label(if applying { "Cancel changes and close" } else { "Cancel preparation and close" }).on_click(move |_, window, cx| {
                            window.close_dialog(cx);
                            let _ = cancel.update(cx, |this, cx| {
                                this.composition.dialog_open = false;
                                if this.operations.accepts(id) && this.composition_running() {
                                    this.composition.close_pending = true;
                                    if let Some((_, token)) = &this.running { token.cancel(); }
                                    this.operations.cancel();
                                } else { this.composition.error = Some("The operation finished while confirmation was open; review its result before closing".into()); }
                                cx.notify();
                            });
                        })))
                    .on_close(move |_, _, cx| { let _ = closed.update(cx, |this, _| this.composition.dialog_open = false); })
            });
            return Some(false);
        }
        if self.composition.dirty {
            self.confirm_composition_reset(None, true, window, cx);
            return Some(false);
        }
        None
    }
    fn finish_composition_close(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if self.composition.close_pending {
            self.composition.close_pending = false;
            if confirmed {
                self.continue_close(cx);
            } else {
                self.composition.teardown_uncertain = true;
                self.operations.expanded = true;
                self.composition.error = Some("Worker teardown is unconfirmed. The window stays open; inspect Activity and the captured project for partial changes.".into());
            }
        }
    }
    pub(in super::super) fn continue_close(&self, cx: &mut Context<Self>) {
        let handle = self.window;
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = handle.update(cx, |_, window, cx| {
                let _ = owner.update(cx, |this, cx| {
                    if this.request_close(window, cx) {
                        window.remove_window();
                    }
                });
            });
        });
    }
}
