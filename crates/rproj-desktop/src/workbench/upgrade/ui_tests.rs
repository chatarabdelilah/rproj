use super::{Preparation, UpgradeOutcome, UpgradePreparation, UpgradeStatus, WriteState};
use crate::workbench::{
    Cancellation, Event, Operation, OperationEvent, RunStatus, WatchStatus, Workbench,
};
use gpui_kit::{
    AnyWindowHandle, AppContext, Entity, TestAppContext, px, size, test::TestWindowExt,
};
use std::path::PathBuf;

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (project, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let project = this.project.clone().unwrap();
            this.upgrade.begin(project.clone());
            this.upgrade.loading = false;
            this.upgrade.support = Some(Ok(()));
            let Preparation::Prepared(prepared) = Preparation::from_worker(
                UpgradePreparation::Prepared(Box::new(super::tests::review(project))),
            ) else {
                panic!()
            };
            this.upgrade.prepared = Some(*prepared);
            this.upgrade_list.reset(this.upgrade.row_count());
            cx.notify();
        })
    });
    (project, handle, view)
}

#[gpui_kit::test]
fn upgrade_diff_skips_and_layouts_preserve_selection(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    for dimensions in [
        size(px(1120.), px(780.)),
        size(px(800.), px(580.)),
        size(px(1120.), px(780.)),
    ] {
        cx.simulate_window_resize(handle, dimensions);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window
                    .find(if dimensions.width == px(800.) {
                        "upgrade-stacked"
                    } else {
                        "upgrade-adjacent"
                    })
                    .visible()
            );
            window.click("upgrade-file-1", cx);
            window.render_frame(cx);
            assert!(
                window
                    .find("upgrade-skipped")
                    .label()
                    .unwrap()
                    .contains("Comments")
            );
            assert!(window.try_find("upgrade-diff").is_none());
            window.click("upgrade-file-0", cx);
            window.render_frame(cx);
            assert!(window.find("upgrade-diff").bounds().size.width > px(100.));
            assert!(window.find("upgrade-apply").visible());
        })
        .unwrap();
    }
    cx.update(|cx| assert_eq!(view.read(cx).upgrade.selected, 0));
}

#[gpui_kit::test]
fn upgrade_navigation_and_project_closure_keep_captured_review(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let original = cx.update(|cx| view.read(cx).upgrade.project.clone());
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("Dependencies", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            assert_eq!(this.upgrade.project, original);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.show_upgrade(window, cx));
        window.render_frame(cx);
        assert!(window.find("upgrade-flow").visible());
        assert!(
            window
                .find("upgrade-captured-path")
                .label()
                .unwrap()
                .contains(original.as_ref().unwrap().to_str().unwrap())
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn upgrade_apply_refresh_and_replacement_default_to_keep_and_restore_focus(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("upgrade-keep-reviewing").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).running.is_none()));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-refresh", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("upgrade-keep-review").visible());
        window.press("escape", cx);
        window.render_frame(cx);
        assert_eq!(window.find("upgrade-flow").focused(), Some(true));
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.confirm_upgrade_refresh(std::env::temp_dir().join("another"), window, cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-keep-review", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(view.read(cx).upgrade.prepared.is_some()));
}

#[gpui_kit::test]
fn upgrade_watch_conflict_and_stale_confirmation_block_apply(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch = Some(WatchStatus {
                supported: true,
                project: this.upgrade.project.clone(),
                state: rproj_core::operations::WatchState::Watching,
                message: "fixture".into(),
            });
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-apply", cx);
        assert!(window.try_find("upgrade-keep-reviewing").is_none());
        assert!(window.find("upgrade-watch-conflict").visible());
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
        window.click("upgrade-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| view.update(cx, |this, _| this.watch_generation += 1));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-confirm-apply", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.running.is_none());
        assert!(this.upgrade.error.as_ref().unwrap().contains("expired"));
    });
}

