use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter, input::Input};
use gpui_kit::prelude::FluentBuilder;

#[derive(Clone)]
pub(super) enum Replace {
    Load(PathBuf, String),
    Discard(bool),
}

impl Workbench {
    pub(in super::super) fn settings_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| {
            matches!(
                op,
                Operation::LoadSettings | Operation::PrepareSettings | Operation::SaveSettings
            )
        })
    }
    pub(in super::super) fn focus_settings(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.visible {
            self.settings_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn probe_settings(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::SettingsSupport(
                client.settings_support().map_err(|e| format!("{e:#}")),
            ));
        });
    }
    pub(in super::super) fn show_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.creation.visible = false;
        self.composition.visible = false;
        self.upgrade.visible = false;
        self.settings.visible = true;
        self.focus_settings(window, cx);
        cx.notify();
    }
    pub(in super::super) fn edit_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.settings.project.as_ref() == Some(&project) {
            self.show_settings(window, cx);
        } else {
            self.replace_settings(Replace::Load(project, "stylua".into()), window, cx);
        }
    }
    pub(super) fn replace_settings(
        &mut self,
        action: Replace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.settings.dialog_open {
            return;
        }
        if !self.settings.dirty() {
            self.perform_settings_replace(action, window, cx);
            return;
        }
        self.settings.dialog_open = true;
        let revision = self.settings.revision;
        let project = self.settings.project.clone();
        let label = match action {
            Replace::Load(..) => "Replace or reload draft",
            Replace::Discard(_) => "Discard draft",
        };
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let replace = owner.clone(); let closed = owner.clone();
            let action = action.clone(); let project = project.clone();
            dialog.title("Keep the captured settings draft?")
                .child(format!("Unsaved settings for {} remain session-only. Replacing, reloading or discarding loses these edits.", project.as_ref().map(|p| p.display().to_string()).unwrap_or_default()))
                .footer(DialogFooter::new()
                    .child(Button::new("settings-keep-draft").primary().label("Keep draft").on_click(move |_, window, cx| {
                        window.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_settings_dialog(window, cx));
                    }))
                    .child(Button::new("settings-replace-draft").label(label).on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        let _ = replace.update(cx, |this, cx| {
                            this.settings.dialog_open = false;
                            if this.settings.revision == revision && this.settings.project == project && this.running.is_none() {
                                this.perform_settings_replace(action.clone(), window, cx);
                            }
                            this.focus_settings(window, cx);
                        });
                    })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| this.finish_settings_dialog(window, cx)); })
        });
    }
    fn perform_settings_replace(
        &mut self,
        action: Replace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            Replace::Load(project, tool) => self.load_settings(project, tool, window, cx),
            Replace::Discard(closing) => {
                let support = self.settings.support.take();
                let revision = self.settings.revision.wrapping_add(1);
                self.settings = SettingsSession {
                    support,
                    revision,
                    ..Default::default()
                };
                self.settings_list.reset(0);
                self.focus_settings(window, cx);
                if closing {
                    self.continue_close(cx);
                }
                cx.notify();
            }
        }
    }
    fn finish_settings_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.settings.dialog_open = false;
        self.focus_settings(window, cx);
        cx.notify();
    }
    fn settings_operation(
        &mut self,
        operation: Operation,
    ) -> Option<(u64, Cancellation, WorkbenchClient)> {
        if self.running.is_some() || self.teardown_uncertain() {
            return None;
        }
        let project = self.settings.project.clone()?;
        let client = self.client.clone()?;
        let token = Cancellation::default();
        let id = self.operations.begin(operation, project.clone());
        self.pending_switch = None;
        self.activity.set_origin(Some(format!(
            "{} · {}",
            project.display(),
            operation_label(operation)
        )));
        self.running = Some((operation, token.clone()));
        self.settings.loading = true;
        Some((id, token, client))
    }
    fn load_settings(
        &mut self,
        project: PathBuf,
        tool: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || !matches!(self.settings.support, Some(Ok(())))
        {
            return;
        }
        let support = self.settings.support.take();
        let revision = self.settings.revision.wrapping_add(1);
        self.settings = SettingsSession {
            support,
            revision,
            project: Some(project.clone()),
            tool: tool.clone(),
            ..Default::default()
        };
        self.show_settings(window, cx);
        self.settings_list.reset(0);
        let Some((id, token, client)) = self.settings_operation(Operation::LoadSettings) else {
            return;
        };
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .settings_context(&project, &tool, &token, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(ReadFailure::from);
            let _ = sender.send(Event::SettingsLoaded(id, revision, result));
        });
        cx.notify();
    }
    fn finish_settings_operation(&mut self, id: u64, outcome: &Result<OperationOutcome, String>) {
        self.operations.finish(id, outcome);
        self.running = None;
        self.settings.loading = false;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
    }
    pub(in super::super) fn accept_settings_loaded(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<SettingsLoaded, ReadFailure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let outcome = result
            .as_ref()
            .map(|_| OperationOutcome::Succeeded)
            .map_err(|e| e.message.clone());
        self.finish_settings_operation(id, &outcome);
        if self.settings.revision == revision {
            match result {
                Ok(loaded) => {
                    if let Err(error) = loaded.validate() {
                        self.settings.error = Some(format!("Invalid settings context: {error:#}"));
                    } else if self
                        .operations
                        .latest
                        .as_ref()
                        .is_some_and(|run| Some(&run.project) == self.settings.project.as_ref())
                        && loaded.context.tool == self.settings.tool
                    {
                        // The client validates the response against the canonical request.
                        // The picker path can still be noncanonical while inspection runs.
                        self.settings.project = Some(loaded.context.project.clone());
                        self.settings.loaded = Some(loaded);
                        self.settings.error = None;
                    } else {
                        self.settings.error = Some("Settings response belongs to another project/target; reload explicitly.".into());
                    }
                }
                Err(error) => {
                    self.settings.teardown_uncertain |= error.teardown;
                    self.settings.error = Some(error.message);
                }
            }
        }
        self.finish_settings_close(!self.settings.teardown_uncertain, cx);
        cx.notify();
    }
    pub(super) fn prepare_settings(&mut self, cx: &mut Context<Self>) {
        if self.settings.reload_required || self.settings.dialog_open {
            return;
        }
        let Some(loaded) = self.settings.loaded.clone() else {
            return;
        };
        if !loaded.draft.dirty() {
            return;
        }
        let Some((id, token, client)) = self.settings_operation(Operation::PrepareSettings) else {
            return;
        };
        self.settings.error = None;
        self.settings.prepared = None;
        let revision = self.settings.revision;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .prepare_settings(loaded.context, loaded.draft, &token, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .and_then(|review| {
                    Ok(Prepared {
                        rows: Arc::new(review.diff()?),
                        review,
                    })
                })
                .map_err(ReadFailure::from);
            let _ = sender.send(Event::SettingsPrepared(id, revision, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_settings_prepared(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<Prepared, ReadFailure>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let outcome = result
            .as_ref()
            .map(|_| OperationOutcome::Succeeded)
            .map_err(|e| e.message.clone());
        self.finish_settings_operation(id, &outcome);
        if self.settings.revision == revision {
            match result {
                Ok(prepared) => {
                    if self.settings.loaded.as_ref().is_some_and(|l| {
                        l.context == prepared.review.context && l.draft == prepared.review.draft
                    }) {
                        self.settings.prepared = Some(prepared);
                        self.settings.reviewing = true;
                        self.settings.error = None;
                    } else {
                        self.settings.error =
                            Some("Review no longer matches this draft; reload explicitly.".into());
                        self.settings.reload_required = true;
                    }
                }
                Err(error) => {
                    self.settings.teardown_uncertain |= error.teardown;
                    // Cancellation, spawn failures and writer refusals preserve a usable
                    // draft. Only a stale captured source requires explicit replacement.
                    self.settings.reload_required =
                        error.teardown || error.message.contains("Reload from file");
                    self.settings.error = Some(error.message);
                }
            }
            self.settings_list.reset(self.settings.row_count());
        }
        self.finish_settings_close(!self.settings.teardown_uncertain, cx);
        cx.notify();
    }
    pub(super) fn confirm_settings_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || self.settings.dialog_open
            || self.settings.watch_conflict(self.watch.as_ref())
        {
            return;
        }
        let Some(review) = self.settings.ready().cloned() else {
            return;
        };
        self.settings.dialog_open = true;
        let revision = self.settings.revision;
        let generation = self.watch_generation;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let save = owner.clone(); let closed = owner.clone(); let review = review.clone();
            dialog.title("Save settings?")
                .child(format!("Project: {}. Target: {}. File: {}. Save only the reviewed settings and writer formatting/cleanup effects. Saving does not install or enable a tool. Watch stays unchanged.", review.context.project.display(), review.context.target().unwrap().display_name, review.context.target().unwrap().target.relative()))
                .footer(DialogFooter::new()
                    .child(Button::new("settings-keep-reviewing").primary().label("Keep reviewing").on_click(move |_, window, cx| {
                        window.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_settings_dialog(window, cx));
                    }))
                    .child(Button::new("settings-confirm-save").label("Save settings").on_click(move |_, window, cx| {
                        window.close_dialog(cx);
                        let _ = save.update(cx, |this, cx| {
                            this.settings.dialog_open = false;
                            if this.settings.revision == revision && this.settings.ready() == Some(&review)
                                && this.watch_generation == generation && !this.settings.watch_conflict(this.watch.as_ref()) {
                                this.save_settings(review.clone(), cx);
                            } else { this.settings.error = Some("Confirmation expired because settings or Watch changed. Review again.".into()); }
                            this.focus_settings(window, cx); cx.notify();
                        });
                    })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| this.finish_settings_dialog(window, cx)); })
        });
    }
    fn save_settings(&mut self, review: SettingsReview, cx: &mut Context<Self>) {
        let Some((id, _, client)) = self.settings_operation(Operation::SaveSettings) else {
            return;
        };
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            // The client converts pre-spawn errors to Failed/NotStarted outcomes.
            // Its only remaining Err means owned process teardown is unconfirmed.
            let result = client
                .save_settings(review, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::SettingsSaved(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_settings_saved(
        &mut self,
        id: u64,
        result: Result<SettingsOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let project = self.operations.latest.as_ref().unwrap().project.clone();
        let result = result.and_then(|r| {
            if r.project != project || r.tool != self.settings.tool {
                Err("Save response ownership is unconfirmed".into())
            } else {
                Ok(r)
            }
        });
        let confirmed = result
            .as_ref()
            .is_ok_and(|r| matches!(r.status, SettingsStatus::Succeeded | SettingsStatus::Failed));
        let outcome = match &result {
            Ok(r) if r.status == SettingsStatus::Succeeded => Ok(OperationOutcome::Succeeded),
            Ok(r) => Err(format!(
                "{:?}. {}",
                r.status,
                r.message.as_deref().unwrap_or("Inspect the captured file.")
            )),
            Err(e) => Err(e.clone()),
        };
        self.activity.append(&format!(
            "Captured project: {}. {}",
            project.display(),
            result
                .as_ref()
                .map(|r| format!(
                    "{:?}; {:?}. {}",
                    r.status,
                    r.writes,
                    r.message.as_deref().unwrap_or("")
                ))
                .unwrap_or_else(|e| format!("Teardown unconfirmed: {e}. The window stays open."))
        ));
        self.finish_settings_operation(id, &outcome);
        match &result {
            Ok(r) if r.status == SettingsStatus::Succeeded => {
                if let Some(saved) = &r.saved {
                    self.settings.loaded = Some(saved.clone());
                    self.settings.consent.clear();
                    self.settings.revision = self.settings.revision.wrapping_add(1);
                } else {
                    self.settings.reload_required = true;
                }
            }
            Ok(r) => {
                self.settings.reload_required = r.status == SettingsStatus::Uncertain
                    || r.message
                        .as_ref()
                        .is_some_and(|s| s.contains("Reload from file"));
            }
            Err(_) => self.settings.teardown_uncertain = true,
        }
        self.settings.prepared = None;
        self.settings.reviewing = false;
        self.settings.result = Some(result);
        if self.project.as_ref() == Some(&project) {
            self.inspect();
        }
        self.poll_status();
        self.finish_settings_close(confirmed, cx);
        cx.notify();
    }
    pub(super) fn consent_settings_value(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some()
            || self.settings.dialog_open
            || self.settings.reload_required
            || self.teardown_uncertain()
        {
            return;
        }
        let Some(loaded) = &self.settings.loaded else {
            return;
        };
        if loaded.blocked.contains_key(&index) {
            return;
        }
        let Some(value) = loaded.draft.current.get(index).and_then(|v| v.as_ref()) else {
            return;
        };
        let setting = &loaded.context.target().unwrap().settings[index];
        let message = format!(
            "{} currently contains {value}. Keep it unless you explicitly want to choose a supported value. Consent alone changes no file or draft value.",
            setting.display_key()
        );
        let revision = self.settings.revision;
        self.settings.dialog_open = true;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone();
            let replace = owner.clone();
            let closed = owner.clone();
            dialog
                .title("Replace the unfamiliar value?")
                .child(message.clone())
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("settings-keep-existing")
                                .primary()
                                .label("Keep existing")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = keep.update(cx, |this, cx| {
                                        this.finish_settings_dialog(window, cx)
                                    });
                                }),
                        )
                        .child(
                            Button::new("settings-replace-existing")
                                .label("Choose a supported value")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = replace.update(cx, |this, cx| {
                                        if this.settings.revision == revision {
                                            this.settings.consent.insert(index);
                                        }
                                        this.finish_settings_dialog(window, cx);
                                    });
                                }),
                        ),
                )
                .on_close(move |_, window, cx| {
                    let _ = closed.update(cx, |this, cx| this.finish_settings_dialog(window, cx));
                })
        });
    }
    pub(super) fn edit_settings_integer(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.settings.dialog_open || !self.settings.editable(index) {
            return;
        }
        let loaded = self.settings.loaded.as_ref().unwrap();
        let setting = &loaded.context.target().unwrap().settings[index];
        let name = setting.display_key();
        let value = loaded
            .draft
            .value(index)
            .and_then(|v| v.as_i64())
            .map(|v| v.to_string())
            .unwrap_or_default();
        self.settings_integer
            .update(cx, |input, cx| input.set_value(value, window, cx));
        let input = self.settings_integer.clone();
        let revision = self.settings.revision;
        self.settings.dialog_open = true;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, cx| {
            let keep = owner.clone();
            let choose = owner.clone();
            let closed = owner.clone();
            let valid = input.read(cx).value().trim().parse::<i64>().is_ok();
            let input = input.clone();
            dialog
                .title(format!("Choose {name}"))
                .child(
                    "Enter an exact whole number from −9223372036854775808 to 9223372036854775807.",
                )
                .child(
                    Input::new(&input)
                        .id("settings-integer-input")
                        .aria_label(name.clone()),
                )
                .when(!valid, |dialog| {
                    dialog.child("A signed 64-bit integer is required.")
                })
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("settings-keep-integer")
                                .primary()
                                .label("Keep existing")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = keep.update(cx, |this, cx| {
                                        this.finish_settings_dialog(window, cx)
                                    });
                                }),
                        )
                        .child(
                            Button::new("settings-choose-integer")
                                .label("Use value")
                                .disabled(!valid)
                                .on_click(move |_, window, cx| {
                                    let value = input.read(cx).value().trim().parse::<i64>();
                                    if let Ok(value) = value {
                                        window.close_dialog(cx);
                                        let _ = choose.update(cx, |this, cx| {
                                            if this.settings.revision == revision
                                                && this.running.is_none()
                                            {
                                                this.settings.set(index, serde_json::json!(value));
                                            }
                                            this.finish_settings_dialog(window, cx);
                                        });
                                    }
                                }),
                        ),
                )
                .on_close(move |_, window, cx| {
                    let _ = closed.update(cx, |this, cx| this.finish_settings_dialog(window, cx));
                })
        });
    }
    pub(in super::super) fn settings_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.settings.teardown_uncertain
            || self.settings.dialog_open
            || self.settings.close_pending
        {
            return Some(false);
        }
        if self.settings_running() {
            let saving = matches!(self.running, Some((Operation::SaveSettings, _)));
            let id = self.operations.latest.as_ref().unwrap().id;
            let project = self
                .settings
                .project
                .as_ref()
                .unwrap()
                .display()
                .to_string();
            self.settings.dialog_open = true;
            let owner = cx.entity().downgrade();
            window.open_alert_dialog(cx, move |dialog, _, _| {
                let keep = owner.clone(); let close = owner.clone(); let dismissed = owner.clone();
                dialog.title(if saving { "Settings are still saving" } else { "Settings preparation is running" })
                    .child(format!("Project: {project}. {} Closing waits for confirmed completion and process teardown, then checks other unsaved drafts. Unconfirmed completion keeps this window open. Detached Watch stays running.", if saving { "Save cannot be cancelled; failure may leave file changes." } else { "Preparation makes no project changes." }))
                    .footer(DialogFooter::new()
                        .child(Button::new("settings-keep-open").primary().label("Keep open").on_click(move |_, window, cx| {
                            window.close_dialog(cx); let _ = keep.update(cx, |this, cx| this.finish_settings_dialog(window, cx));
                        }))
                        .child(Button::new("settings-close-after-save").label(if saving { "Close after save" } else { "Close after preparation" }).on_click(move |_, window, cx| {
                            window.close_dialog(cx); let _ = close.update(cx, |this, cx| {
                                this.settings.dialog_open = false;
                                if this.operations.accepts(id) && this.settings_running() { this.settings.close_pending = true; }
                                else if this.operations.latest.as_ref().is_some_and(|run| run.id == id)
                                    && !this.settings.teardown_uncertain && this.settings.result.as_ref().is_some_and(|r| r.as_ref().is_ok_and(|r| matches!(r.status, SettingsStatus::Succeeded | SettingsStatus::Failed))) {
                                    this.continue_close(cx);
                                } else { this.settings.error = Some("Review the completed or unconfirmed operation before closing.".into()); }
                                this.focus_settings(window, cx); cx.notify();
                            });
                        })))
                    .on_close(move |_, window, cx| { let _ = dismissed.update(cx, |this, cx| this.finish_settings_dialog(window, cx)); })
            });
            return Some(false);
        }
        if self.running.is_none() && self.settings.dirty() {
            self.replace_settings(Replace::Discard(true), window, cx);
            return Some(false);
        }
        None
    }
    fn finish_settings_close(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if self.settings.close_pending {
            self.settings.close_pending = false;
            if confirmed {
                self.continue_close(cx);
            } else {
                self.operations.expanded = true;
            }
        }
    }
}
