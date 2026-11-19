use super::{Completion, Failure, Mode, Status, TestContext, watch_blocks};
use crate::workbench::{
    Cancellation, Event, Operation, OperationEvent, OperationOutcome, Workbench,
};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, ClipboardItem, Entity, ScrollDelta, TestAppContext, point, px,
    size,
};
use rproj_core::{
    creation::{
        PackageWorkflow,
        graph::{JestBackend, TestRunner},
    },
    operations::{WatchState, WatchStatus},
    test_execution::protocol::WriteState,
};

#[cfg(windows)]
#[gpui_kit::test]
fn default_tests_ignore_invalid_custom_draft_and_unsupported_custom_worker(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    let source = root.path().join("default_worker.rs");
    let worker = root.path().join("default_worker.exe");
    std::fs::write(
        &source,
        format!(
            r#"
fn main() {{
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--version"] {{ println!("rproj {}"); return; }}
    if args.last().is_some_and(|a| a == "test-support") {{ std::process::exit(1); }}
    assert_eq!(args.len(), 2);
    assert_eq!(args[1], "test");
    std::fs::write("default-arguments.txt", args.join("\n")).unwrap();
    println!("default worker complete");
}}
"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    assert!(
        std::process::Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&worker)
            .status()
            .unwrap()
            .success()
    );
    let client = rproj_core::operations::WorkbenchClient::new(worker).unwrap();
    assert!(client.test_arguments_support().is_err());
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, _| {
            this.client = Some(client);
            this.last_poll = std::time::Instant::now();
            this.test_args.visible = false;
            this.test_args.support = Some(Err("Matching custom worker unavailable".into()));
            this.test_args.stale = true;
            this.test_args.rows[0].mode = Mode::Json;
            this.test_args.rows[0].edit("invalid JSON".into());
            this.snapshot.as_mut().unwrap().actions.test.available = true;
            this.section = crate::workbench::Section::Checks;
        });
        w.render_frame(cx);
        w.click("edit-test-arguments", cx);
        assert!(!view.read(cx).test_args.visible);
        w.click("run-tests", cx);
        assert!(matches!(view.read(cx).running, Some((Operation::Test, _))));
    })
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let done = cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.drain(cx);
                this.running.is_none()
            })
        });
        if done {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "default worker did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(root.path().join("default-arguments.txt")).unwrap(),
        "--rproj-workbench\ntest"
    );
    cx.update(|cx| {
        let state = view.read(cx);
        assert!(state.test_args.dirty());
        assert_eq!(state.test_args.rows[0].text, "invalid JSON");
        assert!(state.test_args.baseline.is_empty());
        assert!(
            state
                .activity
                .displayed()
                .any(|line| line.contains("Succeeded"))
        );
    });
}

#[gpui_kit::test]
fn stale_support_and_competing_runs_cannot_replace_current_state(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.test_support_generation = 8;
            this.sender
                .send(Event::TestSupport(
                    7,
                    Err(Failure {
                        message: "obsolete".into(),
                        teardown: false,
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(matches!(this.test_args.support, Some(Ok(()))));
            this.sender
                .send(Event::Connected(Err("worker disconnected".into())))
                .unwrap();
            this.sender.send(Event::TestSupport(8, Ok(()))).unwrap();
            this.drain(cx);
            assert!(matches!(this.test_args.support, Some(Err(_))));
            this.test_args.support = Some(Ok(()));
        })
    });
    let id = running(&view, Operation::Check, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.run_test_arguments(cx);
            this.start_default_test(this.project.clone().unwrap(), cx);
            assert_eq!(this.operations.latest.as_ref().unwrap().id, id);
            assert!(matches!(this.running, Some((Operation::Check, _))));
        })
    });
}

