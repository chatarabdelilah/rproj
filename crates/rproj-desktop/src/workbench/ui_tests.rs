use super::{
    Appearance, Cancellation, Event, Operation, OperationEvent, OperationOutcome, Section,
    Workbench,
};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, Point, TestAppContext, WindowBounds,
    WindowOptions, px, size,
};

pub(super) fn fixture(
    cx: &mut TestAppContext,
) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("default.project.json"),
        r#"{"name":"Soft fixture","tree":{"$className":"DataModel"}}"#,
    )
    .unwrap();
    std::fs::write(project.path().join("wally.toml"), "[dependencies]\nSignal = \"sleitnick/signal@2.0.1\"\nUnfamiliar = \"someone/unfamiliar@1.0.0\"\n").unwrap();
    let snapshot = rproj_core::inspect(project.path()).unwrap();
    let preferences = project.path().join("desktop.json");
    let (handle, view) = cx.update(|cx| {
        gpui_kit::init(cx);
        // Hit testing must not sample a different point in the entrance animation.
        cx.set_reduce_motion(true);
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(1120.), px(780.)),
                })),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Workbench::new(None, Some(preferences), window, cx)),
        )
        .unwrap()
    });
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            // Isolate background connection/prefs messages from deterministic UI inputs.
            (this.sender, this.receiver) = std::sync::mpsc::sync_channel(super::EVENT_CAPACITY);
            this.project = Some(snapshot.root.clone());
            this.preferences.recent_projects = vec![snapshot.root.clone()];
            this.snapshot = Some(snapshot);
            cx.notify();
        })
    });
    (project, handle, view)
}

fn enable_creation(view: &Entity<Workbench>, root: &std::path::Path, cx: &mut TestAppContext) {
    use rproj_core::creation::{CreationEnvironment, protocol::*};
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.creation.context = Some(CreationContext {
                protocol: CREATION_PROTOCOL,
                environment: CreationEnvironment {
                    projects_root: root.into(),
                    apps: vec![],
                    extensions: vec![],
                },
                machine_ready: true,
                template: None,
                template_problem: None,
                config_source: SourceSnapshot { bytes: None },
                template_source: SourceSnapshot { bytes: None },
                setups: vec![],
                setups_problem: None,
            });
            this.close_project(cx);
        })
    });
}

#[gpui_kit::test]
fn creation_five_steps_keyboard_retention_and_minimum_layout(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    enable_creation(&view, project.path(), cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("new-project", cx);
        window.input("A Unicode project 世界", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("creation-next", cx);
        window.click("creation-package-react", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, _| {
            assert!(
                this.creation
                    .draft
                    .as_ref()
                    .unwrap()
                    .graph
                    .packages
                    .iter()
                    .any(|p| p == "reactRoblox")
            );
        })
    });
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("creation-next").visible());
        window.click("creation-next", cx);
        window.click("creation-next", cx);
        window.click("creation-file-src", cx);
        window.click("creation-next", cx);
        window.click("creation-create", cx);
        assert!(window.try_find("ok").is_none());
        window.click("Overview", cx);
        assert!(window.try_find("creation-flow").is_none());
        window.click("new-project", cx);
        assert!(
            window
                .find("creation-flow")
                .label()
                .unwrap()
                .ends_with("Review")
        );
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(
            this.creation.draft.as_ref().unwrap().name,
            "A Unicode project 世界"
        );
        assert!(
            !this
                .creation
                .draft
                .as_ref()
                .unwrap()
                .graph
                .dropped
                .iter()
                .any(|k| k == "src")
        );
        assert!(this.watch.is_none());
    });
}

#[gpui_kit::test]
fn creation_draft_default_keep_and_escape_preserve_choices(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    enable_creation(&view, project.path(), cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("new-project", cx);
        window.input("Keep me", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("keep-draft").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).creation.dirty));
    cx.update_window(handle, |_, window, cx| {
        window.click("creation-discard", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).creation.draft.as_ref().unwrap().name,
            "Keep me"
        )
    });
}

