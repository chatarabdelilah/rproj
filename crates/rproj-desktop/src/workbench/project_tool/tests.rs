use super::{Plan, Step, choices, scope, tool_catalog};
use crate::workbench::Workbench;
use crate::workbench::{Cancellation, Event, Operation, OperationEvent, WatchStatus};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::{AnyWindowHandle, Entity, px, size};
use gpui_kit::{AppContext, TestAppContext, test::TestWindowExt};
use rproj_core::{
    operations::WatchState,
    project_tool_setup::protocol::{Completion, Status, WriteState},
};
use std::path::PathBuf;

fn plan(project: PathBuf) -> Plan {
    Plan {
        project,
        tool: "asphalt".into(),
        inputs: Default::default(),
        prerequisites: vec!["Rokit".into()],
        notes: vec![],
        steps: vec![Step::CreateFile {
            relative: "asphalt.toml".into(),
            contents: "creator = 0\n[inputs]\npath = \"assets\"\n".into(),
        }],
    }
}

#[test]
fn catalog_choices_keep_executables_and_integrations_distinct() {
    assert_eq!(choices("").len(), tool_catalog::all_setup_entries().count());
    assert!(choices("luau-lsp").iter().any(|t| t.key == "luau-lsp-cli"));
    assert!(choices("luau-lsp").iter().any(|t| t.key == "luau-lsp"));
    assert_ne!(
        scope(tool_catalog::find("luau-lsp-cli").unwrap()),
        scope(tool_catalog::find("luau-lsp").unwrap())
    );
    assert!(choices("no such tool").is_empty());
}

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (root, handle, view) = super::super::ui_tests::fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.project_tool.project = this.project.clone();
            this.project_tool.tool = "asphalt".into();
            this.project_tool.support = Some(Ok(()));
            this.show_project_tool(w, cx);
        })
    })
    .unwrap();
    (root, handle, view)
}

#[gpui_kit::test]
fn tool_selection_and_review_render_both_layouts_and_themes(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for width in [1120., 800.] {
            cx.simulate_window_resize(handle, size(px(width), px(580.)));
            cx.update_window(handle, |_, w, cx| {
                Theme::change(mode, Some(w), cx);
                view.update(cx, |this, cx| {
                    this.project_tool_query
                        .update(cx, |input, cx| input.set_value("blender", w, cx))
                });
                w.render_frame(cx);
                assert!(
                    w.find(if width == 800. {
                        "project-tool-stacked"
                    } else {
                        "project-tool-adjacent"
                    })
                    .visible()
                );
                assert!(w.find("project-tool-captured").visible());
                w.click("project-tool-choice-blender", cx);
            })
            .unwrap();
            cx.update(|cx| {
                view.update(cx, |this, cx| {
                    assert_eq!(this.project_tool.tool, "blender");
                    this.project_tool.choose("asphalt");
                    cx.notify();
                })
            });
        }
    }
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.project_tool.plan = Some(plan(this.project_tool.project.clone().unwrap()));
            cx.notify();
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("project-tool-review").visible());
        let viewport = w.find("project-tool-scroll").bounds();
        let file = w.find("project-tool-file-0").bounds();
        w.scroll(
            "project-tool-scroll",
            gpui_kit::ScrollDelta::Pixels(gpui_kit::point(
                px(0.),
                viewport.center().y - file.center().y,
            )),
            cx,
        );
        w.render_frame(cx);
        assert!(w.find("project-tool-file-0").visible());
        assert_eq!(
            w.find("project-tool-file-0").label(),
            Some("creator = 0\n[inputs]\npath = \"assets\"\n")
        );
    })
    .unwrap();
}

fn ready(view: &Entity<Workbench>, cx: &mut TestAppContext) {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.project_tool.plan = Some(plan(this.project_tool.project.clone().unwrap()));
            cx.notify();
        })
    });
}
fn running(view: &Entity<Workbench>, cx: &mut TestAppContext) -> u64 {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.operations.begin(
                Operation::SetupProjectTool,
                this.project_tool.project.clone().unwrap(),
            );
            this.running = Some((Operation::SetupProjectTool, Cancellation::default()));
            this.project_tool.loading = true;
            this.project_tool.completion_confirmed = false;
            cx.notify();
            id
        })
    })
}
fn result(view: &Entity<Workbench>, status: Status, cx: &mut TestAppContext) -> Completion {
    cx.update(|cx| {
        let state = &view.read(cx).project_tool;
        Completion {
            project: state.project.clone().unwrap(),
            tool: state.tool.clone(),
            status,
            effects: WriteState::Unknown,
            outcome: None,
            message: Some("fixture result".into()),
        }
    })
}

