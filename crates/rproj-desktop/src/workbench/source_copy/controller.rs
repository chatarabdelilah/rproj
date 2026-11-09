use super::*;
use gpui_kit::component::{button::*, dialog::DialogFooter};

impl Workbench {
    pub(in super::super) fn show_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.browser.leave();
        self.library.visible = false;
        self.template.visible = false;
        self.saved.visible = false;
        self.setup.visible = false;
        self.settings.visible = false;
        self.upgrade.visible = false;
        self.composition.visible = false;
        self.creation.visible = false;
        self.source.visible = true;
        self.source.focus.focus(window, cx);
        cx.notify();
    }

    pub(in super::super) fn edit_source(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.source.owner.as_ref() == Some(&project) {
            self.show_source(window, cx);
        } else if self.source.owner.is_some() {
            self.confirm_source_replace(project, window, cx);
        } else {
            self.prepare_source(project, cx);
            self.show_source(window, cx);
        }
    }

    fn confirm_source_replace(
        &mut self,
        project: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.source.dialog_open {
            return;
        }
        self.source.dialog_open = true;
        let revision = self.source.revision;
        let captured = self.source.owner.clone();
        let focus = window.focused(cx).unwrap_or_else(|| self.focus.clone());
        let owner = cx.entity().downgrade();
        let message = format!(
            "Keep the captured Copy Source review for {} or replace it with a snapshot of {}? Nothing is copied automatically.",
            captured.as_ref().unwrap().display(),
            project.display()
        );
        window.open_alert_dialog(cx, move |dialog, _, _| {
            let keep = owner.clone();
            let replace = owner.clone();
            let closed = owner.clone();
            let keep_focus = focus.clone();
            let close_focus = focus.clone();
            let captured = captured.clone();
            let project = project.clone();
            dialog
                .title("Replace Copy Source review?")
                .child(message.clone())
                .footer(
                    DialogFooter::new()
                        .child(
                            Button::new("source-keep-review")
                                .primary()
                                .label("Keep review")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = keep.update(cx, |this, cx| {
                                        this.source.dialog_open = false;
                                        keep_focus.focus(window, cx);
                                    });
                                }),
                        )
                        .child(
                            Button::new("source-replace-review")
                                .label("Replace review")
                                .on_click(move |_, window, cx| {
                                    window.close_dialog(cx);
                                    let _ = replace.update(cx, |this, cx| {
                                        this.source.dialog_open = false;
                                        if this.source.revision == revision
                                            && this.source.owner == captured
                                            && this.project.as_ref() == Some(&project)
                                        {
                                            this.prepare_source(project.clone(), cx);
                                            this.show_source(window, cx);
                                        }
                                    });
                                }),
                        ),
                )
                .on_close(move |_, window, cx| {
                    let _ = closed.update(cx, |this, cx| {
                        this.source.dialog_open = false;
                        close_focus.focus(window, cx);
                    });
                })
        });
    }

    pub(in super::super) fn prepare_source(&mut self, project: PathBuf, cx: &mut Context<Self>) {
        self.source.request(project);
        self.source_notice(
            "Collecting src snapshot; contents omitted from Activity.",
            true,
        );
        self.start_source_request();
        cx.notify();
    }

    fn start_source_request(&mut self) {
        let Some(request) = self.source.take_request() else {
            return;
        };
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let abandoned = || request.abandoned.load(Ordering::Relaxed);
            let result =
                rproj_core::source_copy::collect_with_abandonment(&request.project, abandoned)
                    .and_then(|raw| Prepared::new(raw, abandoned))
                    .map_err(|e| format!("{e:#}"));
            // Only one collector runs at a time. Even abandoned completions release
            // that lane before the latest queued request starts.
            let _ = sender.send(Event::SourcePrepared(request.id, Box::new(result)));
        });
    }

    pub(in super::super) fn accept_source(
        &mut self,
        id: u64,
        result: Result<Prepared, String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(failed) = self.source.accept(id, result) {
            self.source_notice(&self.source.status.clone(), failed);
        }
        self.start_source_request();
        cx.notify();
    }

    pub(super) fn source_notice(&mut self, text: &str, expand: bool) {
        let owner = self
            .source
            .owner
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let origin = format!("{owner} · Copy Source");
        // Do not flush or retag a concurrent operation's partially buffered line.
        let previous = self.activity.origin.replace(Arc::from(origin));
        self.activity.append(text);
        self.activity.origin = previous;
        if self.running.is_none() {
            self.source.notice = Some((
                self.operations.latest.as_ref().map(|r| r.id),
                format!("{} · Copy Source · {text}", self.source.name()),
            ));
        }
        if expand {
            self.operations.expanded = true;
        }
    }

    pub(in super::super) fn source_activity_summary(&self, compact: bool) -> String {
        if self.running.is_none()
            && let Some((id, text)) = &self.source.notice
            && *id == self.operations.latest.as_ref().map(|r| r.id)
        {
            return text.clone();
        }
        if compact {
            self.operations.compact_summary()
        } else {
            self.operations.summary()
        }
    }

    pub(in super::super) fn copy_source_snapshot(&mut self, revision: u64, cx: &mut Context<Self>) {
        if self.source.revision != revision || !self.source.copyable() {
            return;
        }
        let snapshot = self.source.snapshot.as_ref().unwrap();
        cx.write_to_clipboard(ClipboardItem::new_string(snapshot.raw.text.clone()));
        self.source.status = format!(
            "Copy requested · {}{}",
            snapshot.summary(),
            if self.source.failed_refresh {
                " · Previous snapshot (Refresh failed)"
            } else {
                ""
            }
        );
        self.source_notice(&self.source.status.clone(), true);
        cx.notify();
    }
}