#[gpui_kit::test]
fn creation_close_cancellation_waits_and_teardown_error_keeps_window(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    enable_creation(&view, project.path(), cx);
    let destination = project.path().join("Captured destination");
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            let id = this
                .operations
                .begin(Operation::CreateProject, destination.clone());
            this.running = Some((Operation::CreateProject, Cancellation::default()));
            id
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(!view.read(cx).running.as_ref().unwrap().1.is_cancelled()));
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
        window.click("cancel-creation-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.creation.close_pending);
            assert!(this.running.as_ref().unwrap().1.is_cancelled());
            assert_eq!(
                this.operations.latest.as_ref().unwrap().project,
                destination
            );
            this.sender
                .send(Event::Created(id.wrapping_add(1), Err("stale".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_some());
            this.sender
                .send(Event::Created(id, Err("teardown failure fixture".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.creation.teardown_uncertain);
            assert!(!this.creation.close_pending);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
}

#[gpui_kit::test]
fn creation_events_retain_destination_result_and_watch_after_navigation(cx: &mut TestAppContext) {
    use rproj_core::creation::{CreationDraft, protocol::*};
    let (project, handle, view) = fixture(cx);
    enable_creation(&view, project.path(), cx);
    let destination = project.path().join("Created project");
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.creation.draft = Some(CreationDraft::new("Created project".into()));
            this.creation.visible = true;
            this.creation.step = super::creation::Step::Review;
            this.watch = Some(super::WatchStatus {
                supported: true,
                project: Some(project.path().join("Watch owner")),
                state: rproj_core::operations::WatchState::Watching,
                message: String::new(),
            });
            let id = this
                .operations
                .begin(Operation::CreateProject, destination.clone());
            this.running = Some((Operation::CreateProject, Cancellation::default()));
            this.operations.expanded = false;
            this.section = Section::Overview;
            this.sender
                .send(Event::Created(
                    id,
                    Ok(CreationOutcome {
                        destination: destination.clone(),
                        status: CreationStatus::Succeeded,
                        claim: ClaimState::Claimed,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.operations.expanded);
            assert!(this.project.is_none());
            assert!(
                this.activity
                    .displayed()
                    .any(|line| line.contains("Created project"))
            );
            this.sender
                .send(Event::Created(id, Err("late".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.creation.result.as_ref().unwrap().is_ok());
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("creation-open-result", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(this.project.as_ref(), Some(&destination));
        assert_eq!(
            this.watch.as_ref().unwrap().project.as_ref(),
            Some(&project.path().join("Watch owner"))
        );
    });
}

#[gpui_kit::test]
fn creation_close_completes_only_after_owned_cancel_outcome(cx: &mut TestAppContext) {
    use rproj_core::creation::protocol::*;
    let (project, handle, view) = fixture(cx);
    let destination = project.path().join("Partial files remain");
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            let id = this
                .operations
                .begin(Operation::CreateProject, destination.clone());
            this.running = Some((Operation::CreateProject, Cancellation::default()));
            id
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("keep-open", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(!view.read(cx).creation.close_dialog));
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("cancel-creation-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.creation.close_pending);
            assert!(this.running.is_some());
            this.sender
                .send(Event::Created(
                    id,
                    Ok(CreationOutcome {
                        destination,
                        status: CreationStatus::Cancelled,
                        claim: ClaimState::Unknown,
                        message: Some("Inspect for partial files".into()),
                    }),
                ))
                .unwrap();
            this.drain(cx);
        })
    });
    cx.run_until_parked();
    assert!(cx.update_window(handle, |_, _, _| ()).is_err());
}

#[gpui_kit::test]
fn creation_stale_preparation_loading_and_errors_do_not_remove_workbench(cx: &mut TestAppContext) {
    use rproj_core::creation::{CreationDraft, protocol::*};
    let (project, handle, view) = fixture(cx);
    enable_creation(&view, project.path(), cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("new-project", cx);
        window.input("Review", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.creation.step = super::creation::Step::Review;
            let request = this.creation.request().unwrap();
            let prepared = PreparedCreation {
                plan: request
                    .draft
                    .plan(&this.creation.context.as_ref().unwrap().environment)
                    .unwrap(),
                sources: request.sources,
                template: None,
            };
            let id = this
                .operations
                .begin(Operation::PrepareCreation, project.path().join("Review"));
            this.running = Some((Operation::PrepareCreation, Cancellation::default()));
            this.creation.changed();
            this.sender
                .send(Event::Prepared(
                    id,
                    this.creation.revision.wrapping_sub(1),
                    Ok(PreparationOutcome::Prepared(Box::new(prepared))),
                ))
                .unwrap();
            this.drain(cx);
            assert!(this.creation.fresh_plan().is_none());
            assert!(this.running.is_none());
            this.creation.loading = true;
            this.creation.context_id = 10;
            this.sender
                .send(Event::CreationContext(9, Err("stale context".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.creation.loading);
            assert!(this.creation.context.is_some());
            this.creation.loading = false;
            this.creation.draft = Some(CreationDraft::new("Review".into()));
            this.creation.context.as_mut().unwrap().template_problem =
                Some("Invalid template; repair with rproj configure project".into());
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("creation-prepare", cx);
        assert!(
            window
                .find("creation-error")
                .label()
                .unwrap()
                .contains("rproj configure project")
        );
        assert!(window.find("project-switcher").visible());
        assert!(window.find("watch-start").visible());
        window.click("Overview", cx);
        assert!(window.try_find("creation-flow").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn switcher_keyboard_dismissal_and_sidebar_navigation(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("tab", cx);
        assert_eq!(window.find("project-switcher").focused(), Some(true));
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("popup-menu").is_some());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("project-switcher").focused(), Some(true));
        window.press("tab", cx);
        assert_eq!(window.find("Overview").focused(), Some(true));
        window.press("tab", cx);
        window.press("enter", cx);
        window.click("dependencies:Signal", cx);
        window.render_frame(cx);
        assert!(
            window.find("package-detail").bounds().left()
                >= window.find("package-list").bounds().right()
        );
        assert!(
            window.find("dependencies:Signal").bounds().left()
                > window.find("Dependencies").bounds().right()
        );
    })
    .unwrap();
    cx.update(|cx| {
        assert!(view.read(cx).section == Section::Dependencies);
        assert_eq!(
            view.read(cx).selected_package.as_deref(),
            Some("dependencies:Signal")
        );
    });
}

#[gpui_kit::test]
fn switcher_folder_cancel_and_close_preserve_focus_and_watch(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    let selected = cx.update(|cx| view.read(cx).project.clone());
    let watched = project.path().join("watched");
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.watch = Some(super::WatchStatus {
                supported: true,
                project: Some(watched.clone()),
                state: rproj_core::operations::WatchState::Watching,
                message: String::new(),
            });
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("tab", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("up", cx);
        window.press("up", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(|options| {
        assert!(options.directories && !options.files && !options.multiple);
        None
    });
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(view.read(cx).project, selected));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("project-switcher").focused(), Some(true));
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("up", cx);
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.project.is_none());
        assert_eq!(
            this.watch.as_ref().unwrap().project.as_ref(),
            Some(&watched)
        );
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("project-switcher").focused(), Some(true));
        assert!(window.find("Overview").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn minimum_activity_height_contains_long_failure_and_watch_text(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    let origin = project
        .path()
        .join("long-path-component/".repeat(12))
        .join("Original project");
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.operations.begin(Operation::StartWatch, origin.clone());
            let failure = format!(
                "Watch did not acknowledge the requested project: {}",
                "failure details ".repeat(30)
            );
            this.operations.finish(id, &Err(failure.clone()));
            this.activity_height = px(144.);
            this.activity
                .set_origin(Some(format!("{} · Starting Watch", origin.display())));
            this.activity.append(&failure);
            this.watch = Some(super::WatchStatus {
                supported: true,
                project: Some(origin.clone()),
                state: rproj_core::operations::WatchState::Failed,
                message: failure,
            });
            cx.notify();
        })
    });
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let summary = window.find("activity-summary-row").bounds();
        let toolbar = window.find("activity-toolbar").bounds();
        let output = window.find("activity-output").bounds();
        assert!(summary.bottom() <= toolbar.top());
        assert!(toolbar.bottom() <= output.top());
        assert!(
            output.size.height >= px(24.),
            "at least one complete output row must fit: {output:?}"
        );
        assert!(output.bottom() <= window.find("watch-summary").bounds().top());
        assert!(
            window
                .find("operation-summary")
                .label()
                .unwrap()
                .contains("failure details")
        );
        assert!(
            window
                .find("watch-summary")
                .label()
                .unwrap()
                .contains("long-path-component")
        );
        window.click("activity-toggle", cx);
        assert!(window.try_find("activity-toolbar").is_none());
        window.click("activity-toggle", cx);
        assert!(window.find("activity-output").bounds().size.height >= px(24.));
    })
    .unwrap();
    cx.update(|cx| assert_eq!(view.read(cx).activity_height, px(144.)));
}

#[gpui_kit::test]
fn activity_resize_height_survives_collapse_reopen(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.operations.expanded = true;
            this.activity.append("Retained output");
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.render_frame(cx);
        let summary = window.find("activity-summary-row").bounds();
        let initial = window.find("activity-output").bounds().bottom() - summary.top();
        let divider = Point {
            x: summary.center().x,
            y: summary.top() - px(2.),
        };
        window.drag(
            divider,
            Point {
                y: divider.y - px(90.),
                ..divider
            },
            cx,
        );
        window.render_frame(cx);
        window.render_frame(cx);
        let resized = window.find("activity-output").bounds().bottom()
            - window.find("activity-summary-row").bounds().top();
        assert!(
            resized >= initial + px(50.),
            "the divider must actually move: {initial:?} -> {resized:?}"
        );
        window.click("activity-toggle", cx);
        assert!(window.try_find("activity-toolbar").is_none());
        window.click("activity-toggle", cx);
        window.render_frame(cx);
        window.render_frame(cx);
        let reopened = window.find("activity-output").bounds().bottom()
            - window.find("activity-summary-row").bounds().top();
        assert!(
            (reopened - resized).abs() <= px(1.),
            "Activity height changed after collapse: {resized:?} -> {reopened:?}"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn activity_tracks_origin_collapse_filter_cancel_and_late_events(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    let origin = project.path().join("origin");
    let id = cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.operations.begin(Operation::Check, origin.clone());
            this.running = Some((Operation::Check, Cancellation::default()));
            cx.notify();
            id
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("output-empty").label(),
            Some("Waiting for output…")
        );
        window.click("activity-toggle", cx);
        window.click("Tools", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.project = Some(project.path().join("browsed-project"));
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "first\nneedle\n".into(),
                    },
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.operations.expanded);
            assert_eq!(this.operations.latest.as_ref().unwrap().project, origin);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window
                .find("operation-summary")
                .label()
                .unwrap()
                .contains("origin")
        );
        window.click("activity-toggle", cx);
        window.click("output-filter", cx);
        window.input("absent", cx);
        assert_eq!(
            window.find("output-empty").label(),
            Some("No search matches.")
        );
        window.click("cancel-operation", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.running.as_ref().unwrap().1.is_cancelled());
            assert!(this.operations.summary().contains("Cancellation requested"));
            this.sender
                .send(Event::Complete(id, Ok(OperationOutcome::Cancelled)))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_none());
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "late output".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::Complete(id, Ok(OperationOutcome::Succeeded)))
                .unwrap();
            this.drain(cx);
            assert!(this.operations.summary().contains("Cancelled"));
            assert!(
                !this
                    .activity
                    .displayed()
                    .any(|line| line.contains("late output"))
            );
        })
    });
}

