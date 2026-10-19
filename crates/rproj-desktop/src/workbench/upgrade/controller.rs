use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

impl Workbench {
    pub(in super::super) fn focus_upgrade(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.upgrade.visible {
            self.upgrade_focus.focus(window, cx);
        } else {
            self.focus.focus(window, cx);
        }
    }
    pub(in super::super) fn teardown_uncertain(&self) -> bool {
        self.creation.teardown_uncertain
            || self.composition.teardown_uncertain
            || self.upgrade.teardown_uncertain
    }
    pub(in super::super) fn upgrade_running(&self) -> bool {
        self.running.as_ref().is_some_and(|(op, _)| {
            matches!(op, Operation::PrepareUpgrade | Operation::ApplyUpgrade)
        })
    }
    pub(in super::super) fn probe_upgrade(&self) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let _ = sender.send(Event::UpgradeSupport(
                client.upgrade_support().map_err(|e| format!("{e:#}")),
            ));
        });
    }
    pub(in super::super) fn show_upgrade(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.creation.visible = false;
        self.composition.visible = false;
        self.upgrade.visible = true;
        self.focus_upgrade(window, cx);
        cx.notify();
    }
    pub(in super::super) fn edit_upgrade(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.upgrade.project.as_ref() == Some(&project) {
            self.show_upgrade(window, cx);
        } else if self.upgrade.project.is_some() {
            self.confirm_upgrade_refresh(project, window, cx);
        } else {
            self.prepare_upgrade(project, window, cx);
        }
    }
    pub(in super::super) fn confirm_upgrade_refresh(
        &mut self,
        project: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some() || self.teardown_uncertain() || self.upgrade.dialog_open {
            return;
        }
        self.upgrade.dialog_open = true;
        let revision = self.upgrade.revision;
        let captured = self.upgrade.project.clone();
        let message = format!(
            "Keep the captured review for {} or explicitly prepare a fresh review for {}. Source changes are never merged automatically. Activity and other drafts stay available.",
            captured
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            project.display()
        );
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone();
            let refresh = owner.clone();
            let closed = owner.clone();
            let project = project.clone();
            let captured = captured.clone();
            dialog
                .title("Replace the captured Upgrade review?")
                .child(message.clone())
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("upgrade-keep-review")
                                .primary()
                                .label("Keep review")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = keep.update(cx, |this, cx| {
                                        this.upgrade.dialog_open = false;
                                        this.focus_upgrade(window, cx);
                                    });
                                }),
                        )
                        .child(
                            Button::new("upgrade-confirm-refresh")
                                .label("Refresh review")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = refresh.update(cx, |this, cx| {
                                        this.upgrade.dialog_open = false;
                                        if this.upgrade.revision == revision
                                            && this.upgrade.project == captured
                                        {
                                            this.prepare_upgrade(project.clone(), window, cx);
                                        }
                                    });
                                }),
                        ),
                )
                .on_close(move |_, window, cx| {
                    let _ = closed.update(cx, |this, cx| {
                        this.upgrade.dialog_open = false;
                        this.focus_upgrade(window, cx);
                    });
                })
        });
    }
    fn upgrade_operation(
        &mut self,
        operation: Operation,
    ) -> Option<(u64, Cancellation, WorkbenchClient)> {
        if self.running.is_some() || self.teardown_uncertain() {
            return None;
        }
        let project = self.upgrade.project.clone()?;
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
        Some((id, token, client))
    }
    fn prepare_upgrade(&mut self, project: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || self.client.is_none()
            || !matches!(self.upgrade.support, Some(Ok(())))
        {
            return;
        }
        let name = if self.project.as_ref() == Some(&project) {
            self.snapshot.as_ref().and_then(|p| p.name.clone())
        } else {
            self.upgrade.project_name.clone()
        };
        self.upgrade.begin(project.clone());
        self.upgrade.project_name = name;
        self.show_upgrade(window, cx);
        self.upgrade_list.reset(0);
        let Some((id, token, client)) = self.upgrade_operation(Operation::PrepareUpgrade) else {
            return;
        };
        let revision = self.upgrade.revision;
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = match client.upgrade_context(&project) {
                Ok(context) => client
                    .prepare_upgrade(context, &token, move |event| {
                        if progress.try_send(Event::Progress(id, event)).is_err() {
                            dropped.fetch_add(1, Ordering::Relaxed);
                        }
                    })
                    .map(Preparation::from_worker)
                    .map_err(|e| format!("{e:#}")),
                Err(error) => Ok(Preparation::Failed(format!("{error:#}"))),
            };
            let _ = sender.send(Event::UpgradePrepared(id, revision, result));
        });
        cx.notify();
    }
    fn finish_upgrade_operation(&mut self, id: u64, outcome: &Result<OperationOutcome, String>) {
        self.operations.finish(id, outcome);
        self.running = None;
        self.activity.flush();
        self.activity.append(&self.operations.summary());
        self.activity.set_origin(None);
    }
    pub(in super::super) fn accept_upgrade_prepared(
        &mut self,
        id: u64,
        revision: u64,
        result: Result<Preparation, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let confirmed = result.is_ok();
        if !confirmed {
            self.upgrade.teardown_uncertain = true;
        }
        let outcome = match &result {
            Ok(Preparation::Prepared(_)) => Ok(OperationOutcome::Succeeded),
            Ok(Preparation::Cancelled) => Ok(OperationOutcome::Cancelled),
            Ok(Preparation::Failed(error)) | Err(error) => Err(error.clone()),
        };
        self.finish_upgrade_operation(id, &outcome);
        if self.upgrade.revision == revision {
            self.upgrade.loading = false;
            match result {
                Ok(Preparation::Prepared(prepared))
                    if Some(&prepared.review.project) == self.upgrade.project.as_ref() =>
                {
                    self.upgrade.prepared = Some(*prepared);
                    self.upgrade.error = None;
                }
                Ok(Preparation::Prepared(_)) => {
                    self.upgrade.error = Some(
                        "Review belongs to another captured project; explicitly Refresh review"
                            .into(),
                    )
                }
                Ok(Preparation::Cancelled) => {
                    self.upgrade.error = Some(
                        "Preparation cancelled; no project changes made. Refresh review to retry."
                            .into(),
                    )
                }
                Ok(Preparation::Failed(error)) => {
                    self.upgrade.error = Some(format!(
                        "{error}. Repair the indicated input, then explicitly Refresh review."
                    ))
                }
                Err(error) => {
                    self.upgrade.error = Some(format!(
                        "Worker teardown unconfirmed: {error}. The window must stay open."
                    ));
                    self.upgrade.teardown_uncertain = true;
                }
            }
            self.upgrade_list.reset(self.upgrade.row_count());
        }
        self.finish_upgrade_close(confirmed, cx);
        cx.notify();
    }
    pub(in super::super) fn confirm_upgrade_apply(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.running.is_some()
            || self.teardown_uncertain()
            || self.upgrade.dialog_open
            || self.upgrade.watch_conflict(self.watch.as_ref())
        {
            return;
        }
        let Some(review) = self
            .upgrade
            .ready()
            .filter(|p| !p.files.is_empty())
            .cloned()
        else {
            return;
        };
        self.upgrade.dialog_open = true;
        let revision = self.upgrade.revision;
        let generation = self.watch_generation;
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let apply = owner.clone(); let closed = owner.clone(); let review = review.clone();
            dialog.title("Apply all proposed configuration changes?")
                .child(format!("Project: {}. Apply all {} proposed files; {} safely skipped files stay unchanged. This refreshes generated configuration, not package/tool versions. Failure or cancellation may leave partial changes; no rollback is promised. Watch is never switched.", review.project.display(), review.files.len(), review.skipped.len()))
                .footer(DialogFooter::new()
                    .child(Button::new("upgrade-keep-reviewing").primary().label("Keep reviewing").on_click(move |_, window, cx| { window.close_dialog(cx); let _ = keep.update(cx, |this, cx| { this.upgrade.dialog_open = false; this.focus_upgrade(window, cx); }); }))
                    .child(Button::new("upgrade-confirm-apply").label("Apply all proposed changes").on_click(move |_, window, cx| { window.close_dialog(cx); let _ = apply.update(cx, |this, cx| {
                        this.upgrade.dialog_open = false;
                        if this.upgrade.revision == revision && this.upgrade.ready() == Some(&review) && this.watch_generation == generation && !this.upgrade.watch_conflict(this.watch.as_ref()) { this.apply_upgrade(review.clone(), cx); }
                        else { this.upgrade.error = Some("Confirmation expired because the review or Watch changed; explicitly Refresh review.".into()); cx.notify(); }
                        this.focus_upgrade(window, cx);
                    }); })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| { this.upgrade.dialog_open = false; this.focus_upgrade(window, cx); }); })
        });
    }
    fn apply_upgrade(&mut self, review: UpgradeReview, cx: &mut Context<Self>) {
        let Some((id, token, client)) = self.upgrade_operation(Operation::ApplyUpgrade) else {
            return;
        };
        let sender = self.sender.clone();
        let progress = sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let result = client
                .apply_upgrade(review, &token, move |event| {
                    if progress.try_send(Event::Progress(id, event)).is_err() {
                        dropped.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .map_err(|e| format!("{e:#}"));
            let _ = sender.send(Event::UpgradeApplied(id, result));
        });
        cx.notify();
    }
    pub(in super::super) fn accept_upgrade_result(
        &mut self,
        id: u64,
        result: Result<UpgradeOutcome, String>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id) {
            return;
        }
        let project = self.operations.latest.as_ref().unwrap().project.clone();
        let result = result.and_then(|r| {
            if r.project == project {
                Ok(r)
            } else {
                Err("Upgrade result has another project; teardown/outcome unconfirmed".into())
            }
        });
        let outcome = match &result {
            Ok(r) => match r.status {
                UpgradeStatus::Succeeded => Ok(OperationOutcome::Succeeded),
                UpgradeStatus::Cancelled => Ok(OperationOutcome::Cancelled),
                UpgradeStatus::Failed | UpgradeStatus::Uncertain => {
                    Err(r.message.clone().unwrap_or_else(|| {
                        "Upgrade failed or is uncertain; inspect possible partial changes".into()
                    }))
                }
            },
            Err(error) => Err(error.clone()),
        };
        self.activity.append(&format!("Captured project: {}. {}", project.display(), result.as_ref().map(|r| format!("{:?}; {:?}. {}", r.status, r.writes, r.message.as_deref().unwrap_or(""))).unwrap_or_else(|e| format!("Teardown unconfirmed: {e}. Inspect possible partial changes; the window stays open."))));
        self.finish_upgrade_operation(id, &outcome);
        if result.is_err() {
            self.upgrade.teardown_uncertain = true;
        }
        self.upgrade.result = Some(result);
        if self.project.as_ref() == Some(&project) {
            self.inspect();
        }
        self.poll_status();
        self.finish_upgrade_close(!self.upgrade.teardown_uncertain, cx);
        cx.notify();
    }
    pub(in super::super) fn upgrade_close(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.upgrade.teardown_uncertain || self.upgrade.close_pending || self.upgrade.dialog_open
        {
            return Some(false);
        }
        if !self.upgrade_running() {
            return None;
        }
        self.upgrade.dialog_open = true;
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
            .is_some_and(|(op, _)| *op == Operation::ApplyUpgrade);
        let owner = cx.entity().downgrade();
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone(); let cancel = owner.clone(); let closed = owner.clone();
            dialog.title(if applying { "Configuration changes are still running" } else { "Upgrade preparation is still running" })
                .child(format!("Project: {project}. {} Closing waits for worker/descendant teardown; detached Watch stays running. Other dirty drafts still require discard confirmation.", if applying { "Cancellation may leave partial changes; no rollback is promised." } else { "Preparation makes no project changes." }))
                .footer(DialogFooter::new()
                    .child(Button::new("upgrade-keep-open").primary().label("Keep open").on_click(move |_, window, cx| { window.close_dialog(cx); let _ = keep.update(cx, |this, cx| { this.upgrade.dialog_open = false; this.focus_upgrade(window, cx); }); }))
                    .child(Button::new("upgrade-cancel-close").label(if applying { "Cancel changes and close" } else { "Cancel preparation and close" }).on_click(move |_, window, cx| { window.close_dialog(cx); let _ = cancel.update(cx, |this, cx| {
                        this.upgrade.dialog_open = false;
                        if this.operations.accepts(id) && this.upgrade_running() { this.upgrade.close_pending = true; if let Some((_, token)) = &this.running { token.cancel(); } this.operations.cancel(); }
                        else { this.upgrade.error = Some("Operation finished while confirmation was open; review the result before closing".into()); }
                        cx.notify();
                    }); })))
                .on_close(move |_, window, cx| { let _ = closed.update(cx, |this, cx| { this.upgrade.dialog_open = false; this.focus_upgrade(window, cx); }); })
        });
        Some(false)
    }
    fn finish_upgrade_close(&mut self, confirmed: bool, cx: &mut Context<Self>) {
        if self.upgrade.close_pending {
            self.upgrade.close_pending = false;
            if confirmed {
                self.continue_close(cx);
            } else {
                self.upgrade.teardown_uncertain = true;
                self.operations.expanded = true;
            }
        }
    }
}
