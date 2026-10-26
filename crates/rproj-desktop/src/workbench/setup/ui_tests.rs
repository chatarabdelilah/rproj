use super::{Category, Failure, Step};
use crate::workbench::{Cancellation, Event, Operation, OperationEvent, WatchStatus, Workbench};
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Entity, TestAppContext, Window, px, size, test::TestWindowExt,
};
use rproj_core::{
    machine_setup::{Outcome, Persistence, SetupResult, Status, protocol::*},
    operations::{SetupStop, WatchState},
};

fn reveal(window: &mut Window, cx: &mut App, id: &'static str) {
    let viewport = window.find("setup-scroll").bounds();
    let control = window.find(id).bounds();
    window.scroll(
        "setup-scroll",
        gpui_kit::ScrollDelta::Pixels(gpui_kit::point(
            px(0.),
            viewport.center().y - control.center().y,
        )),
        cx,
    );
    window.render_frame(cx);
}

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (root, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup = super::tests::session(root.path().into());
            cx.notify();
        })
    });
    (root, handle, view)
}

#[gpui_kit::test]
fn setup_selections_and_responsive_layouts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for dimensions in [size(px(1120.), px(780.)), size(px(800.), px(580.))] {
        cx.simulate_window_resize(handle, dimensions);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find("setup-flow").visible());
            let layout = if dimensions.width == px(800.) {
                "setup-stacked"
            } else {
                "setup-adjacent"
            };
            assert!(window.try_find(layout).is_some());
            reveal(window, cx, "setup-choice-git");
            window.click("setup-choice-git", cx);
            window.render_frame(cx);
            view.update(cx, |this, _| {
                assert_eq!(
                    this.setup.dirty(),
                    dimensions.width == px(1120.),
                    "choice click at {dimensions:?}; control={:?}; viewport={:?}",
                    window.find("setup-choice-git").bounds(),
                    window.find("setup-scroll").bounds()
                )
            });
        })
        .unwrap();
    }
    cx.update(|cx| assert!(!view.read(cx).setup.dirty()));
}

#[gpui_kit::test]
fn setup_categories_search_and_keyboard_preserve_selections(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for category in Category::ALL {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let id = format!("setup-category-{}", category.label());
            window.click(gpui_kit::SharedString::from(id), cx);
            window.render_frame(cx);
            assert_eq!(view.read(cx).setup.category, category);
        })
        .unwrap();
    }
    cx.update_window(handle, |_, window, cx| {
        window.click("setup-category-System Apps", cx);
        window.render_frame(cx);
        reveal(window, cx, "setup-choice-git");
        window.click("setup-choice-git", cx);
        for _ in 0..60 {
            if window.find("setup-choice-git").focused() == Some(true) {
                break;
            }
            window.press("tab", cx);
        }
        assert_eq!(window.find("setup-choice-git").focused(), Some(true));
        window.press("space", cx);
        assert!(!view.read(cx).setup.dirty());
        window.press("enter", cx);
        assert!(view.read(cx).setup.dirty());
        reveal(window, cx, "setup-search");
        window.click("setup-search", cx);
        window.input("no-such-software", cx);
        window.render_frame(cx);
        assert!(window.try_find("setup-choice-git").is_none());
        assert!(
            view.read(cx)
                .setup
                .selection
                .as_ref()
                .unwrap()
                .apps
                .contains("git")
        );
        window.press("ctrl-a", cx);
        window.press("backspace", cx);
        window.render_frame(cx);
        assert!(window.try_find("setup-choice-git").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn setup_reliable_warning_result_reveals_new_failure_only_once(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    let id = running(&view, cx);
    let mut report = result(&view, cx, SetupStatus::Succeeded);
    report.statuses[1] = Status::Failed;
    report.result.as_mut().unwrap().outcome = Outcome::Warnings;
    report.result.as_mut().unwrap().statuses = report.statuses.clone();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.operations.expanded = false;
            this.accept_setup_applied(id, Ok(report.clone()), cx);
            assert!(this.operations.expanded);
            this.operations.expanded = false;
            this.accept_setup_applied(id, Ok(report), cx);
            assert!(!this.operations.expanded);
            assert!(!this.setup.dirty());
        })
    });
}