#[gpui_kit::test]
fn soft_themes_survive_mode_changes_and_narrow_dependencies_keep_selection(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("Dependencies", cx);
        window.click("dependencies:Signal", cx);
    })
    .unwrap();
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("dependencies:Signal").visible());
        assert!(
            window.find("package-detail").bounds().top()
                >= window.find("package-list").bounds().bottom()
        );
        window.click("dependencies:Unfamiliar", cx);
        for appearance in [Appearance::Light, Appearance::Dark, Appearance::System] {
            super::apply_appearance(appearance, window, cx);
            assert!(Theme::global(cx).theme_name().starts_with("rproj Soft"));
        }
        Theme::change(ThemeMode::Dark, Some(window), cx);
        assert_eq!(
            Theme::global(cx).colors.background,
            gpui_kit::rgb(0x1e1e1d).into()
        );
        Theme::change(ThemeMode::Light, Some(window), cx);
        assert_eq!(
            Theme::global(cx).colors.background,
            gpui_kit::rgb(0xfcfbf8).into()
        );
        window.click("activity-toggle", cx);
        window.render_frame(cx);
        assert!(window.find("output-filter").visible());
        assert!(window.find("package-detail").visible());
        assert!(window.find("project-switcher").visible());
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).selected_package.as_deref(),
            Some("dependencies:Unfamiliar")
        )
    });
}