#[gpui_kit::test]
fn refresh_keep_preserves_arguments_and_new_context_does_not_retarget_navigation(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.request_test_intent(super::controller::Intent::Refresh, w, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.simulate_next_frame(cx);
        w.render_frame(cx);
        assert_eq!(w.find("test-keep-draft").focused(), Some(true));
        w.press("escape", cx);
        assert_eq!(view.read(cx).test_args.values().unwrap(), ["original"]);
        assert!(view.read(cx).running.is_none());
    })
    .unwrap();
    let id = running(&view, Operation::LoadTestContext, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let mut context = this.test_args.context.clone().unwrap();
            context.runner = TestRunner::JestRoblox;
            let captured = context.project.clone();
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some(captured.clone()),
                state: WatchState::Watching,
                message: String::new(),
            });
            this.enter_project(root.path().join("other"));
            this.test_args.rows[0].edit("edited during context load".into());
            this.accept_test_context(id, 0, Ok(context), cx);
            assert!(!this.test_args.visible);
            assert_eq!(this.test_args.project, Some(captured.clone()));
            assert_eq!(
                this.test_args.values().unwrap(),
                ["edited during context load"]
            );
            assert_eq!(
                this.test_args.context.as_ref().unwrap().runner,
                TestRunner::JestRoblox
            );
            assert_eq!(this.watch.as_ref().unwrap().project, Some(captured));
            assert!(this.test_args.ready().is_ok());
            this.close_project(cx);
            assert!(this.test_args.project.is_some());
            assert!(this.watch.as_ref().unwrap().project.is_some());
        })
    });
}

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (root, handle, view) = super::super::ui_tests::fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            let project = this.project.clone().unwrap();
            this.test_args.project = Some(project.clone());
            this.test_args.context = Some(TestContext {
                project,
                name: "Captured fixture".into(),
                runner: TestRunner::TestEz,
                backend: JestBackend::Studio,
                workflow: PackageWorkflow::None,
                inputs: Default::default(),
            });
            this.test_args.support = Some(Ok(()));
            this.show_test_args(w, cx);
            this.add_test_row(w, cx);
            this.test_args.rows[0].edit("original".into());
            this.test_args_inputs.rows[&1]
                .update(cx, |input, cx| input.set_value("original", w, cx));
        })
    })
    .unwrap();
    (root, handle, view)
}

#[gpui_kit::test]
fn obsolete_support_probe_teardown_failure_still_blocks_close_and_mutations(
    cx: &mut TestAppContext,
) {
    let (_root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.test_support_generation = 2;
            this.accept_test_support(
                1,
                Err(Failure {
                    message: "fixture child exit not confirmed".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(matches!(this.test_args.support, Some(Ok(()))));
            assert!(this.teardown_uncertain());
            assert!(!this.request_close(w, cx));
            this.start_default_test(this.project.clone().unwrap(), cx);
            this.run_test_arguments(cx);
            assert!(this.running.is_none());
            assert!(this.operations.expanded);
            assert!(
                this.activity
                    .displayed()
                    .any(|line| line.contains("probe teardown is unconfirmed"))
            );
        })
    })
    .unwrap();
}

fn running(view: &Entity<Workbench>, operation: Operation, cx: &mut TestAppContext) -> u64 {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this
                .operations
                .begin(operation, this.test_args.project.clone().unwrap());
            this.running = Some((operation, Cancellation::default()));
            this.test_args.completed = None;
            if operation == Operation::CustomTest {
                this.test_args.submitted = Some((id, this.test_args.values().unwrap()));
            }
            this.activity
                .set_origin(this.operations.latest.as_ref().map(|r| r.origin()));
            cx.notify();
            id
        })
    })
}
fn completion(
    view: &Entity<Workbench>,
    status: Status,
    accepted: Option<bool>,
    cx: &mut TestAppContext,
) -> Completion {
    cx.update(|cx| {
        let state = &view.read(cx).test_args;
        Completion {
            project: state.project.clone().unwrap(),
            accepted,
            status,
            effects: if accepted == Some(true) {
                WriteState::MayHaveBegun
            } else {
                WriteState::NotStarted
            },
            runner_exit: (status == Status::Succeeded).then_some(0),
            message: None,
        }
    })
}

#[gpui_kit::test]
fn argument_controls_render_wide_narrow_and_both_themes(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for (width, height) in [(1120., 780.), (800., 580.)] {
            cx.simulate_window_resize(handle, size(px(width), px(height)));
            cx.update_window(handle, |_, w, cx| {
                Theme::change(mode, Some(w), cx);
                w.render_frame(cx);
                assert!(w.find("test-arguments-screen").visible());
                assert!(
                    w.find(if width == 800. {
                        "test-arguments-stacked"
                    } else {
                        "test-arguments-adjacent"
                    })
                    .visible(),
                    "width={width} scroll={:?} input={:?} screen={:?}",
                    w.find("test-arguments-scroll").bounds(),
                    w.find("test-input-1").bounds(),
                    w.find("test-arguments-screen").bounds()
                );
                assert!(
                    w.find("test-captured")
                        .label()
                        .unwrap()
                        .contains("Captured fixture")
                );
                assert!(w.find("test-input-1").visible());
                assert!(w.find("test-run-arguments").visible());
                assert!(view.read(cx).test_args.dirty());
            })
            .unwrap();
        }
    }
}

