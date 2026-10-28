use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

#[derive(Clone)]
pub(super) enum Intent {
    Edit(LoadedSetup),
    Load(String, LoadPurpose),
    Discard(bool),
    Name(LoadedSetup, bool),
    Prepare(CapturedSetup, SetupAction),
    Reuse(CapturedSetup),
}
impl Workbench {
    pub(in super::super) fn probe_saved(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::SavedSupport(
                client.saved_setups_support().map_err(|e| format!("{e:#}")),
            ));
        });
    }
    pub(in super::super) fn show_saved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.creation.visible = false;
        self.composition.visible = false;
        self.upgrade.visible = false;
        self.settings.visible = false;
        self.setup.visible = false;
        self.saved.visible = true;
        self.focus_saved(window, cx);
        if self.saved.listing.is_none() && self.saved.pending.is_none() {
            self.start_saved(Task::Browse, cx);
        }
        cx.notify();
    }
    fn focus_saved(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saved.visible {
            self.saved_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    fn dismiss_saved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.saved.dialog = false;
        self.focus_saved(window, cx);
        cx.notify();
    }
    pub(super) fn request_saved(
        &mut self,
        intent: Intent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.saved.dialog {
            return;
        }
        let affects = match &intent {
            Intent::Edit(loaded) => {
                if self
                    .saved
                    .draft
                    .as_ref()
                    .is_some_and(|d| path(&d.loaded.source) == path(&loaded.source))
                {
                    self.saved.editing = true;
                    self.saved.step = Step::Review;
                    self.saved.naming = None;
                    self.saved_list.reset(self.saved.row_count());
                    self.show_saved(window, cx);
                    return;
                }
                true
            }
            Intent::Load(_, purpose) => matches!(
                purpose,
                LoadPurpose::Reload {
                    replace_draft: true
                }
            ),
            Intent::Discard(_) => true,
            Intent::Name(loaded, _) => self
                .saved
                .draft
                .as_ref()
                .is_some_and(|d| path(&d.loaded.source) == path(&loaded.source)),
            Intent::Prepare(source, _) | Intent::Reuse(source) => self
                .saved
                .draft
                .as_ref()
                .is_some_and(|d| path(&d.loaded.source) == path(source)),
        };
        if !affects || !self.saved.dirty() {
            self.perform_saved_intent(intent, window, cx);
            return;
        }
        self.saved.dialog = true;
        let revision = self.saved.revision;
        let name = self.saved.name();
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let discard = owner.clone(); let review = owner.clone(); let closed = owner.clone(); let intent = intent.clone();
            dialog.title("Keep saved setup edits?").child(format!("Unsaved changes belong to {name}. Save them from Review or explicitly discard them before replacing, reloading or managing that setup."))
                .footer(DialogFooter::new()
                    .child(Button::new("saved-keep-draft").primary().label("Keep draft").on_click(move |_, w, cx| { w.close_dialog(cx); let _ = keep.update(cx, |this,cx| this.dismiss_saved(w,cx)); }))
                    .child(Button::new("saved-review-draft").label("Review changes").on_click(move |_,w,cx| { w.close_dialog(cx); let _ = review.update(cx, |this,cx| { this.dismiss_saved(w,cx); this.saved.editing = true; this.saved.step = Step::Review; this.show_saved(w,cx); this.prepare_saved_edit(cx); }); }))
                    .child(Button::new("saved-discard-continue").label("Discard edits and continue").on_click(move |_,w,cx| { w.close_dialog(cx); let _ = discard.update(cx, |this,cx| {
                        this.saved.dialog = false;
                        if this.saved.revision == revision && this.running.is_none() { this.saved.draft = None; this.saved.changed(); this.perform_saved_intent(intent.clone(),w,cx); }
                        this.focus_saved(w,cx);
                    }); })))
                .on_close(move |_,w,cx| { let _ = closed.update(cx, |this,cx| this.dismiss_saved(w,cx)); })
        });
    }
    fn perform_saved_intent(
        &mut self,
        intent: Intent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match intent {
            Intent::Edit(loaded) => {
                if let Err(e) = self.saved.begin_edit(loaded) {
                    self.saved.error = Some(e.to_string());
                } else {
                    self.prepare_saved_edit(cx);
                }
            }
            Intent::Load(name, replace) => self.start_saved(Task::Load(name, replace), cx),
            Intent::Discard(closing) => {
                self.saved.draft = None;
                self.saved.editing = false;
                self.saved.changed();
                if closing {
                    self.continue_close(cx);
                }
            }
            Intent::Name(loaded, rename) => {
                self.saved.selected = Some(loaded);
                self.saved.editing = false;
                self.saved.naming = Some(rename);
                self.saved.changed();
                self.saved_name
                    .update(cx, |input, cx| input.set_value("", window, cx));
            }
            Intent::Prepare(source, action) => {
                self.saved.editing = false;
                self.start_saved(Task::Prepare(source, action), cx);
            }
            Intent::Reuse(source) => self.start_saved(Task::Reuse(source), cx),
        }
        self.focus_saved(window, cx);
        cx.notify();
    }
    pub(super) fn prepare_saved_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.saved.draft.clone() else {
            return;
        };
        if let Err(e) = edit.draft.validate() {
            self.saved.error = Some(e.to_string());
            cx.notify();
            return;
        }
        self.saved.step = Step::Review;
        self.start_saved(
            Task::Prepare(edit.loaded.source, SetupAction::Save(edit.draft.graph)),
            cx,
        );
    }
    pub(super) fn start_saved(&mut self, task: Task, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.saved.support, Some(Ok(())))
        {
            return;
        }
        let Some(client) = self.client.clone() else {
            return;
        };
        let owner = task
            .source()
            .map(path)
            .or_else(|| {
                self.saved.listing.as_ref().map(|l| match &task {
                    Task::Load(name, _) => l.root.join(format!("{name}.toml")),
                    _ => l.root.clone(),
                })
            })
            .unwrap_or_else(|| PathBuf::from("Saved Setups"));
        let operation = task.operation();
        let id = self.operations.begin(operation, owner);
        self.activity
            .set_origin(self.operations.latest.as_ref().map(|r| r.origin()));
        let token = Cancellation::default();
        self.running = Some((operation, token.clone()));
        self.saved.revision = self.saved.revision.wrapping_add(1);
        let revision = self.saved.revision;
        self.saved.error = None;
        if matches!(task, Task::Prepare(..)) {
            self.saved.prepared = None;
        }
        self.saved.pending = Some(task.clone());
        self.saved.last_confirmed = None;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let sink = move |event| {
                if progress.try_send(Event::Progress(id, event)).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            };
            let result = (|| -> anyhow::Result<Response> {
                Ok(match task {
                    Task::Browse => Response::Browse(client.saved_setups(&token, sink)?),
                    Task::Load(name, _) => {
                        Response::Loaded(client.load_saved_setup(name, &token, sink)?)
                    }
                    Task::Prepare(source, action) => {
                        let prepared = client.prepare_saved_setup(source, action, &token, sink)?;
                        let rows = Arc::new(prepared.rows()?);
                        Response::Prepared(Prepared {
                            review: prepared.review,
                            rows,
                        })
                    }
                    Task::Manage(review) => {
                        Response::Managed(client.manage_saved_setup(*review, sink)?)
                    }
                    Task::Reuse(source) => {
                        Response::Reused(client.reuse_saved_setup(source, &token, sink)?)
                    }
                })
            })()
            .map_err(Failure::from);
            let _ = sender.send(Event::SavedResponse(id, revision, Box::new(result)));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_saved(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<Response, Failure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) || self.saved.revision != revision {
            return;
        }
        let Some(task) = self.saved.pending.take() else {
            return;
        };
        let result = result.and_then(|response| {
            let matches = match (&task, &response) {
                (Task::Browse, Response::Browse(_)) => true,
                (Task::Load(name, _), Response::Loaded(loaded)) => name == &loaded.source.name,
                (Task::Prepare(source, action), Response::Prepared(p)) => {
                    source == &p.review.source && action == &p.review.action
                }
                (Task::Manage(review), Response::Managed(r)) => {
                    r.source == path(&review.source) && r.destination == review.destination
                }
                (Task::Reuse(source), Response::Reused(r)) => source == &r.source,
                _ => false,
            };
            if matches {
                Ok(response)
            } else {
                Err(Failure {
                    message: "Saved Setups response ownership is unconfirmed; Reload from file"
                        .into(),
                    teardown: false,
                    unconfirmed: true,
                })
            }
        });
        let mut confirmed = true;
        let outcome = match &result {
            Ok(Response::Managed(r)) if r.status != ManagementStatus::Succeeded => Err(format!(
                "{:?}: {}",
                r.status,
                r.message
                    .as_deref()
                    .unwrap_or("Inspect saved setup storage")
            )),
            Ok(_) => Ok(OperationOutcome::Succeeded),
            Err(e) => Err(e.message.clone()),
        };
        match result {
            Ok(Response::Browse(listing)) => self.saved.listing = Some(listing),
            Ok(Response::Loaded(loaded)) => {
                if matches!(task, Task::Load(_, LoadPurpose::Reload { .. })) {
                    self.saved.reload_sources.remove(&path(&loaded.source));
                }
                self.saved.selected = Some(loaded.clone());
                self.saved.naming = None;
                self.saved.prepared = None;
                if matches!(
                    task,
                    Task::Load(
                        _,
                        LoadPurpose::Reload {
                            replace_draft: true
                        }
                    )
                ) {
                    if let Err(e) = self.saved.begin_edit(loaded) {
                        self.saved.error = Some(e.to_string());
                    }
                } else {
                    self.saved.editing = false;
                }
            }
            Ok(Response::Prepared(prepared)) => {
                if let Task::Prepare(source, action) = &task {
                    if prepared.review.source == *source && prepared.review.action == *action {
                        self.saved.prepared = Some(prepared);
                        self.saved.naming = None;
                        self.saved.step = Step::Review;
                    } else {
                        self.saved.error =
                            Some("Review ownership changed; Reload from file".into());
                        self.saved.reload_sources.insert(path(source));
                    }
                }
            }
            Ok(Response::Managed(result)) => {
                confirmed = matches!(
                    result.status,
                    ManagementStatus::Succeeded
                        | ManagementStatus::Failed
                        | ManagementStatus::Partial
                );
                self.activity.append(&format!(
                    "Saved setup {}: {:?}. {}",
                    result.source.display(),
                    result.status,
                    result.message.as_deref().unwrap_or("Operation confirmed")
                ));
                if let Task::Manage(review) = &task {
                    if result.source != path(&review.source)
                        || result.destination != review.destination
                    {
                        self.saved.reload_sources.insert(path(&review.source));
                        self.saved.error =
                            Some("Result ownership is unconfirmed; Reload from file".into());
                        confirmed = false;
                    } else {
                        self.apply_saved_result(review, &result);
                    }
                }
                self.saved.prepared = None;
                self.saved.result = Some(result);
            }
            Ok(Response::Reused(reused)) => {
                let handle = self.window;
                let owner = cx.entity().downgrade();
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, w, cx| {
                        let _ =
                            owner.update(cx, |this, cx| this.offer_saved_handoff(reused, w, cx));
                    });
                });
            }
            Err(e) => {
                confirmed = !e.teardown && !e.unconfirmed;
                self.saved.teardown_uncertain |= e.teardown;
                if (e.teardown
                    || e.message.to_lowercase().contains("reload")
                    || e.message.to_lowercase().contains("changed"))
                    && let Some(source) = task.source()
                {
                    self.saved.reload_sources.insert(path(source));
                }
                self.saved.error = Some(e.message);
            }
        }
        self.operations.finish(id, &outcome);
        self.running = None;
        self.activity.flush();
        self.saved_list.reset(self.saved.row_count());
        self.saved.last_confirmed = Some((id, confirmed));
        if self.saved.close_pending {
            self.saved.close_pending = false;
            if confirmed {
                self.continue_close(cx);
            } else {
                self.operations.expanded = true;
            }
        }
        cx.notify();
    }
    fn apply_saved_result(&mut self, review: &ManagementReview, result: &ManagementOutcome) {
        if result.status != ManagementStatus::Succeeded {
            if result.status != ManagementStatus::Failed
                || result
                    .message
                    .as_ref()
                    .is_some_and(|m| m.to_lowercase().contains("reload"))
            {
                self.saved.reload_sources.insert(path(&review.source));
            }
            return;
        }
        let same_draft = self
            .saved
            .draft
            .as_ref()
            .is_some_and(|d| path(&d.loaded.source) == result.source);
        if matches!(review.action, SetupAction::Delete) {
            if same_draft {
                self.saved.draft = None;
                self.saved.editing = false;
            }
            if self
                .saved
                .selected
                .as_ref()
                .is_some_and(|s| path(&s.source) == result.source)
            {
                self.saved.selected = None;
            }
        }
        if let Some(saved) = &result.saved {
            let loaded = LoadedSetup::new(saved.clone());
            self.saved.selected = Some(loaded.clone());
            if same_draft
                && !matches!(review.action, SetupAction::Duplicate(_))
                && let Some(graph) = &loaded.graph
            {
                self.saved.draft = SavedSetupDraft::new(graph.clone())
                    .ok()
                    .map(|draft| Editing {
                        loaded: loaded.clone(),
                        draft,
                    });
            }
            if let Some(listing) = &mut self.saved.listing {
                listing.entries.retain(|e| e.path != path(saved));
                listing.entries.push(SetupEntry {
                    name: saved.name.clone(),
                    path: path(saved),
                    summary: "Confirmed saved composition".into(),
                    warning: loaded.problem.or(loaded.reuse_problem),
                });
                listing.entries.sort_by_key(|e| e.name.to_lowercase());
            }
        }
        if matches!(review.action, SetupAction::Rename(_) | SetupAction::Delete)
            && review.destination.as_ref() != Some(&result.source)
            && let Some(listing) = &mut self.saved.listing
        {
            listing.entries.retain(|e| e.path != result.source);
        }
        self.saved.reload_sources.remove(&result.source);
    }
    pub(super) fn confirm_saved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some() || self.teardown_uncertain() || self.saved.dialog {
            return;
        }
        let Some(review) = self.saved.ready().cloned() else {
            return;
        };
        let revision = self.saved.revision;
        self.saved.dialog = true;
        let owner = cx.entity().downgrade();
        let (title, keep_label, apply_label) = match review.action {
            SetupAction::Save(_) => ("Save setup changes?", "Keep reviewing", "Save changes"),
            SetupAction::Delete => ("Permanently delete setup?", "Keep setup", "Delete setup"),
            SetupAction::Rename(_) => ("Rename setup?", "Keep setup", "Rename setup"),
            SetupAction::Duplicate(_) => ("Duplicate setup?", "Keep setup", "Duplicate setup"),
        };
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone();
            let apply = owner.clone();
            let closed = owner.clone();
            let review = review.clone();
            dialog
                .title(title)
                .child(format!(
                    "Setup: {}. Source: {}. {} Existing projects and Watch are unaffected. {}",
                    review.source.name,
                    path(&review.source).display(),
                    review
                        .destination
                        .as_ref()
                        .map(|p| format!("Destination: {}.", p.display()))
                        .unwrap_or_default(),
                    if matches!(review.action, SetupAction::Delete) {
                        "This permanently removes the saved document."
                    } else {
                        "Only the reviewed document operation will run; no tools are installed."
                    }
                ))
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("saved-keep-reviewing")
                                .primary()
                                .label(keep_label)
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let _ = keep.update(cx, |this, cx| this.dismiss_saved(w, cx));
                                }),
                        )
                        .child(Button::new("saved-confirm").label(apply_label).on_click(
                            move |_, w, cx| {
                                w.close_dialog(cx);
                                let _ = apply.update(cx, |this, cx| {
                                    this.saved.dialog = false;
                                    if this.saved.revision == revision
                                        && this.saved.ready() == Some(&review)
                                    {
                                        this.start_saved(
                                            Task::Manage(Box::new(review.clone())),
                                            cx,
                                        );
                                    } else {
                                        this.saved.error =
                                            Some("Confirmation expired; review again".into());
                                    }
                                    this.focus_saved(w, cx);
                                });
                            },
                        )),
                )
                .on_close(move |_, w, cx| {
                    let _ = closed.update(cx, |this, cx| this.dismiss_saved(w, cx));
                })
        });
    }
    pub(super) fn saved_workflow(
        &mut self,
        workflow: PackageWorkflow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.saved.editable() || self.saved.dialog || self.running.is_some() {
            return;
        }
        let Some(draft) = self.saved.draft.as_ref() else {
            return;
        };
        if draft.draft.graph.package_workflow == workflow {
            return;
        }
        self.saved.dialog = true;
        let revision = self.saved.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx,move |dialog,_,_|{let keep=owner.clone();let change=owner.clone();let closed=owner.clone();
            dialog.title("Change dependency workflow?").child("Changing workflow clears package selections. Incompatible testing stays selected until you explicitly choose a compatible implementation or None; saving remains blocked until then. File exclusions and provenance are retained.")
                .footer(DialogFooter::new().child(Button::new("saved-keep-workflow").primary().label("Keep workflow").on_click(move |_,w,cx|{w.close_dialog(cx);let _=keep.update(cx,|this,cx|this.dismiss_saved(w,cx));}))
                    .child(Button::new("saved-change-workflow").label("Change workflow").on_click(move |_,w,cx|{w.close_dialog(cx);let _=change.update(cx,|this,cx|{this.saved.dialog=false;if this.saved.revision==revision && this.running.is_none(){this.saved.edit(|d|d.set_workflow(workflow));}this.dismiss_saved(w,cx);});})))
                .on_close(move |_,w,cx|{let _=closed.update(cx,|this,cx|this.dismiss_saved(w,cx));})});
    }
    pub(super) fn offer_saved_handoff(
        &mut self,
        reused: ReusedSetup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() {
            return;
        }
        let occupied = self.creation.draft.as_ref().is_some_and(|d| {
            !d.name.is_empty()
                || d.save_setup.is_some()
                || d.graph != rproj_core::creation::CreationDraft::new(String::new()).graph
        });
        if !occupied {
            self.apply_saved_handoff(reused, window, cx);
            return;
        }
        self.saved.dialog = true;
        let revision = self.creation.revision;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx,move |dialog,_,_|{let keep=owner.clone();let replace=owner.clone();let closed=owner.clone();let reused=reused.clone();
            dialog.title("Keep the New Project draft?").child(format!("Using {} replaces the current creation draft. It starts at Project and creates nothing automatically.",reused.source.name))
                .footer(DialogFooter::new().child(Button::new("saved-keep-creation").primary().label("Keep draft").on_click(move |_,w,cx|{w.close_dialog(cx);let _=keep.update(cx,|this,cx|this.dismiss_saved(w,cx));}))
                    .child(Button::new("saved-replace-creation").label("Replace creation draft").on_click(move |_,w,cx|{w.close_dialog(cx);let _=replace.update(cx,|this,cx|{this.saved.dialog=false;if this.creation.revision==revision && this.running.is_none(){this.apply_saved_handoff(reused.clone(),w,cx);}else{this.dismiss_saved(w,cx);}});})))
                .on_close(move |_,w,cx|{let _=closed.update(cx,|this,cx|this.dismiss_saved(w,cx));})});
    }
    fn apply_saved_handoff(
        &mut self,
        reused: ReusedSetup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.creation.draft = Some(rproj_core::creation::CreationDraft {
            name: String::new(),
            graph: reused.graph,
            save_setup: None,
        });
        self.creation.selected_setup = Some(reused.source.name);
        self.creation.setup_source = Some(rproj_core::creation::protocol::SourceSnapshot {
            bytes: Some(reused.source.bytes),
        });
        self.creation.step = creation::Step::Project;
        self.creation.result = None;
        self.creation.selection_problem = None;
        self.creation.changed();
        self.creation_inputs
            .name
            .update(cx, |i, cx| i.set_value("", window, cx));
        self.creation_inputs
            .setup
            .update(cx, |i, cx| i.set_value("", window, cx));
        self.show_creation(window, cx);
        self.load_creation_context(cx);
    }
    pub(in super::super) fn saved_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.saved.dialog || self.saved.close_pending || self.saved.teardown_uncertain {
            return Some(false);
        }
        if let Some(task) = &self.saved.pending {
            let id = self.operations.latest.as_ref()?.id;
            let mutation = matches!(task, Task::Manage(_));
            let owner_path = task
                .source()
                .map(path)
                .unwrap_or_else(|| PathBuf::from("Saved Setups"));
            self.saved.dialog = true;
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx,move |dialog,_,_|{let keep=owner.clone();let close=owner.clone();let dismissed=owner.clone();
                dialog.title("Saved Setups is still working").child(format!("Captured owner: {}. {} Closing waits for confirmed completion and process teardown, then protects other drafts. Watch remains running.",owner_path.display(),if mutation{"This operation cannot be cancelled; partial file changes may remain."}else{"Reading or review preparation changes no documents."}))
                    .footer(DialogFooter::new().child(Button::new("saved-keep-open").primary().label("Keep open").on_click(move |_,w,cx|{w.close_dialog(cx);let _=keep.update(cx,|this,cx|this.dismiss_saved(w,cx));}))
                        .child(Button::new("saved-close-after").label("Close after operation").on_click(move |_,w,cx|{w.close_dialog(cx);let _=close.update(cx,|this,cx|{this.saved.dialog=false;if this.operations.accepts(id) && this.saved.pending.is_some(){this.saved.close_pending=true;}else if this.saved.last_confirmed==Some((id,true)){this.continue_close(cx);}else{this.saved.error=Some("Completion is unconfirmed; inspect and reload before closing".into());}this.focus_saved(w,cx);cx.notify();});})))
                    .on_close(move |_,w,cx|{let _=dismissed.update(cx,|this,cx|this.dismiss_saved(w,cx));})});
            return Some(false);
        }
        if self.running.is_none() && self.saved.dirty() {
            self.request_saved(Intent::Discard(true), window, cx);
            return Some(false);
        }
        None
    }
}