#[gpui_kit::test]
fn watch_confirmation_restores_focus_and_rejects_changed_selection(cx: &mut TestAppContext) {
    let (project, handle, view) = fixture(cx);
    let source = project.path().join("worker.rs");
    let worker = project.path().join("worker.exe");
    std::fs::write(
        &source,
        format!(
            "fn main() {{ println!(\"rproj {}\"); }}",
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
    let watched = project.path().join("watched-project");
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.client = Some(client);
            this.snapshot.as_mut().unwrap().actions.watch.available = true;
            this.watch = Some(super::WatchStatus {
                supported: true,
                project: Some(watched.clone()),
                state: rproj_core::operations::WatchState::Watching,
                message: String::new(),
            });
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("watch-start", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(window.find("cancel").label(), Some("Keep current Watch"));
        assert_eq!(window.find("ok").label(), Some("Stop and switch"));
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(300));
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find("ok").is_none());
        assert_eq!(window.find("watch-start").focused(), Some(true));
        window.click("watch-start", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            assert_eq!(
                this.watch.as_ref().unwrap().project.as_ref(),
                Some(&watched)
            );
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("ok", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let workbench = view.read(cx);
        assert!(workbench.running.is_none());
        assert!(workbench.operations.expanded);
        assert!(
            workbench
                .activity
                .displayed()
                .any(|line| line.contains("Watch switch expired"))
        );
        assert_eq!(
            workbench.watch.as_ref().unwrap().project.as_ref(),
            Some(&watched)
        );
    });
}
