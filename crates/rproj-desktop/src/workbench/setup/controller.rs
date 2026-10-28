use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

#[derive(Clone, Copy)]
pub(super) enum Replace {
    Reload,
    Retry,
    Discard(bool),
}

impl Workbench {
    pub(in super::super) fn setup_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| op.is_setup())
    }
    pub(in super::super) fn setup_applying(&self) -> bool {
        matches!(self.running, Some((Operation::ApplySetup, _)))
    }
    pub(in super::super) fn setup_stop_requested(&self) -> bool {
        self.setup.stop.as_ref().is_some_and(SetupStop::requested)
    }
    pub(in super::super) fn probe_setup(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::SetupSupport(
                client.setup_support().map_err(|e| format!("{e:#}")),
            ));
        });
    }
    fn focus_setup(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.setup.visible {
            self.setup_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn show_setup(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.saved.visible = false;
        self.creation.visible = false;
        self.composition.visible = false;
        self.upgrade.visible = false;
        self.settings.visible = false;
        self.setup.visible = true;
        if self.setup.context.is_none() && !self.setup.loading && self.setup.error.is_none() {
            self.load_setup(cx);
        }
        self.focus_setup(window, cx);
        cx.notify();
    }
    fn setup_operation(&mut self, operation: Operation) -> Option<(u64, WorkbenchClient)> {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.setup.support, Some(Ok(())))
        {
            return None;
        }
        let client = self.client.clone()?;
        let id = self.operations.begin(
            operation,
            self.setup
                .context
                .as_ref()
                .map(|c| c.config_path.clone())
                .unwrap_or_default(),
        );
        self.pending_switch = None;
        self.activity.set_origin(Some(format!(
            "Machine Setup · {}",
            operation_label(operation)
        )));
        // Setup never routes Stop through this process-killing cancellation token.
        self.running = Some((operation, Cancellation::default()));
        self.setup.loading = true;
        self.setup.progress = None;
        Some((id, client))
    }
    fn setup_sink(&self, id: u64) -> impl Fn(SetupEvent) + Send + Sync + 'static {
        let sender = self.sender.clone();
        let dropped = self.dropped.clone();
        move |event| {
            let event = match event {
                SetupEvent::Output(event) => Event::Progress(id, event),
                SetupEvent::Progress(report) => Event::SetupProgress(id, report),
            };
            if sender.try_send(event).is_err() {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    fn load_setup(&mut self, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.setup.support, Some(Ok(())))
        {
            return;
        }
        let support = self.setup.support.take();
        let revision = self.setup.revision.wrapping_add(1);
        self.setup = SetupSession {
            visible: true,
            support,
            revision,
            ..Default::default()
        };
        let Some((id, client)) = self.setup_operation(Operation::LoadSetup) else {
            return;
        };
        let sender = self.sender.clone();
        let sink = self.setup_sink(id);
        std::thread::spawn(move || {
            let result = client.setup_context(sink).map_err(Failure::from);
            let _ = sender.send(Event::SetupLoaded(id, revision, result));
        });
        cx.notify();
    }
    fn finish_setup_operation(&mut self, id: u64, outcome: &Result<OperationOutcome, String>) {
        self.operations.finish(id, outcome);
        self.running = None;
        self.setup.loading = false;
        self.setup.stop = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
        self.poll_status();
    }
    pub(in super::super) fn accept_setup_loaded(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<SetupContext, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || self.setup.revision != revision
            || !matches!(self.running, Some((Operation::LoadSetup, _)))
        {
            return;
        }
        let retry = self.setup.retry_selection.take();
        let review_after_load = retry.is_some();
        let outcome = match result {
            Ok(context) => {
                let mut selection = retry.unwrap_or_else(|| context.saved.clone());
                selection.root = context.saved.root.clone();
                let valid = SetupReview::new(context.clone(), selection.clone());
                self.setup.selection = Some(selection);
                self.setup.context = Some(context);
                match valid {
                    Ok(_) => {
                        self.setup.error = None;
                        Ok(OperationOutcome::Succeeded)
                    }
                    Err(error) => {
                        self.setup.reload_required = true;
                        self.setup.error = Some(format!(
                            "Saved choices changed. Reload saved selections before review: {error:#}"
                        ));
                        Err(self.setup.error.clone().unwrap())
                    }
                }
            }
            Err(error) => {
                self.setup.selection = retry;
                self.setup.teardown_uncertain = error.teardown;
                self.setup.error = Some(error.message.clone());
                Err(error.message)
            }
        };
        self.finish_setup_operation(id, &outcome);
        if review_after_load && outcome.is_ok() && !self.setup.close_pending {
            self.prepare_machine_setup(cx);
        }
        self.finish_setup_close(cx);
        cx.notify();
    }
    pub(super) fn prepare_machine_setup(&mut self, cx: &mut Context<Self>) {
        if self.setup.reload_required || self.setup.dialog_open {
            return;
        }
        let Some(context) = self.setup.context.clone() else {
            return;
        };
        let Some(selection) = self.setup.selection.clone() else {
            return;
        };
        let Some((id, client)) = self.setup_operation(Operation::PrepareSetup) else {
            return;
        };
        self.setup.step = Step::Review;
        self.setup.result = None;
        self.setup.prepared = None;
        self.setup.error = None;
        let revision = self.setup.revision;
        let sender = self.sender.clone();
        let sink = self.setup_sink(id);
        std::thread::spawn(move || {
            let result = client
                .prepare_setup(context, selection, sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::SetupPrepared(id, revision, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_setup_prepared(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<SetupReview, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || self.setup.revision != revision
            || !matches!(self.running, Some((Operation::PrepareSetup, _)))
        {
            return;
        }
        let outcome = match result {
            Ok(review)
                if self.setup.context.as_ref() == Some(&review.context)
                    && self.setup.selection.as_ref() == Some(&review.selection) =>
            {
                self.setup.prepared = Some(review);
                self.setup.error = None;
                Ok(OperationOutcome::Succeeded)
            }
            Ok(_) => {
                self.setup.reload_required = true;
                self.setup.error =
                    Some("Prepared selections changed; explicitly Reload saved selections.".into());
                Err(self.setup.error.clone().unwrap())
            }
            Err(error) => {
                self.setup.teardown_uncertain = error.teardown;
                self.setup.reload_required = error.message.to_ascii_lowercase().contains("reload");
                self.setup.error = Some(error.message.clone());
                Err(error.message)
            }
        };
        self.finish_setup_operation(id, &outcome);
        self.finish_setup_close(cx);
        cx.notify();
    }
    pub(super) fn confirm_setup_apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || self.setup.dialog_open
            || SetupSession::watch_conflict(self.watch.as_ref())
        {
            return;
        }
        let Some(review) = self.setup.ready().cloned() else {
            return;
        };
        self.setup.dialog_open = true;
        let revision = self.setup.revision;
        let generation = self.watch_generation;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let apply = owner.clone(); let closed = owner.clone(); let review = review.clone();
            dialog.title("Apply Machine Setup?")
                .child(format!("Run {} items on this machine, including Rokit bootstrap and the projects folder {}. This can download software, change global tools, show installer/UAC dialogs and require manual steps. Completed installations remain after failure or Stop. Selections save only after a completed run, including warnings. Watch stays unchanged.", review.items.len(), review.selection.root.display()))
                .footer(DialogFooter::new()
                    .child(Button::new("setup-keep-reviewing").primary().label("Keep reviewing").on_click(move |_, window, cx| { window.close_dialog(cx); let _ = keep.update(cx, |this,cx| this.finish_setup_dialog(window,cx)); }))
                    .child(Button::new("setup-confirm-apply").label("Apply setup").on_click(move |_,window,cx| {
                        window.close_dialog(cx); let _ = apply.update(cx, |this,cx| {
                            this.setup.dialog_open = false;
                            if this.setup.revision == revision && this.setup.ready() == Some(&review) && this.watch_generation == generation && !SetupSession::watch_conflict(this.watch.as_ref()) {
                                this.apply_machine_setup(review.clone(),cx);
                            } else { this.setup.error = Some("Confirmation expired because selections or Watch changed. Review again.".into()); }
                            this.focus_setup(window,cx); cx.notify();
                        });
                    })))
                .on_close(move |_,window,cx| { let _ = closed.update(cx, |this,cx| this.finish_setup_dialog(window,cx)); })
        });
    }
    fn apply_machine_setup(&mut self, review: SetupReview, cx: &mut Context<Self>) {
        let Some((id, client)) = self.setup_operation(Operation::ApplySetup) else {
            return;
        };
        let stop = SetupStop::default();
        self.setup.stop = Some(stop.clone());
        self.setup.step = Step::Results;
        self.setup.error = None;
        let sender = self.sender.clone();
        let sink = self.setup_sink(id);
        std::thread::spawn(move || {
            let result = client
                .apply_setup(review, &stop, sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::SetupApplied(id, Box::new(result)));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_setup_progress(&mut self, id: u64, report: Box<SetupReport>) {
        if self.operations.accepts(id)
            && self.setup_applying()
            && self.setup.accept_progress(report)
        {
            self.operations.expanded = true;
        }
    }
    pub(in super::super) fn accept_setup_applied(
        &mut self,
        id: u64,
        result: Result<SetupReport, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) || !self.setup_applying() {
            return;
        }
        let outcome = match result {
            Ok(report) => {
                // Bounded progress delivery can drop an intermediate failure;
                // the reliable final report must still reveal a new failure.
                if self.setup.has_new_failure(&report) {
                    self.operations.expanded = true;
                }
                self.setup.reload_required = report.effects != WriteState::NotStarted
                    || report
                        .message
                        .as_ref()
                        .is_some_and(|m| m.to_ascii_lowercase().contains("reload"));
                if report.status == Some(SetupStatus::Succeeded) {
                    if let Some(context) = &mut self.setup.context
                        && let Some(selection) = &self.setup.selection
                    {
                        context.saved = selection.clone();
                    }
                    // The exact saved bytes are retained in the result. Filesystem
                    // identities require an explicit reload before another review.
                    self.load_creation_context(cx);
                }
                let outcome = match report.status {
                    Some(SetupStatus::Succeeded) => Ok(OperationOutcome::Succeeded),
                    Some(SetupStatus::Stopped) => Ok(OperationOutcome::Cancelled),
                    _ => Err(report.message.clone().unwrap_or_else(|| "Setup has failures; inspect item results and whether selections were saved. Completed installations may remain.".into())),
                };
                self.setup.error = report.message.clone();
                self.setup.result = Some(report);
                outcome
            }
            Err(error) => {
                self.setup.teardown_uncertain = error.teardown;
                self.setup.reload_required |= error.teardown;
                self.setup.error = Some(error.message.clone());
                Err(error.message)
            }
        };
        self.finish_setup_operation(id, &outcome);
        self.finish_setup_close(cx);
        cx.notify();
    }
    fn finish_setup_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.setup.dialog_open = false;
        self.focus_setup(window, cx);
        cx.notify();
    }
    pub(super) fn replace_setup(
        &mut self,
        action: Replace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.setup.dialog_open {
            return;
        }
        if !self.setup.dirty() && !matches!(action, Replace::Retry) {
            self.perform_setup_replace(action, window, cx);
            return;
        }
        self.setup.dialog_open = true;
        let revision = self.setup.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog,_,_| {
            let keep=owner.clone(); let replace=owner.clone(); let closed=owner.clone();
            dialog.title("Keep the Machine Setup draft?")
                .child(if matches!(action,Replace::Retry) { "Reload current machine configuration and its projects folder, retain the proposed software selections, then prepare a new Review. Nothing is installed until you explicitly Apply again. Unfamiliar changed choices may require reloading saved selections." } else { "These machine selections are session-only. Reloading or discarding replaces the unsaved choices. It does not undo any installed software or alter other project drafts." })
                .footer(DialogFooter::new()
                    .child(Button::new("setup-keep-draft").primary().label("Keep draft").on_click(move |_,window,cx| { window.close_dialog(cx); let _=keep.update(cx,|this,cx|this.finish_setup_dialog(window,cx)); }))
                    .child(Button::new("setup-replace-draft").label(match action { Replace::Reload => "Reload saved selections", Replace::Retry => "Reload and review", Replace::Discard(_) => "Discard draft" }).on_click(move |_,window,cx| {
                        window.close_dialog(cx); let _=replace.update(cx,|this,cx| {
                            this.setup.dialog_open=false;
                            if this.setup.revision==revision && this.running.is_none() { this.perform_setup_replace(action,window,cx); }
                            this.focus_setup(window,cx);
                        });
                    })))
                .on_close(move |_,window,cx| { let _=closed.update(cx,|this,cx|this.finish_setup_dialog(window,cx)); })
        });
    }
    fn perform_setup_replace(
        &mut self,
        action: Replace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            Replace::Reload => self.load_setup(cx),
            Replace::Retry => {
                let selection = self.setup.selection.clone();
                self.load_setup(cx);
                if self.setup.loading {
                    self.setup.retry_selection = selection;
                }
            }
            Replace::Discard(closing) => {
                let support = self.setup.support.take();
                let revision = self.setup.revision.wrapping_add(1);
                self.setup = SetupSession {
                    support,
                    revision,
                    ..Default::default()
                };
                if closing {
                    self.continue_close(cx);
                }
            }
        }
        self.focus_setup(window, cx);
        cx.notify();
    }
    pub(in super::super) fn confirm_setup_stop(
        &mut self,
        closing: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.setup_running() || self.setup.dialog_open || self.teardown_uncertain() {
            return;
        }
        let applying = self.setup_applying();
        let id = self.operations.latest.as_ref().unwrap().id;
        self.setup.dialog_open = true;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx,move|dialog,_,_| {
            let keep=owner.clone(); let stop=owner.clone(); let closed=owner.clone();
            dialog.title(if applying { "Machine Setup is running" } else { "Machine Setup preparation is running" })
                .child(if applying { "Stop waits for the active installer and owned descendants. Installed software can remain; stopped runs do not save selections. Closing then checks other unsaved drafts. Unconfirmed termination keeps this window open." } else { "Preparation is read-only. Closing waits for this worker to finish, then checks unsaved drafts." })
                .footer(DialogFooter::new()
                    .child(Button::new("setup-keep-open").primary().label(if closing {"Keep open"} else {"Keep running"}).on_click(move|_,window,cx| { window.close_dialog(cx); let _=keep.update(cx,|this,cx|this.finish_setup_dialog(window,cx)); }))
                    .child(Button::new("setup-confirm-stop").label(if !applying {"Close after preparation"} else if closing {"Stop and close"} else {"Stop after active item"}).on_click(move|_,window,cx| {
                        window.close_dialog(cx); let _=stop.update(cx,|this,cx| {
                            this.setup.dialog_open=false;
                            if this.operations.accepts(id) && this.setup_running() {
                                this.setup.close_pending=closing;
                                if applying && let Some(stop)=&this.setup.stop {
                                    stop.request(); this.operations.stop_setup();
                                }
                            } else if closing && this.operations.latest.as_ref().is_some_and(|r|r.id==id) && this.running.is_none() && !this.teardown_uncertain() {
                                this.continue_close(cx);
                            }
                            this.focus_setup(window,cx); cx.notify();
                        });
                    })))
                .on_close(move|_,window,cx| { let _=closed.update(cx,|this,cx|this.finish_setup_dialog(window,cx)); })
        });
    }
    pub(in super::super) fn setup_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.setup.dialog_open || self.setup.close_pending || self.setup.teardown_uncertain {
            return Some(false);
        }
        if self.setup_running() {
            self.confirm_setup_stop(true, window, cx);
            return Some(false);
        }
        // Submitted results already explain unsaved selections and partial effects.
        if self.running.is_none() && self.setup.dirty() && self.setup.result.is_none() {
            self.replace_setup(Replace::Discard(true), window, cx);
            return Some(false);
        }
        None
    }
    fn finish_setup_close(&mut self, cx: &mut Context<Self>) {
        if std::mem::take(&mut self.setup.close_pending) {
            if !self.setup.teardown_uncertain {
                self.continue_close(cx);
            } else {
                self.operations.expanded = true;
            }
        }
    }
}