#[gpui_kit::test]
fn setup_long_root_and_controls_fit_with_activity_in_both_themes(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let root = this
                .setup
                .selection
                .as_ref()
                .unwrap()
                .root
                .join("long projects folder 世界 ".repeat(12));
            this.setup.selection.as_mut().unwrap().root = root.clone();
            this.setup.context.as_mut().unwrap().saved.root = root;
            super::tests::prepare(&mut this.setup);
            this.operations.expanded = true;
            this.activity_height = px(144.);
            cx.notify();
        })
    });
    for theme in ["Light", "Dark"] {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(theme, cx);
            window.render_frame(cx);
            let root = window.find("setup-root");
            assert!(root.visible());
            assert!(root.label().unwrap().contains("世界"));
            let apply = window.find("setup-apply").bounds();
            let activity = window.find("activity-summary-row").bounds();
            assert!(apply.origin.y + apply.size.height <= activity.origin.y + px(1.));
            assert!(window.find("watch-summary").visible());
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn setup_reload_retains_retry_choices_and_rejects_stale_context_events(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup.toggle(Category::Apps, "git");
            let proposed = this.setup.selection.clone();
            this.setup.retry_selection = proposed.clone();
            let mut fresh = this.setup.context.clone().unwrap();
            fresh.saved.root = fresh.saved.root.join("changed root");
            let id = this
                .operations
                .begin(Operation::LoadSetup, fresh.config_path.clone());
            this.running = Some((Operation::LoadSetup, Cancellation::default()));
            this.setup.loading = true;
            let revision = this.setup.revision;
            this.accept_setup_loaded(id, revision + 1, Ok(fresh.clone()), cx);
            assert!(this.setup.loading && this.setup.retry_selection.is_some());
            this.accept_setup_loaded(id, revision, Ok(fresh.clone()), cx);
            assert_eq!(
                this.setup.selection.as_ref().unwrap().apps,
                proposed.unwrap().apps
            );
            assert_eq!(
                this.setup.selection.as_ref().unwrap().root,
                fresh.saved.root
            );
            assert!(this.setup.dirty());
            assert!(!this.setup.reload_required);
            // A second explicit retry must not lose selections if reading fails.
            this.setup.retry_selection = this.setup.selection.clone();
            this.setup.context = None;
            let id = this
                .operations
                .begin(Operation::LoadSetup, fresh.config_path);
            this.running = Some((Operation::LoadSetup, Cancellation::default()));
            this.setup.loading = true;
            this.accept_setup_loaded(
                id,
                revision,
                Err(Failure {
                    message: "Malformed machine configuration; repair it and reload".into(),
                    teardown: false,
                }),
                cx,
            );
            assert!(this.setup.selection.as_ref().unwrap().apps.contains("git"));
            assert!(this.setup.error.as_ref().unwrap().contains("Malformed"));
            assert!(this.setup.dirty());
            assert!(this.running.is_none());
        })
    });
}

#[gpui_kit::test]
fn setup_changed_unknown_choices_require_explicit_reload(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup.retry_selection = this.setup.selection.clone();
            let mut fresh = this.setup.context.clone().unwrap();
            fresh.saved.tools.insert("future-tool".into());
            let id = this
                .operations
                .begin(Operation::LoadSetup, fresh.config_path.clone());
            this.running = Some((Operation::LoadSetup, Cancellation::default()));
            this.setup.loading = true;
            this.accept_setup_loaded(id, this.setup.revision, Ok(fresh), cx);
            assert!(this.setup.reload_required);
            assert!(this.setup.ready().is_none());
            assert!(
                this.setup
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("Saved choices changed")
            );
        })
    });
}

#[gpui_kit::test]
fn setup_completion_during_stop_dialog_restores_focus_without_late_stop(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let id = running(&view, cx);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.confirm_setup_stop(false, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    let success = result(&view, cx, SetupStatus::Succeeded);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_setup_applied(id, Ok(success), cx)
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-confirm-stop", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("setup-flow").focused(), Some(true));
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(
            this.setup.result.as_ref().unwrap().status,
            Some(SetupStatus::Succeeded)
        );
        assert!(!this.setup.dialog_open && !this.setup_stop_requested());
    });
}