#[gpui_kit::test]
fn native_paste_hook_keeps_crlf_tabs_and_refuses_raw_json_paste(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("test-input-1", cx);
        w.press("ctrl-a", cx);
        cx.write_to_clipboard(ClipboardItem::new_string("保存\r\nline\n\ttab".into()));
        w.dispatch_action(Box::new(gpui_kit::component::input::Paste), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        assert_eq!(
            view.read(cx).test_args.values().unwrap(),
            ["保存\r\nline\n\ttab"]
        );
        assert_eq!(view.read(cx).test_args.rows[0].mode, Mode::Json);
        let before = view.read(cx).test_args.rows[0].text.clone();
        cx.write_to_clipboard(ClipboardItem::new_string("raw\r\n".into()));
        w.dispatch_action(Box::new(gpui_kit::component::input::Paste), cx);
        assert_eq!(view.read(cx).test_args.rows[0].text, before);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(
            view.read(cx)
                .test_args
                .error
                .as_ref()
                .unwrap()
                .contains("JSON escapes")
        );
    });
}

#[gpui_kit::test]
fn modes_empty_consent_order_and_invalid_text_remain_explicit(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, _| {
            this.test_args.baseline = vec!["original".into()]
        });
        w.render_frame(cx);
        w.click("test-json-1", cx);
        assert!(!view.read(cx).test_args.dirty());
        view.update(cx, |this, cx| {
            this.test_args.rows[0].edit("\"broken".into());
            assert_eq!(this.test_args.rows[0].last_valid, "original");
            assert!(this.test_args.ready().is_err());
            this.test_args.rows[0].edit("\"original\"".into());
            this.add_test_row(w, cx);
            assert!(this.test_args.values().is_err());
        });
        w.render_frame(cx);
        let viewport = w.find("test-arguments-scroll").bounds();
        let target = w.find("test-empty-2").bounds();
        w.scroll(
            "test-arguments-scroll",
            ScrollDelta::Pixels(point(px(0.), viewport.center().y - target.center().y)),
            cx,
        );
        w.render_frame(cx);
        w.click("test-empty-2", cx);
        assert_eq!(view.read(cx).test_args.values().unwrap(), ["original", ""]);
        view.update(cx, |this, cx| this.move_test_row(2, true, cx));
        assert_eq!(view.read(cx).test_args.values().unwrap(), ["", "original"]);
        view.update(cx, |this, cx| this.remove_test_row(2, w, cx));
        assert_eq!(view.read(cx).test_args_inputs.rows.len(), 1);
        assert_eq!(view.read(cx).test_args_inputs.subscriptions.len(), 1);
    })
    .unwrap();
}

#[gpui_kit::test]
fn project_navigation_retains_draft_and_keep_dialog_restores_focus(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let original = cx.update(|cx| view.read(cx).test_args.project.clone());
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.creation.dirty = true;
            this.enter_project(root.path().join("other"));
            assert!(!this.test_args.visible);
            this.show_test_args(w, cx);
            this.edit_test_args(w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("test-keep-draft").visible());
        w.press("escape", cx);
        w.render_frame(cx);
        assert_eq!(w.find("test-arguments-screen").focused(), Some(true));
        view.update(cx, |this, cx| {
            assert_eq!(this.test_args.project, original);
            assert!(this.creation.dirty);
            this.close_project(cx);
            assert_eq!(this.test_args.project, original);
            this.show_test_args(w, cx);
            assert_eq!(this.test_args.values().unwrap(), ["original"]);
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn accepted_submission_retains_later_edits_ownership_and_activity(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let id = running(&view, Operation::CustomTest, cx);
    let result = completion(&view, Status::Succeeded, Some(true), cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.test_args.rows[0].edit("later edit".into());
            this.enter_project(root.path().join("other"));
            this.operations.expanded = false;
            this.activity_height = px(290.);
            this.accept_custom_tests(id + 1, Ok(result.clone()), cx);
            assert!(this.running.is_some());
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "owned output\n".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::CustomTestComplete(id, Ok(result.clone())))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_none());
            assert!(this.test_args.dirty());
            assert_eq!(this.test_args.baseline, ["original"]);
            assert!(this.test_args.stale);
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(290.));
            assert!(
                this.activity
                    .displayed()
                    .any(|line| line.contains("owned output")
                        && line.contains(result.project.to_str().unwrap()))
            );
            this.filter
                .update(cx, |input, cx| input.set_value("owned output", w, cx));
            this.accept_custom_tests(
                id,
                Err(Failure {
                    message: "obsolete".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(!this.test_args.teardown_uncertain);
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn refusal_uncertainty_and_wrong_context_preserve_unsubmitted_values(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    for (status, accepted) in [(Status::Failed, Some(false)), (Status::Uncertain, None)] {
        let id = running(&view, Operation::CustomTest, cx);
        let result = completion(&view, status, accepted, cx);
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.accept_custom_tests(id, Ok(result), cx);
                assert!(this.test_args.baseline.is_empty());
                assert!(this.test_args.dirty());
                assert!(this.test_args.ready().is_err());
            })
        });
    }
    let id = running(&view, Operation::LoadTestContext, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let mut context = this.test_args.context.clone().unwrap();
            context.project.push("wrong");
            this.accept_test_context(id, 0, Ok(context), cx);
            assert!(
                this.test_args
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("another project")
            );
            assert!(this.test_args.stale);
            assert_eq!(this.test_args.values().unwrap(), ["original"]);
        })
    });
}