#[gpui_kit::test]
fn upgrade_operation_events_keep_ownership_output_and_manual_activity_collapse(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let project = this.upgrade.project.clone().unwrap();
            let id = this
                .operations
                .begin(Operation::ApplyUpgrade, project.clone());
            this.activity
                .set_origin(Some(project.display().to_string()));
            this.running = Some((Operation::ApplyUpgrade, Cancellation::default()));
            this.operations.expanded = false;
            this.operations.cancel();
            this.activity_height = px(270.);
            this.project = None;
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "captured upgrade output\n".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::UpgradeApplied(
                    id,
                    Ok(UpgradeOutcome {
                        project,
                        status: UpgradeStatus::Succeeded,
                        writes: WriteState::Completed,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(270.));
            assert!(
                this.activity
                    .displayed()
                    .any(|l| l.contains("captured upgrade output"))
            );
            assert!(this.upgrade.ready().is_none());
            assert!(this.upgrade.result.is_some());
            this.sender
                .send(Event::UpgradeApplied(
                    id,
                    Ok(UpgradeOutcome {
                        project: PathBuf::from("wrong"),
                        status: UpgradeStatus::Failed,
                        writes: WriteState::Unknown,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(matches!(
                this.upgrade.result,
                Some(Ok(UpgradeOutcome {
                    status: UpgradeStatus::Succeeded,
                    ..
                }))
            ));
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window
                .find("upgrade-result")
                .label()
                .unwrap()
                .contains("Refresh review")
        );
        window.click("upgrade-apply", cx);
        assert!(window.try_find("upgrade-keep-reviewing").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn upgrade_close_waits_for_cancellation_and_keeps_other_dirty_drafts(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let (id, token) = cx.update(|cx| {
        view.update(cx, |this, _| {
            let id = this.operations.begin(
                Operation::ApplyUpgrade,
                this.upgrade.project.clone().unwrap(),
            );
            let token = Cancellation::default();
            this.running = Some((Operation::ApplyUpgrade, token.clone()));
            this.creation.dirty = true;
            (id, token)
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("upgrade-keep-open").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(!token.is_cancelled());
    cx.update_window(handle, |_, window, cx| {
        assert!(view.read(cx).upgrade_focus.is_focused(window));
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
    assert!(!token.is_cancelled());
    cx.update_window(handle, |_, window, cx| {
        assert!(view.read(cx).upgrade_focus.is_focused(window));
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("upgrade-cancel-close", cx);
    })
    .unwrap();
    assert!(token.is_cancelled());
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.upgrade.close_pending);
            assert!(matches!(
                this.operations.latest.as_ref().unwrap().status,
                RunStatus::Cancelling
            ));
            let project = this.upgrade.project.clone().unwrap();
            this.accept_upgrade_result(
                id,
                Ok(UpgradeOutcome {
                    project,
                    status: UpgradeStatus::Cancelled,
                    writes: WriteState::MayHaveBegun,
                    message: Some("Partial changes may remain".into()),
                }),
                cx,
            );
        })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
}

#[gpui_kit::test]
fn upgrade_uncertain_reports_do_not_lock_close_but_teardown_failure_does(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let project = this.upgrade.project.clone().unwrap();
            let id = this
                .operations
                .begin(Operation::ApplyUpgrade, project.clone());
            this.running = Some((Operation::ApplyUpgrade, Cancellation::default()));
            this.accept_upgrade_result(
                id,
                Ok(UpgradeOutcome {
                    project: project.clone(),
                    status: UpgradeStatus::Uncertain,
                    writes: WriteState::Unknown,
                    message: Some("Inspect possible partial changes".into()),
                }),
                cx,
            );
            assert!(!this.upgrade.teardown_uncertain);
            let id = this.operations.begin(Operation::ApplyUpgrade, project);
            this.running = Some((Operation::ApplyUpgrade, Cancellation::default()));
            this.accept_upgrade_result(id, Err("owned teardown unconfirmed".into()), cx);
            assert!(this.teardown_uncertain());
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
}

#[gpui_kit::test]
fn upgrade_large_wrapped_diff_renders_only_viewport_and_keeps_last_line_accessible(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    let last = cx.update(|cx| {
        view.update(cx, |this, cx| {
            let mut review = this.upgrade.prepared.as_ref().unwrap().review.clone();
            review.originals.insert(
                ".luaurc".into(),
                Some((0..1100).map(|i| format!("old {i}\n")).collect()),
            );
            review.files[0].contents = (0..1100)
                .map(|i| format!("new {i} {}\n", "wrapped long line 世界 ".repeat(12)))
                .collect();
            let Preparation::Prepared(prepared) =
                Preparation::from_worker(UpgradePreparation::Prepared(Box::new(review)))
            else {
                panic!()
            };
            this.upgrade.prepared = Some(*prepared);
            let count = this.upgrade.row_count();
            this.upgrade_list.reset(count);
            cx.notify();
            count - 1
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window
                .try_find(gpui_kit::SharedString::from(format!("upgrade-row-{last}")))
                .is_none()
        );
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.upgrade_focus.focus(window, cx));
        window.render_frame(cx);
        for _ in 0..16 {
            window.press("tab", cx);
            window.render_frame(cx);
            if window.find("upgrade-diff").focused() == Some(true) {
                break;
            }
        }
        assert_eq!(window.find("upgrade-diff").focused(), Some(true));
        window.press("end", cx);
        window.render_frame(cx);
        assert!(
            window
                .find(gpui_kit::SharedString::from(format!("upgrade-row-{last}")))
                .visible()
        );
        assert!(
            window
                .find(gpui_kit::SharedString::from(format!("upgrade-row-{last}")))
                .label()
                .unwrap()
                .contains("new 1099")
        );
        view.update(cx, |this, _| {
            this.upgrade_list.scroll_to(gpui_kit::ListOffset {
                item_ix: 1000,
                offset_in_item: px(0.),
            });
        });
        window.render_frame(cx);
    })
    .unwrap();
    cx.simulate_window_resize(handle, size(px(1060.), px(780.)));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            view.read(cx).upgrade_list.logical_scroll_top().item_ix,
            1000
        );
        window.click("upgrade-wrap", cx);
        window.render_frame(cx);
        assert_eq!(
            view.read(cx).upgrade_list.logical_scroll_top().item_ix,
            1000
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn upgrade_long_path_stays_visible_with_activity_in_both_themes(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let path = this
                .upgrade
                .project
                .clone()
                .unwrap()
                .join("long project prefix 世界 ".repeat(12))
                .join("Captured example");
            this.upgrade.project = Some(path.clone());
            this.upgrade.prepared.as_mut().unwrap().review.project = path;
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
            let path = window.find("upgrade-captured-path");
            let y = path.bounds().origin.y;
            assert!(path.visible());
            assert!(path.label().unwrap().contains("世界"));
            window.scroll(
                "upgrade-scroll",
                gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-260.))),
                cx,
            );
            window.render_frame(cx);
            assert_eq!(window.find("upgrade-captured-path").bounds().origin.y, y);
            let apply = window.find("upgrade-apply").bounds();
            let activity = window.find("activity-summary-row").bounds();
            assert!(apply.origin.y + apply.size.height <= activity.origin.y + px(1.));
            assert!(window.find("watch-summary").visible());
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn upgrade_read_only_review_does_not_add_close_prompt_and_loading_errors_stay_visible(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            assert!(this.request_close(window, cx));
            this.upgrade.prepared = None;
            this.upgrade.loading = true;
            cx.notify();
        });
        window.render_frame(cx);
        assert!(window.find("upgrade-loading").visible());
        view.update(cx, |this, cx| {
            this.upgrade.loading = false;
            this.upgrade.error =
                Some("Missing rproj.toml; repair the record, then Refresh review".into());
            cx.notify();
        });
        window.render_frame(cx);
        assert!(
            window
                .find("upgrade-error")
                .label()
                .unwrap()
                .contains("rproj.toml")
        );
        window.click("upgrade-apply", cx);
        assert!(window.try_find("upgrade-keep-reviewing").is_none());
        assert!(window.find("project-switcher").visible());
    })
    .unwrap();
}