#[gpui_kit::test]
fn captured_review_survives_browsing_and_replacement_defaults_keep(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    ready(&view, cx);
    let captured = cx.update(|cx| view.read(cx).project_tool.project.clone());
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.creation.dirty = true;
            this.watch = Some(WatchStatus {
                supported: true,
                project: captured.clone(),
                state: WatchState::Watching,
                message: String::new(),
            });
            this.enter_project(root.path().join("other"));
            assert!(!this.project_tool.visible);
            this.edit_project_tool(w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("project-tool-keep-review").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            assert_eq!(this.project_tool.project, captured);
            assert!(this.project_tool.ready().is_some());
            this.close_project(cx);
            this.show_project_tool(w, cx);
            assert!(this.creation.dirty);
            assert_eq!(this.watch.as_ref().unwrap().project, captured);
        });
        w.render_frame(cx);
        assert!(
            w.find("project-tool-captured")
                .label()
                .unwrap()
                .contains(captured.as_ref().unwrap().to_str().unwrap())
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn confirmation_escape_and_changed_watch_preserve_review(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    ready(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| this.confirm_project_tool(w, cx))
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("project-tool-keep-reviewing").visible());
        w.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("project-tool-screen").focused(), Some(true));
        view.update(cx, |this, cx| this.confirm_project_tool(w, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| view.update(cx, |this, _| this.watch_generation += 1));
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("project-tool-confirm", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.running.is_none());
        assert!(
            this.project_tool
                .error
                .as_ref()
                .unwrap()
                .contains("Watch changed")
        );
        assert!(this.project_tool.ready().is_none());
    });
}

#[gpui_kit::test]
fn setup_completion_retains_owner_output_height_and_rejects_obsolete_events(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    ready(&view, cx);
    let id = running(&view, cx);
    let completion = result(&view, Status::Succeeded, cx);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("cancel-operation").is_none());
        w.click("Overview", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.enter_project(root.path().join("browsing"));
            this.operations.expanded = false;
            this.activity_height = px(290.);
            this.accept_project_tool_applied(id + 1, Ok(completion.clone()), cx);
            assert!(this.running.is_some());
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "tool fixture output\n".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::ProjectToolApplied(id, Ok(completion.clone())))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_none());
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(290.));
            assert!(
                this.activity
                    .displayed()
                    .any(|line| line.contains("tool fixture output")
                        && line.contains(completion.project.to_str().unwrap()))
            );
            this.accept_project_tool_applied(
                id,
                Err(super::Failure {
                    message: "duplicate".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(!this.project_tool.teardown_uncertain);
            assert_eq!(
                this.project_tool.result.as_ref().unwrap().status,
                Status::Succeeded
            );
            assert!(this.project_tool.ready().is_none());
        })
    });
}

#[gpui_kit::test]
fn close_waits_without_cancelling_and_protects_other_drafts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    ready(&view, cx);
    let id = running(&view, cx);
    let completion = result(&view, Status::Succeeded, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.creation.dirty = true;
            assert!(!this.request_close(w, cx));
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("project-tool-keep-open").visible());
        w.click("project-tool-close-after", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.project_tool.close_pending);
            assert!(!this.running.as_ref().unwrap().1.is_cancelled());
            this.accept_project_tool_applied(id, Ok(completion), cx);
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

#[gpui_kit::test]
fn uncertain_completion_and_teardown_keep_closing_blocked(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let id = running(&view, cx);
    let completion = result(&view, Status::Uncertain, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.project_tool.close_pending = true;
            this.accept_project_tool_applied(id, Ok(completion), cx);
            assert!(!this.project_tool.close_pending);
            assert!(!this.project_tool.completion_confirmed);
            assert!(this.project_tool.ready().is_none());
        })
    });
    let id = running(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.accept_project_tool_applied(
                id,
                Err(super::Failure {
                    message: "owned teardown uncertain".into(),
                    teardown: true,
                }),
                cx,
            );
            assert!(this.teardown_uncertain());
            assert!(!this.request_close(w, cx));
        })
    })
    .unwrap();
}