#[gpui_kit::test]
fn setup_machine_draft_survives_project_close_without_retargeting_watch(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let original = cx.update(|cx| view.read(cx).setup.context.clone());
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        reveal(window, cx, "setup-choice-git");
        window.click("setup-choice-git", cx);
        window.click("Overview", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            assert!(this.setup.dirty());
            assert_eq!(this.setup.context, original);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("machine-setup", cx);
        window.render_frame(cx);
        assert!(window.find("setup-flow").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn setup_apply_and_discard_default_to_keep_and_restore_focus(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup.toggle(Category::Apps, "git");
            super::tests::prepare(&mut this.setup);
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("setup-keep-reviewing").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).setup.dirty()));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("setup-flow").focused(), Some(true));
        window.click("setup-discard", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("setup-keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).setup.dirty()));
}

fn running(view: &Entity<Workbench>, cx: &mut TestAppContext) -> u64 {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup.toggle(Category::Apps, "git");
            super::tests::prepare(&mut this.setup);
            let id = this.operations.begin(
                Operation::ApplySetup,
                this.setup.context.as_ref().unwrap().config_path.clone(),
            );
            this.running = Some((Operation::ApplySetup, Cancellation::default()));
            this.setup.loading = true;
            this.setup.step = Step::Results;
            this.setup.stop = Some(SetupStop::default());
            cx.notify();
            id
        })
    })
}
fn result(view: &Entity<Workbench>, cx: &mut TestAppContext, status: SetupStatus) -> SetupReport {
    cx.update(|cx| {
        let review = view.read(cx).setup.prepared.clone().unwrap();
        let request = SetupRequest {
            protocol: SETUP_PROTOCOL,
            id: "ui-fixture".into(),
            action: SetupAction::Apply(review.clone()),
        };
        let mut report = SetupReport::initial(&request);
        report.status = Some(status);
        report.effects = WriteState::MayHaveBegun;
        report.statuses = vec![Status::Installed; review.items.len()];
        report.result = Some(SetupResult {
            outcome: if status == SetupStatus::Stopped {
                Outcome::Cancelled
            } else {
                Outcome::Completed
            },
            statuses: report.statuses.clone(),
            changes_may_have_begun: true,
            persistence: if status == SetupStatus::Succeeded {
                Persistence::Saved {
                    contents: "headless fixture baseline".into(),
                }
            } else {
                Persistence::NotSaved
            },
        });
        if status == SetupStatus::Succeeded {
            report.effects = WriteState::Completed;
        }
        report
    })
}

#[gpui_kit::test]
fn setup_stop_and_close_wait_without_killing_and_honor_other_drafts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let id = running(&view, cx);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(!view.read(cx).setup_stop_requested()));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(300));
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-confirm-stop", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, _| {
            assert!(this.setup_stop_requested() && this.setup.close_pending);
            assert!(!this.running.as_ref().unwrap().1.is_cancelled());
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new(
                "Keep other draft".into(),
            ));
            this.creation.dirty = true;
        })
    });
    let stopped = result(&view, cx, SetupStatus::Stopped);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_setup_applied(id, Ok(stopped), cx)
        })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).creation.dirty));
}

#[gpui_kit::test]
fn setup_stale_events_activity_ownership_and_failure_expansion_survive_navigation(
    cx: &mut TestAppContext,
) {
    let (_root, _handle, view) = fixture(cx);
    let id = running(&view, cx);
    let success = result(&view, cx, SetupStatus::Succeeded);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some("other watch".into()),
                state: WatchState::Watching,
                message: String::new(),
            });
            this.close_project(cx);
            this.operations.expanded = false;
            this.activity_height = px(280.);
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "setup output\n".into(),
                    },
                ))
                .unwrap();
            this.drain(cx);
            assert!(
                this.activity
                    .displayed()
                    .any(|s| s.contains("Machine Setup") && s.contains("setup output"))
            );
            assert!(!this.operations.expanded);
            this.accept_setup_applied(
                id + 1,
                Err(Failure {
                    message: "stale".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(this.setup_applying() && !this.teardown_uncertain());
            this.accept_setup_applied(id, Ok(success), cx);
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(280.));
            assert!(!this.setup.dirty());
            assert_eq!(
                this.watch.as_ref().unwrap().project,
                Some("other watch".into())
            );
            assert!(this.operations.summary().starts_with("Machine Setup"));
        })
    });
}

#[gpui_kit::test]
fn setup_teardown_uncertainty_blocks_close_and_mutations_but_prespawn_error_recovers(
    cx: &mut TestAppContext,
) {
    let (_root, handle, view) = fixture(cx);
    let id = running(&view, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_setup_applied(
                id,
                Err(Failure {
                    message: "Could not start worker".into(),
                    teardown: false,
                }),
                cx,
            );
            assert!(!this.teardown_uncertain());
            assert!(this.setup.dirty());
        })
    });
    let id = running(&view, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.setup.close_pending = true;
            this.accept_setup_applied(
                id,
                Err(Failure {
                    message: "Owned teardown unconfirmed".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(this.teardown_uncertain());
            assert!(!this.setup.close_pending);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
}

#[gpui_kit::test]
fn setup_watch_and_expired_confirmations_never_apply(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            super::tests::prepare(&mut this.setup);
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some("other".into()),
                state: WatchState::Watching,
                message: String::new(),
            });
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-apply", cx);
        assert!(window.try_find("setup-keep-reviewing").is_none());
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = None;
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| view.update(cx, |this, _| this.watch_generation += 1));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("setup-confirm-apply", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.running.is_none());
        assert!(this.setup.error.as_ref().unwrap().contains("expired"));
    });
}