#[gpui_kit::test]
fn both_test_paths_wait_for_cancel_then_protect_other_drafts(cx: &mut TestAppContext) {
    for operation in [Operation::Test, Operation::CustomTest] {
        let (_root, handle, view) = fixture(cx);
        cx.update(|cx| {
            view.update(cx, |this, _| {
                this.test_args.baseline = vec!["original".into()];
                this.test_args.visible = operation == Operation::CustomTest;
                this.creation.dirty = true;
                this.watch = Some(WatchStatus {
                    supported: true,
                    project: this.project.clone(),
                    state: WatchState::Watching,
                    message: String::new(),
                });
            })
        });
        let id = running(&view, operation, cx);
        let result = completion(&view, Status::Cancelled, Some(true), cx);
        cx.update_window(handle, |_, w, cx| {
            view.update(cx, |this, cx| assert!(!this.request_close(w, cx)));
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            w.simulate_next_frame(cx);
            w.render_frame(cx);
            assert!(w.find("test-keep-open").visible());
            assert_eq!(w.find("test-keep-open").focused(), Some(true));
            w.press("escape", cx);
            view.update(cx, |this, cx| {
                assert!(!this.running.as_ref().unwrap().1.is_cancelled());
                assert!(!this.request_close(w, cx));
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            w.click("test-cancel-close", cx);
        })
        .unwrap();
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                assert!(this.test_args.close_pending);
                assert!(this.running.as_ref().unwrap().1.is_cancelled());
                if operation == Operation::Test {
                    this.accept_default_tests(id, Ok(OperationOutcome::Cancelled), cx);
                } else {
                    this.accept_custom_tests(id, Ok(result), cx);
                }
                assert_eq!(this.watch.as_ref().unwrap().state, WatchState::Watching);
                assert_eq!(this.watch.as_ref().unwrap().project, this.project);
            })
        });
        cx.run_until_parked();
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.find("keep-draft").visible());
            w.press("escape", cx);
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn completion_during_dialog_does_not_close_and_unconfirmed_teardown_blocks(
    cx: &mut TestAppContext,
) {
    let (_root, handle, view) = fixture(cx);
    let id = running(&view, Operation::Test, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(w, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_default_tests(id, Ok(OperationOutcome::Succeeded), cx)
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("test-keep-open").visible());
        w.click("test-keep-open", cx);
    })
    .unwrap();
    let id = running(&view, Operation::Test, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.test_args.close_pending = true;
            this.accept_default_tests(
                id,
                Err(Failure {
                    message: "unconfirmed teardown".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(this.teardown_uncertain());
            assert!(!this.request_close(w, cx));
            assert_eq!(this.test_args.values().unwrap(), ["original"]);
        });
    })
    .unwrap();
}

#[test]
fn tests_allow_steady_watch_and_block_transitions_without_changing_owner() {
    for state in [
        WatchState::Watching,
        WatchState::Stopped,
        WatchState::Failed,
    ] {
        assert!(!watch_blocks(Some(&WatchStatus {
            supported: true,
            project: Some("same or other".into()),
            state,
            message: String::new()
        })));
    }
    for state in [
        WatchState::Preparing,
        WatchState::Stopping,
        WatchState::Unresponsive,
    ] {
        assert!(watch_blocks(Some(&WatchStatus {
            supported: true,
            project: Some("same or other".into()),
            state,
            message: String::new()
        })));
    }
}
