use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

#[derive(Clone)]
pub(super) enum Intent {
    Replace(PathBuf),
    Refresh,
    Discard(bool),
}

impl Workbench {
    pub(in super::super) fn probe_tests(&mut self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        self.test_support_generation = self.test_support_generation.wrapping_add(1);
        let generation = self.test_support_generation;
        self.test_args.support = None;
        std::thread::spawn(move || {
            let _ = sender.send(Event::TestSupport(
                generation,
                client.test_arguments_support().map_err(Failure::from),
            ));
        });
    }
    pub(in super::super) fn accept_test_support(
        &mut self,
        generation: u64,
        result: Result<(), Failure>,
        cx: &mut Context<Self>,
    ) {
        // A superseded probe cannot replace support state, but its unconfirmed
        // process teardown remains a closing/mutation guard for this session.
        if let Err(error) = &result
            && error.teardown
        {
            self.test_args.teardown_uncertain = true;
            self.operations.expanded = true;
            self.activity.append_untagged(&format!(
                "Custom-test support probe teardown is unconfirmed: {}",
                error.message
            ));
        }
        if generation == self.test_support_generation {
            self.test_args.support = Some(result.map_err(|error| error.message));
        }
        cx.notify();
    }
    fn test_focus(&self, w: &mut Window, cx: &mut Context<Self>) {
        if self.test_args.visible {
            self.test_args_focus.focus(w, cx);
        } else {
            self.focus.focus(w, cx);
        }
    }
    pub(in super::super) fn show_test_args(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.browser.leave();
        self.source.visible = false;
        self.library.visible = false;
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.settings.visible = false;
        self.project_tool.visible = false;
        self.upgrade.visible = false;
        self.composition.visible = false;
        self.creation.visible = false;
        self.test_args.visible = true;
        self.test_focus(w, cx);
        cx.notify();
    }
    pub(in super::super) fn edit_test_args(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.test_args.project.as_ref() == Some(&project) {
            self.show_test_args(w, cx);
        } else {
            self.request_test_intent(Intent::Replace(project), w, cx);
        }
    }
    fn dismiss_test_dialog(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.test_args.dialog = false;
        self.test_focus(w, cx);
        cx.notify();
    }
    fn focus_test_keep(&self, anchor: FocusHandle, w: &mut Window, cx: &Context<Self>) {
        let owner = cx.entity().downgrade();
        // Kit owns button focus handles. Enter the first (Keep) tab stop only
        // after the dialog has registered its controls in the next frame.
        w.on_next_frame(move |w, cx| {
            let _ = owner.update(cx, |this, cx| {
                if this.test_args.dialog {
                    anchor.focus(w, cx);
                    w.focus_next(cx);
                }
            });
        });
    }
    pub(super) fn request_test_intent(
        &mut self,
        intent: Intent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.test_args.dialog {
            return;
        }
        if !self.test_args.dirty() {
            self.perform_test_intent(intent, w, cx);
            return;
        }
        self.test_args.dialog = true;
        let revision = self.test_args.revision;
        let name = self.test_args.name();
        let label = match intent {
            Intent::Refresh => "Refresh context, keep arguments",
            Intent::Replace(_) => "Replace draft",
            Intent::Discard(_) => "Discard draft",
        };
        let owner = cx.entity().downgrade();
        let keep_anchor = cx.focus_handle();
        let footer_anchor = keep_anchor.clone();
        w.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let apply = owner.clone(); let closed = owner.clone(); let intent = intent.clone();
            dialog.title("Keep unsubmitted test arguments?")
                .child(format!("Unsubmitted arguments belong to {name}. Refresh retains their exact values; replacement or discard removes this draft. Other drafts and Watch stay unchanged."))
                .footer(DialogFooter::new()
                    .child(div().track_focus(&footer_anchor).w_0().h_0())
                    .child(Button::new("test-keep-draft").primary().label("Keep draft").on_click(move |_, w, cx| { w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.dismiss_test_dialog(w, cx)); }))
                    .child(Button::new("test-draft-continue").label(label).on_click(move |_, w, cx| {
                        w.close_dialog(cx); let _ = apply.update(cx, |this, cx| {
                            this.dismiss_test_dialog(w, cx);
                            if this.test_args.revision == revision && this.running.is_none() && !this.teardown_uncertain() { this.perform_test_intent(intent.clone(), w, cx); }
                        });
                    })))
                .on_close(move |_, w, cx| { let _ = closed.update(cx, |this, cx| this.dismiss_test_dialog(w, cx)); })
        });
        self.focus_test_keep(keep_anchor, w, cx);
    }
    fn perform_test_intent(&mut self, intent: Intent, w: &mut Window, cx: &mut Context<Self>) {
        match intent {
            Intent::Replace(project) => {
                let support = self.test_args.support.take();
                let revision = self.test_args.revision.wrapping_add(1);
                self.test_args = Session {
                    project: Some(project),
                    support,
                    revision,
                    ..Default::default()
                };
                self.test_args_inputs = Inputs::default();
                self.show_test_args(w, cx);
                self.load_test_context(cx);
            }
            Intent::Refresh => self.load_test_context(cx),
            Intent::Discard(closing) => {
                let support = self.test_args.support.take();
                let revision = self.test_args.revision.wrapping_add(1);
                self.test_args = Session {
                    support,
                    revision,
                    ..Default::default()
                };
                self.test_args_inputs = Inputs::default();
                self.test_focus(w, cx);
                if closing {
                    self.continue_close(cx);
                }
            }
        }
        cx.notify();
    }
    pub(super) fn add_test_row(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let id = self.test_args.add();
        self.test_args.changed();
        let input = cx.new(|cx| InputState::new(w, cx).placeholder("One exact argument value"));
        let sub = cx.subscribe(&input, move |this, input, _: &InputEvent, cx| {
            let value = input.read(cx).value().to_string();
            if let Some(row) = this.test_args.rows.iter_mut().find(|r| r.id == id)
                && row.text != value
            {
                row.edit(value);
                this.test_args.changed();
                cx.notify();
            }
        });
        self.test_args_inputs.rows.insert(id, input.clone());
        self.test_args_inputs.subscriptions.insert(id, sub);
        input.read(cx).focus_handle(cx).focus(w, cx);
        cx.notify();
    }
    pub(super) fn change_test_mode(
        &mut self,
        id: u64,
        mode: Mode,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(row) = self.test_args.rows.iter_mut().find(|r| r.id == id) else {
            return;
        };
        match row.switch_mode(mode) {
            Ok(()) => {
                let text = row.text.clone();
                self.test_args.error = None;
                if let Some(input) = self.test_args_inputs.rows.get(&id) {
                    input.update(cx, |input, cx| input.set_value(text, w, cx));
                }
            }
            Err(error) => self.test_args.error = Some(error),
        }
        cx.notify();
    }
    pub(super) fn paste_test_row(
        &mut self,
        id: u64,
        text: &str,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(input) = self.test_args_inputs.rows.get(&id).cloned() else {
            return true;
        };
        let selection = input.read(cx).selected_range();
        let Some(row) = self.test_args.rows.iter_mut().find(|r| r.id == id) else {
            return true;
        };
        match row.paste(text, selection) {
            Ok(false) => false,
            Ok(true) => {
                let text = row.text.clone();
                self.test_args.changed();
                input.update(cx, |input, cx| input.set_value(text, w, cx));
                cx.notify();
                true
            }
            Err(error) => {
                self.test_args.error = Some(error);
                cx.notify();
                true
            }
        }
    }
    pub(super) fn move_test_row(&mut self, id: u64, up: bool, cx: &mut Context<Self>) {
        let Some(index) = self.test_args.rows.iter().position(|r| r.id == id) else {
            return;
        };
        let next = if up {
            index.checked_sub(1)
        } else {
            (index + 1 < self.test_args.rows.len()).then_some(index + 1)
        };
        if let Some(next) = next {
            self.test_args.rows.swap(index, next);
            self.test_args.changed();
            cx.notify();
        }
    }
    pub(super) fn remove_test_row(&mut self, id: u64, w: &mut Window, cx: &mut Context<Self>) {
        self.test_args.rows.retain(|r| r.id != id);
        self.test_args_inputs.rows.remove(&id);
        self.test_args_inputs.subscriptions.remove(&id);
        self.test_args.changed();
        self.test_focus(w, cx);
        cx.notify();
    }
    fn begin_test_operation(
        &mut self,
        operation: Operation,
        project: PathBuf,
    ) -> Option<(u64, WorkbenchClient, Cancellation)> {
        if self.running.is_some() || self.teardown_uncertain() {
            return None;
        }
        let client = self.client.clone()?;
        let id = self.operations.begin(operation, project);
        let token = Cancellation::default();
        self.running = Some((operation, token.clone()));
        self.pending_switch = None;
        self.test_args.completed = None;
        self.activity.flush();
        self.activity
            .set_origin(self.operations.latest.as_ref().map(|r| r.origin()));
        Some((id, client, token))
    }
    fn test_sink(&self, id: u64) -> impl Fn(OperationEvent) + Send + Sync + 'static {
        let sender = self.sender.clone();
        let dropped = self.dropped.clone();
        move |event| {
            if sender.try_send(Event::Progress(id, event)).is_err() {
                dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    pub(super) fn load_test_context(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.test_args.support, Some(Ok(()))) {
            return;
        }
        let Some(project) = self.test_args.project.clone() else {
            return;
        };
        let Some((id, client, token)) =
            self.begin_test_operation(Operation::LoadTestContext, project.clone())
        else {
            return;
        };
        self.test_args.stale = true;
        self.test_args.error = None;
        let revision = self.test_args.revision;
        let sender = self.sender.clone();
        let sink = self.test_sink(id);
        std::thread::spawn(move || {
            let result = client
                .test_context(&project, &token, sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::TestContext(id, revision, result));
        });
        cx.notify();
    }
    pub(super) fn run_test_arguments(&mut self, cx: &mut Context<Self>) {
        if self.test_args.dialog
            || !matches!(self.test_args.support, Some(Ok(())))
            || watch_blocks(self.watch.as_ref())
        {
            return;
        }
        let (context, arguments) = match self.test_args.ready() {
            Ok(value) => value,
            Err(error) => {
                self.test_args.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some((id, client, token)) =
            self.begin_test_operation(Operation::CustomTest, context.project.clone())
        else {
            return;
        };
        self.test_args.submitted = Some((id, arguments.clone()));
        self.test_args.result = None;
        self.test_args.error = None;
        let sender = self.sender.clone();
        let sink = self.test_sink(id);
        std::thread::spawn(move || {
            let result = client
                .run_custom_tests(context, arguments, &token, sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::CustomTestComplete(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn start_default_test(
        &mut self,
        project: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some((id, client, token)) = self.begin_test_operation(Operation::Test, project.clone())
        else {
            return;
        };
        let sender = self.sender.clone();
        let sink = self.test_sink(id);
        std::thread::spawn(move || {
            let result = client
                .run(WorkerOperation::Test, &project, &token, sink)
                .map_err(Failure::from);
            let _ = sender.send(Event::DefaultTestComplete(id, result));
        });
        cx.notify();
    }
    fn finish_test_operation(&mut self, id: u64, result: &Result<OperationOutcome, String>) {
        self.operations.finish(id, result);
        self.running = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
        self.test_args.completed = Some(id);
    }
    pub(in super::super) fn accept_test_context(
        &mut self,
        id: u64,
        _revision: u64,
        result: Result<TestContext, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || !matches!(self.running, Some((Operation::LoadTestContext, _)))
        {
            return;
        }
        let result = result.and_then(|context| {
            if self
                .operations
                .latest
                .as_ref()
                .is_some_and(|run| run.project == context.project)
                && self.test_args.project.as_ref() == Some(&context.project)
            {
                Ok(context)
            } else {
                Err(Failure {
                    message: "Test context belongs to another project; Refresh explicitly.".into(),
                    teardown: false,
                })
            }
        });
        let outcome = result
            .as_ref()
            .map(|_| OperationOutcome::Succeeded)
            .map_err(|e| e.message.clone());
        self.finish_test_operation(id, &outcome);
        match result {
            Ok(context) => {
                self.test_args.project = Some(context.project.clone());
                self.test_args.context = Some(context);
                self.test_args.stale = false;
                self.test_args.error = None;
            }
            Err(error) => {
                self.test_args.teardown_uncertain |= error.teardown;
                self.test_args.error = Some(error.message);
            }
        }
        cx.notify();
    }
    pub(in super::super) fn accept_custom_tests(
        &mut self,
        id: u64,
        result: Result<Completion, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) || !matches!(self.running, Some((Operation::CustomTest, _)))
        {
            return;
        }
        let result = result.and_then(|value| {
            if self.test_args.project.as_ref() == Some(&value.project) {
                Ok(value)
            } else {
                Err(Failure {
                    message: "Test completion belongs to another project; Refresh explicitly."
                        .into(),
                    teardown: false,
                })
            }
        });
        let outcome = result
            .as_ref()
            .map_err(|e| e.message.clone())
            .and_then(|value| match value.status {
                Status::Succeeded => Ok(OperationOutcome::Succeeded),
                Status::Cancelled => Ok(OperationOutcome::Cancelled),
                Status::Failed | Status::Uncertain => Err(value
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("{:?}", value.status))),
            });
        self.finish_test_operation(id, &outcome);
        match result {
            Ok(value) => {
                if value.accepted == Some(true)
                    && let Some((submitted, arguments)) = &self.test_args.submitted
                    && *submitted == id
                {
                    self.test_args.baseline = arguments.clone();
                }
                self.test_args.stale = true;
                self.test_args.error = value.message.clone();
                self.test_args.result = Some(value);
            }
            Err(error) => {
                self.test_args.teardown_uncertain |= error.teardown;
                self.test_args.stale = true;
                self.test_args.error = Some(error.message);
            }
        }
        self.finish_test_close(cx);
        self.inspect();
        cx.notify();
    }
    pub(in super::super) fn accept_default_tests(
        &mut self,
        id: u64,
        result: Result<OperationOutcome, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) || !matches!(self.running, Some((Operation::Test, _))) {
            return;
        }
        let outcome = result.as_ref().cloned().map_err(|e| e.message.clone());
        if let Err(error) = &result {
            self.test_args.teardown_uncertain |= error.teardown;
        }
        self.finish_test_operation(id, &outcome);
        self.finish_test_close(cx);
        self.inspect();
        cx.notify();
    }
    fn finish_test_close(&mut self, cx: &mut Context<Self>) {
        if self.test_args.close_pending {
            self.test_args.close_pending = false;
            if !self.test_args.teardown_uncertain {
                self.continue_close(cx);
            } else {
                self.operations.expanded = true;
            }
        }
    }
    pub(in super::super) fn test_close(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.test_args.dialog || self.test_args.close_pending {
            return Some(false);
        }
        if matches!(
            self.running,
            Some((Operation::Test | Operation::CustomTest, _))
        ) {
            self.test_args.dialog = true;
            let run = self.operations.latest.as_ref().unwrap();
            let id = run.id;
            let project = run.project.display().to_string();
            let owner = cx.entity().downgrade();
            let keep_anchor = cx.focus_handle();
            let footer_anchor = keep_anchor.clone();
            w.open_alert_dialog(cx, move |dialog, _, _| {
                let keep = owner.clone(); let cancel = owner.clone(); let closed = owner.clone();
                dialog.title("Tests are running").child(format!("Project: {project}. Preparation may leave partial changes. Cancel tests and close waits for the worker, descendants and output, then protects other drafts. Watch remains running."))
                    .footer(DialogFooter::new()
                        .child(div().track_focus(&footer_anchor).w_0().h_0())
                        .child(Button::new("test-keep-open").primary().label("Keep open").on_click(move |_, w, cx| { w.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.dismiss_test_dialog(w, cx)); }))
                        .child(Button::new("test-cancel-close").label("Cancel tests and close").on_click(move |_, w, cx| {
                            w.close_dialog(cx); let _ = cancel.update(cx, |this, cx| {
                                this.dismiss_test_dialog(w, cx);
                                if this.operations.accepts(id) && matches!(this.running, Some((Operation::Test | Operation::CustomTest, _))) {
                                    this.test_args.close_pending = true;
                                    if let Some((_, token)) = &this.running { token.cancel(); }
                                    this.operations.cancel();
                                } else if this.test_args.completed == Some(id) && !this.test_args.teardown_uncertain { this.continue_close(cx); }
                                cx.notify();
                            });
                        })))
                    .on_close(move |_, w, cx| { let _ = closed.update(cx, |this, cx| this.dismiss_test_dialog(w, cx)); })
            });
            self.focus_test_keep(keep_anchor, w, cx);
            return Some(false);
        }
        // An unrelated running operation keeps its existing close/cancel policy.
        if self.running.is_none() && self.test_args.dirty() {
            self.request_test_intent(Intent::Discard(true), w, cx);
            return Some(false);
        }
        None
    }
}
