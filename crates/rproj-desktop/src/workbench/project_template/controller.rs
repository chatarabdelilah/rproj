use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

#[derive(Clone, Copy)]
pub(super) enum Intent {
    Reload,
    Discard,
    DiscardClose,
    Execute,
    CloseAfter(u64),
    Delete,
}

impl Workbench {
    pub(in super::super) fn template_connected(
        &mut self,
        result: Result<(), String>,
        cx: &mut Context<Self>,
    ) {
        self.template.support = Some(result);
        if self.template.visible && self.template.draft.is_none() && self.template.pending.is_none()
        {
            self.start_template(Task::Load, cx);
        }
    }
    pub(in super::super) fn probe_template(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::TemplateSupport(
                client
                    .project_template_support()
                    .map_err(|e| format!("{e:#}")),
            ));
        });
    }
    pub(super) fn show_template(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.creation.visible = false;
        self.composition.visible = false;
        self.settings.visible = false;
        self.setup.visible = false;
        self.saved.visible = false;
        self.upgrade.visible = false;
        self.browser.leave();
        self.template.visible = true;
        self.template_inputs.focus.focus(w, cx);
        if self.template.draft.is_none() && self.template.pending.is_none() {
            self.start_template(Task::Load, cx);
        }
        cx.notify();
    }
    pub(in super::super) fn start_template(&mut self, task: Task, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.template.support, Some(Ok(())))
        {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        let owner = self
            .template
            .draft
            .as_ref()
            .map(|d| d.loaded.source.path.clone())
            .unwrap_or_else(|| "Project Template".into());
        let operation = task.operation();
        let id = self.operations.begin(operation, owner);
        self.activity
            .set_origin(self.operations.latest.as_ref().map(|r| r.origin()));
        self.running = Some((operation, Cancellation::default()));
        self.template.pending = Some(task.clone());
        self.template.last = None;
        self.template.notice = None;
        self.template.error = None;
        let tx = self.sender.clone();
        let progress = tx.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let sink = move |event| {
                if progress.try_send(Event::Progress(id, event)).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            };
            let result = (|| -> anyhow::Result<Response> {
                Ok(match task {
                    Task::Load => Response::Loaded(client.project_template_context(sink)?),
                    Task::Prepare(source, action) => {
                        let prepared = client.prepare_project_template(source, action, sink)?;
                        let rows = Arc::new(prepared.review.diff());
                        {
                            let summary = semantic_summary(&prepared);
                            Response::Prepared(Box::new(prepared), rows, summary)
                        }
                    }
                    Task::Execute(review) => {
                        Response::Executed(client.apply_project_template(*review, sink)?)
                    }
                })
            })();
            let _ = tx.send(Event::TemplateResponse(id, Box::new(result)));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_template(
        &mut self,
        id: u64,
        result: Result<Response, anyhow::Error>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let Some(task) = self.template.pending.take() else {
            return;
        };
        let mut confirmed = true;
        let result = result.and_then(|response| {
            match (task, response) {
                (Task::Load, Response::Loaded(loaded)) => self.template.load(loaded),
                (Task::Prepare(source, action), Response::Prepared(plan, rows, summary))
                    if plan.review.source == source && plan.review.action == action => {
                    self.template_inputs.list.reset(rows.len());
                    self.template.rows = rows;
                    self.template.summary = summary;
                    self.template.prepared = Some(plan);
                    self.template.page = Page::Review;
                }
                (Task::Execute(plan), Response::Executed(out)) if out.path == plan.review.source.path => {
                    if out.status != TemplateStatus::Succeeded {
                        confirmed = out.status != TemplateStatus::Uncertain;
                        self.template.reload |= !confirmed;
                        let message = out.message.unwrap_or_else(|| "Template operation failed. Reload from file before retrying.".into());
                        return Err(anyhow::anyhow!(message));
                    }
                    let Some(saved) = out.saved else {
                        confirmed = false;
                        anyhow::bail!("Saved template baseline is missing. Reload from file.");
                    };
                    self.template.load(LoadedTemplate::new(saved));
                    let notice = match plan.review.action {
                        TemplateAction::Reset => "Restored the built-in template for future projects.",
                        TemplateAction::Save(_) => "Saved the template for future projects.",
                    };
                    self.template.notice = Some(notice.into());
                }
                _ => {
                    confirmed = false;
                    anyhow::bail!("Template response does not belong to the submitted operation. Reload from file.");
                }
            }
            Ok(())
        });
        let outcome = result.map(|()| OperationOutcome::Succeeded).map_err(|e| {
            self.template.teardown_uncertain |=
                rproj_core::operations::owned_teardown_unconfirmed(&e);
            let message = format!("{e:#}");
            confirmed &= !self.template.teardown_uncertain
                && !e.is::<rproj_core::project_template::protocol::UnconfirmedCompletion>();
            self.template.reload |=
                !confirmed || message.to_lowercase().contains("reload from file");
            self.template.error = Some(message.clone());
            message
        });
        self.operations.finish(id, &outcome);
        self.running = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
        self.template.last = Some((id, confirmed));
        if self.template.close_after.take() == Some(id) && confirmed {
            self.continue_close(cx);
        }
        cx.notify();
    }
    pub(super) fn prepare_template(&mut self, reset: bool, cx: &mut Context<Self>) {
        if !self.template.editable() {
            return;
        }
        let Some(d) = &self.template.draft else {
            return;
        };
        let action = if reset && d.loaded.source.original.is_some() {
            TemplateAction::Reset
        } else if !reset && self.template.page != Page::Json {
            let Some(model) = &d.model else { return };
            TemplateAction::Save(model.value().clone())
        } else {
            return;
        };
        self.template.review_return = self.template.page;
        self.start_template(Task::Prepare(d.loaded.source.clone(), action), cx);
    }
    pub(super) fn dismiss_template(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.template.dialog = false;
        if self.template.visible {
            self.template_inputs.focus.focus(w, cx)
        } else {
            self.focus.focus(w, cx)
        }
        cx.notify();
    }
    pub(super) fn template_decision(
        &mut self,
        intent: Intent,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.template.dialog {
            return;
        }
        let revision = self.template.revision;
        let path = self
            .template
            .draft
            .as_ref()
            .map(|d| d.loaded.source.path.display().to_string())
            .unwrap_or_else(|| "Project Template".into());
        let (title, body, keep, accept) = match intent {
            Intent::CloseAfter(_) => (
                "Project Template is still working",
                format!(
                    "Closing waits for confirmed completion at {path}, then checks your other drafts."
                ),
                "Keep open",
                "Close after operation",
            ),
            Intent::Reload | Intent::Discard | Intent::DiscardClose => {
                if !self.template.draft.as_ref().is_some_and(Draft::dirty) {
                    self.perform_template_intent(intent, revision, cx);
                    return;
                }
                (
                    "Discard unsaved template edits?",
                    format!("These edits to {path} exist only in this session."),
                    "Keep draft",
                    if matches!(intent, Intent::Reload) {
                        "Discard and reload"
                    } else {
                        "Discard draft"
                    },
                )
            }
            Intent::Execute => {
                let Some(p) = &self.template.prepared else {
                    return;
                };
                if !p.review.changes() || self.template.reload {
                    return;
                }
                match p.review.action {
                    TemplateAction::Save(_) => (
                        "Save the reviewed template?",
                        format!("Save the reviewed configuration to {path} for future projects."),
                        "Keep reviewing",
                        "Save template",
                    ),
                    TemplateAction::Reset => (
                        "Restore built-in template?",
                        format!(
                            "Remove the override at {path}. Pending edits are discarded only after confirmed removal. Existing projects are unaffected."
                        ),
                        "Keep custom template",
                        "Restore built-in template",
                    ),
                }
            }
            Intent::Delete => (
                "Delete this template node?",
                format!(
                    "Remove this node and its children from the draft for {path}. You can undo this edit."
                ),
                "Keep node",
                "Delete from draft",
            ),
        };
        self.template.dialog = true;
        let owner = cx.entity().downgrade();
        w.open_alert_dialog(cx, move |dialog, _, _| {
            let stay = owner.clone();
            let go = owner.clone();
            let dismissed = owner.clone();
            dialog
                .title(title)
                .child(body.clone())
                .footer(
                    DialogFooter::new()
                        .child(Button::new("template-keep").primary().label(keep).on_click(
                            move |_, w, cx| {
                                w.close_dialog(cx);
                                let _ = stay.update(cx, |this, cx| this.dismiss_template(w, cx));
                            },
                        ))
                        .child(Button::new("template-confirm").label(accept).on_click(
                            move |_, w, cx| {
                                w.close_dialog(cx);
                                let _ = go.update(cx, |this, cx| {
                                    this.dismiss_template(w, cx);
                                    this.perform_template_intent(intent, revision, cx);
                                });
                            },
                        )),
                )
                .on_close(move |_, w, cx| {
                    let _ = dismissed.update(cx, |this, cx| this.dismiss_template(w, cx));
                })
        });
    }
    fn perform_template_intent(&mut self, intent: Intent, revision: u64, cx: &mut Context<Self>) {
        if let Intent::CloseAfter(id) = intent {
            if self.template.pending.is_some() && self.operations.accepts(id) {
                self.template.close_after = Some(id)
            } else if self.template.last == Some((id, true)) {
                self.continue_close(cx)
            }
            return;
        }
        if self.template.revision != revision
            || self.template.pending.is_some()
            || self.teardown_uncertain()
        {
            return;
        }
        match intent {
            Intent::CloseAfter(_) => unreachable!(),
            Intent::Execute => {
                if let Some(plan) = self.template.prepared.clone()
                    && self.template.editable()
                    && plan.review.changes()
                {
                    self.start_template(Task::Execute(plan), cx)
                }
            }
            Intent::Reload => self.start_template(Task::Load, cx),
            Intent::Discard | Intent::DiscardClose => {
                if let Some(loaded) = self.template.draft.as_ref().map(|d| d.loaded.clone()) {
                    let reload = self.template.reload;
                    self.template.load(loaded);
                    self.template.reload = reload;
                }
                if matches!(intent, Intent::DiscardClose) {
                    self.continue_close(cx)
                }
            }
            Intent::Delete => self.edit_template(
                |d| {
                    d.model
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("Repair the template first"))?
                        .delete(&d.selected)?;
                    d.selected.pop();
                    Ok(())
                },
                cx,
            ),
        }
        cx.notify();
    }
    pub(in super::super) fn template_close(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.template.dialog
            || self.template.close_after.is_some()
            || self.template.teardown_uncertain
        {
            return Some(false);
        }
        if self.template.pending.is_some() {
            let id = self.operations.latest.as_ref()?.id;
            self.template_decision(Intent::CloseAfter(id), w, cx);
            return Some(false);
        }
        if self.running.is_none() && self.template.draft.as_ref().is_some_and(Draft::dirty) {
            self.template_decision(Intent::DiscardClose, w, cx);
            return Some(false);
        }
        None
    }
    pub(super) fn edit_template(
        &mut self,
        apply: impl FnOnce(&mut Draft) -> anyhow::Result<()>,
        cx: &mut Context<Self>,
    ) {
        if !self.template.editable() {
            return;
        }
        if let Some(d) = self.template.draft.as_mut() {
            match apply(d) {
                Ok(()) => self.template.changed(),
                Err(e) => self.template.error = Some(format!("{e:#}")),
            };
        }
        cx.notify();
    }
}
