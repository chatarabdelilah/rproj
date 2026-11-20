use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};
use rproj_core::updates::{
    Entry,
    review::{Action, Plan, Selection, State, choices},
};
#[cfg(test)]
mod tests;

#[derive(Default)]
pub(in super::super) struct ReviewSession {
    pub(in super::super) support: Option<Result<(), String>>,
    picked: Option<Selection>,
    pub(in super::super) plan: Option<Plan>,
    pub(super) active: Option<(u64, Selection)>,
    dialog: bool,
    close_after: bool,
    confirmed: Option<u64>,
    pub(in super::super) teardown_uncertain: bool,
}

impl Workbench {
    fn observed_selection(&self, s: &Selection) -> bool {
        if self.project.as_ref() != Some(&s.project) || self.section != Section::Updates {
            return false;
        }
        let Some(project) = self.updates.projects.get(&s.project) else {
            return false;
        };
        let (observation, error, stale) = project.current(&s.local);
        project.loaded
            && project.fresh_check
            && project.pending.is_none()
            && project.selected.as_ref() == Some(&key(&s.local))
            && project.entries.contains(&s.local)
            && !stale
            && error.is_none()
            && observation.is_some_and(|o| choices(&s.project, &o.entry).contains(s))
    }
    fn can_prepare_update(&self, s: &Selection) -> bool {
        self.running.is_none()
            && !self.teardown_uncertain()
            && !self.updates.apply.dialog
            && matches!(self.updates.apply.support, Some(Ok(())))
            && self.observed_selection(s)
    }
    fn update_watch_conflict(&self, s: &Selection) -> bool {
        self.watch.as_ref().is_some_and(|w| {
            w.project.as_ref() == Some(&s.project)
                && !matches!(
                    w.state,
                    rproj_core::operations::WatchState::Stopped
                        | rproj_core::operations::WatchState::Failed
                )
        })
    }
    fn can_apply_update(&self, p: &Plan) -> bool {
        self.can_prepare_update(&p.selection)
            && p.blockers.is_empty()
            && self.updates.apply.plan.as_ref() == Some(p)
            && !self.update_watch_conflict(&p.selection)
    }
    pub(super) fn update_review_controls(
        &self,
        entry: &Entry,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut panel = div().v_flex().gap_2();
        let Some(path) = self.project.as_ref() else {
            return panel.into_any_element();
        };
        let options = choices(path, entry);
        if let Some((_, s)) = &self.updates.apply.active
            && &s.project == path
            && s.local == entry.local
        {
            let phase = if matches!(self.running, Some((Operation::ApplyReviewedUpdate, _))) {
                "Applying"
            } else {
                "Preparing review for"
            };
            return panel
                .child(format!(
                    "{phase} {} {} · {}",
                    s.local.alias(),
                    s.version,
                    s.project.display()
                ))
                .into_any_element();
        }
        if options.is_empty() {
            if entry.comparison.status == rproj_core::updates::Status::NewerAvailable {
                panel=panel.child("No supported declaration edit for this release. Review the requirement manually.");
            }
            return panel.into_any_element();
        }
        match &self.updates.apply.support {
            Some(Ok(())) => {}
            Some(Err(reason)) => {
                return panel
                    .child(format!("Apply unavailable. {reason}"))
                    .into_any_element();
            }
            None => {
                return panel
                    .child(if self.client.is_none() {
                        "Apply requires the matching bundled CLI."
                    } else {
                        "Checking update worker support…"
                    })
                    .into_any_element();
            }
        }
        if !self
            .updates
            .projects
            .get(path)
            .is_some_and(|p| p.fresh_check)
        {
            return panel
                .child("Check for updates again before preparing a review.")
                .into_any_element();
        }
        let selected = self
            .updates
            .apply
            .picked
            .as_ref()
            .filter(|s| options.contains(s))
            .unwrap_or(&options[0])
            .clone();
        panel = panel.child("Choose version");
        for option in options {
            let chosen = option == selected;
            let description = match &option.local {
                Local::Tool { .. } => "Exact project pin",
                Local::Package { declaration, .. }
                    if option
                        .declaration()
                        .as_ref()
                        .is_ok_and(|s| s == declaration) =>
                {
                    "Keep declaration"
                }
                Local::Package { .. } => "Change declaration",
            };
            panel = panel.child(
                Button::new(SharedString::from(format!(
                    "update-version-{}",
                    option.version
                )))
                .label(format!("{} · {description}", option.version))
                .selected(chosen)
                .disabled(!self.can_prepare_update(&option))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.can_prepare_update(&option) {
                        this.updates.apply.picked = Some(option.clone());
                        this.updates.apply.plan = None;
                        cx.notify();
                    }
                })),
            );
        }
        if let Some(plan) = self
            .updates
            .apply
            .plan
            .as_ref()
            .filter(|p| p.selection == selected)
        {
            panel = panel
                .child("Review changes")
                .children(plan.changes.iter().cloned())
                .children(plan.blockers.iter().map(|s| format!("Blocked · {s}")));
            if self.update_watch_conflict(&selected) {
                panel = panel.child(
                    "Stop Watch for this project and wait for acknowledgement before applying.",
                );
            }
            let enabled = self.can_apply_update(plan);
            panel = panel.child(
                Button::new("apply-reviewed-update")
                    .label("Apply update…")
                    .disabled(!enabled)
                    .on_click(cx.listener(|this, _, w, cx| this.confirm_reviewed_update(w, cx))),
            );
        } else {
            panel = panel.child(
                Button::new("prepare-reviewed-update")
                    .label("Prepare review")
                    .disabled(!self.can_prepare_update(&selected))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.can_prepare_update(&selected) {
                            this.start_reviewed_update(Action::Prepare(selected.clone()), cx);
                        }
                    })),
            );
        }
        panel.into_any_element()
    }
    fn start_reviewed_update(&mut self, action: Action, cx: &mut Context<Self>) {
        let Some(client) = self.client.clone() else {
            return;
        };
        let s = action.selection().clone();
        let op = match &action {
            Action::Prepare(_) => Operation::PrepareReviewedUpdate,
            Action::Apply(_) => Operation::ApplyReviewedUpdate,
        };
        let id = self.operations.begin(op, s.project.clone());
        self.running = Some((op, Cancellation::default()));
        self.pending_switch = None;
        self.activity.set_origin(Some(format!(
            "{} · {} · {}",
            s.project.display(),
            s.local.alias(),
            s.version
        )));
        self.updates.apply.active = Some((id, s.clone()));
        self.updates.apply.picked = Some(s);
        self.updates.apply.plan = None;
        self.updates.apply.confirmed = None;
        let sender = self.sender.clone();
        let dropped = self.dropped.clone();
        std::thread::spawn(move || {
            let events = sender.clone();
            let result = client.reviewed_update(action, move |event| {
                if events.try_send(Event::Progress(id, event)).is_err() {
                    dropped.fetch_add(1, Ordering::Relaxed);
                }
            });
            let _ = sender.send(Event::ReviewedUpdateFinished(id, Box::new(result)));
        });
        cx.notify();
    }
    fn update_dialog_focus(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.updates.apply.dialog = false;
        if self.section == Section::Updates {
            self.updates_focus.focus(w, cx);
        } else {
            self.focus.focus(w, cx);
        }
        cx.notify();
    }
    fn confirm_reviewed_update(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        let Some(plan) = self
            .updates
            .apply
            .plan
            .clone()
            .filter(|p| self.can_apply_update(p))
        else {
            return;
        };
        let generation = self.watch_generation;
        let entity = cx.entity().downgrade();
        self.updates.apply.dialog = true;
        w.open_alert_dialog(cx, move |dialog, _, _| {
            let (keep, apply, dismiss) = (entity.clone(), entity.clone(), entity.clone());
            let p = plan.clone();
            dialog
                .title("Apply reviewed update?")
                .child(format!(
                    "{} · {} → {}. {}",
                    p.selection.project.display(),
                    p.selection.local.alias(),
                    p.selection.version,
                    p.changes.join("\n")
                ))
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("update-keep-reviewing")
                                .primary()
                                .label("Keep reviewing")
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let _ =
                                        keep.update(cx, |this, cx| this.update_dialog_focus(w, cx));
                                }),
                        )
                        .child(
                            Button::new("update-confirm")
                                .label("Apply update")
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let _ = apply.update(cx, |this, cx| {
                                        this.update_dialog_focus(w, cx);
                                        if this.watch_generation == generation
                                            && this.can_apply_update(&p)
                                        {
                                            this.start_reviewed_update(
                                                Action::Apply(p.clone()),
                                                cx,
                                            );
                                        }
                                    });
                                }),
                        ),
                )
                .on_close(move |_, w, cx| {
                    let _ = dismiss.update(cx, |this, cx| this.update_dialog_focus(w, cx));
                })
        });
    }
    pub(in super::super) fn finish_reviewed_update(
        &mut self,
        id: u64,
        result: Result<State, anyhow::Error>,
        cx: &mut Context<Self>,
    ) {
        if !self.operations.accepts(id)
            || !self
                .updates
                .apply
                .active
                .as_ref()
                .is_some_and(|(pending, _)| *pending == id)
        {
            return;
        }
        let (_, s) = self.updates.apply.active.take().unwrap();
        let applying = matches!(self.running, Some((Operation::ApplyReviewedUpdate, _)));
        let mut confirmed = true;
        let outcome = match result {
            Ok(State::Prepared(p)) if !applying && p.selection == s => {
                self.updates.apply.plan = Some(*p);
                Ok(OperationOutcome::Succeeded)
            }
            Ok(State::Applied) if applying => Ok(OperationOutcome::Succeeded),
            Ok(State::Failed {
                possible_writes,
                reason,
            }) => Err(if possible_writes {
                format!("{reason}. Partial changes may remain.")
            } else {
                reason
            }),
            Err(error) => {
                self.updates.apply.teardown_uncertain |=
                    rproj_core::operations::owned_teardown_unconfirmed(&error);
                confirmed = !self.updates.apply.teardown_uncertain;
                Err(format!("{error:#}"))
            }
            Ok(State::Uncertain(reason)) => {
                confirmed = false;
                Err(reason)
            }
            _ => {
                confirmed = false;
                Err("Unexpected update result. Inspect the project before retrying.".into())
            }
        };
        self.operations.finish(id, &outcome);
        self.running = None;
        self.activity.flush();
        self.activity.append(&format!(
            "{} {} · {}",
            s.local.alias(),
            s.version,
            self.operations.summary()
        ));
        self.activity.set_origin(None);
        if applying || outcome.is_err() {
            if let Some(project) = self.updates.projects.get_mut(&s.project) {
                let message = outcome.as_ref().err().cloned().unwrap_or_else(|| {
                    format!(
                        "Verified {} {}. Check for updates again.",
                        s.local.alias(),
                        s.version
                    )
                });
                project.observations.remove(&key(&s.local));
                if outcome.is_err() {
                    project
                        .failures
                        .insert(key(&s.local), (s.local.clone(), message.clone()));
                } else {
                    project.failures.remove(&key(&s.local));
                }
                project.summary = message;
            }
            if self.project.as_ref() == Some(&s.project) {
                self.inspect();
            }
        }
        self.updates.apply.confirmed = confirmed.then_some(id);
        if std::mem::take(&mut self.updates.apply.close_after)
            && confirmed
            && !self.updates.apply.dialog
        {
            self.continue_close(cx);
        }
        cx.notify();
    }
    pub(in super::super) fn reviewed_update_close(
        &mut self,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        if self.updates.apply.teardown_uncertain || self.updates.apply.dialog {
            return Some(false);
        }
        let (id, selection) = self.updates.apply.active.clone()?;
        self.updates.apply.dialog = true;
        self.updates.apply.close_after = false;
        let entity = cx.entity().downgrade();
        w.open_alert_dialog(cx,move|dialog,_,_|{
            let (keep,close,dismiss)=(entity.clone(),entity.clone(),entity.clone());
            dialog.title("Wait for this update?").child(format!("{} · {} {}. Closing waits for completion. An interrupted installation can leave partial changes.",selection.project.display(),selection.local.alias(),selection.version))
            .footer(DialogFooter::new().child(Button::new("update-keep-open").primary().label("Keep open").on_click(move|_,w,cx|{w.close_dialog(cx);let _=keep.update(cx,|this,cx|this.update_dialog_focus(w,cx));}))
            .child(Button::new("update-close-after").label("Close after update").on_click(move|_,w,cx|{w.close_dialog(cx);let _=close.update(cx,|this,cx|{
                this.update_dialog_focus(w,cx);
                if this.operations.accepts(id){this.updates.apply.close_after=true;}
                else if this.updates.apply.confirmed==Some(id) && !this.teardown_uncertain(){this.continue_close(cx);}
            });}))).on_close(move|_,w,cx|{let _=dismiss.update(cx,|this,cx|this.update_dialog_focus(w,cx));})
        });
        Some(false)
    }
}