#[test]
fn changing_tool_invalidates_review_and_watch_guards_only_captured_project() {
    let project = PathBuf::from("captured");
    let mut state = super::Session {
        project: Some(project.clone()),
        tool: "asphalt".into(),
        plan: Some(plan(project.clone())),
        ..Default::default()
    };
    let mut watch = WatchStatus {
        supported: true,
        project: Some(project),
        state: WatchState::Watching,
        message: String::new(),
    };
    assert!(state.ready().is_some());
    assert!(state.watch_conflict(Some(&watch)));
    watch.state = WatchState::Stopping;
    assert!(state.watch_conflict(Some(&watch)));
    watch.state = WatchState::Stopped;
    assert!(!state.watch_conflict(Some(&watch)));
    watch.state = WatchState::Watching;
    watch.project = Some("other".into());
    assert!(!state.watch_conflict(Some(&watch)));
    state.choose("tungsten");
    assert!(state.ready().is_none());
    assert!(state.plan.is_none());
}

#[gpui_kit::test]
fn completion_while_close_dialog_is_open_honors_keep_and_other_drafts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for keep in [true, false] {
        let id = running(&view, cx);
        let completion = result(&view, Status::Failed, cx);
        cx.update_window(handle, |_, w, cx| {
            view.update(cx, |this, cx| {
                this.creation.dirty = true;
                assert!(!this.request_close(w, cx));
            })
        })
        .unwrap();
        cx.run_until_parked();
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.accept_project_tool_applied(id, Ok(completion), cx)
            })
        });
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            w.click(
                if keep {
                    "project-tool-keep-open"
                } else {
                    "project-tool-close-after"
                },
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            if keep {
                assert_eq!(w.find("project-tool-screen").focused(), Some(true));
                assert!(w.try_find("keep-draft").is_none());
            } else {
                assert!(w.find("keep-draft").visible());
                w.press("escape", cx);
            }
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn preparation_errors_stale_events_and_unavailable_worker_stay_actionable(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let project = this.project_tool.project.clone().unwrap();
            let id = this
                .operations
                .begin(Operation::PrepareProjectTool, project.clone());
            this.running = Some((Operation::PrepareProjectTool, Cancellation::default()));
            this.project_tool.loading = true;
            this.accept_project_tool_prepared(id + 1, 0, Ok(plan(project)), cx);
            assert!(this.running.is_some());
            assert!(this.project_tool.plan.is_none());
            this.accept_project_tool_prepared(
                id,
                this.project_tool.revision,
                Err(super::Failure {
                    message:
                        "Rokit is required; install it with Machine Setup, then Refresh review."
                            .into(),
                    teardown: false,
                }),
                cx,
            );
            assert!(this.running.is_none());
            assert!(this.project_tool.ready().is_none());
            this.project_tool.support =
                Some(Err("Unsupported worker; use a matching distribution".into()));
            cx.notify();
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(
            w.find("project-tool-error")
                .label()
                .unwrap()
                .contains("Rokit")
        );
        w.click("project-tool-prepare", cx);
        w.click("project-tool-apply", cx);
        assert!(view.read(cx).running.is_none());
        assert!(!view.read(cx).project_tool.dialog_open);
        view.update(cx, |this, cx| {
            this.prepare_project_tool(cx);
            assert!(this.running.is_none());
            let id = this
                .operations
                .begin(Operation::Check, this.project.clone().unwrap());
            this.running = Some((Operation::Check, Cancellation::default()));
            this.project_tool.support = Some(Ok(()));
            this.prepare_project_tool(cx);
            assert!(this.operations.accepts(id));
            assert!(matches!(this.running, Some((Operation::Check, _))));
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn reopened_close_decision_and_tool_replacement_default_keep(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    ready(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("project-tool-back", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("project-tool-keep-choice").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(view.read(cx).project_tool.plan.is_some()));
    let id = running(&view, cx);
    let completion = result(&view, Status::Succeeded, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(w, cx)))
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("project-tool-close-after", cx);
    })
    .unwrap();
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            assert!(this.project_tool.close_pending);
            assert!(!this.request_close(w, cx));
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(!this.project_tool.close_pending);
            this.accept_project_tool_applied(id, Ok(completion), cx);
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("project-tool-screen").focused(), Some(true));
        assert!(view.read(cx).project_tool.result.is_some());
    })
    .unwrap();
}
