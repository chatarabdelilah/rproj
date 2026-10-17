use super::{
    COMPOSITION_PROTOCOL, CompositionContext, CompositionOutcome, CompositionPlan,
    CompositionPreparation, CompositionStatus, Step, WriteState,
};
use crate::workbench::{Cancellation, Event, Operation, Section, Workbench};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, px, size};
use rproj_core::creation::ProjectGraph;

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (project, handle, view) = crate::workbench::ui_tests::fixture(cx);
    let source =
        "mode='expert'\npackages=['vide']\ndropped=['.gitignore']\n[capabilities]\nlint='selene'\n";
    std::fs::write(project.path().join("rproj.toml"), source).unwrap();
    let snapshot = rproj_core::inspect(project.path()).unwrap();
    let mut graph = ProjectGraph {
        mode: "expert".into(),
        packages: vec!["vide".into()],
        dropped: vec![".gitignore".into()],
        ..Default::default()
    };
    graph.choose("lint", None);
    let context = CompositionContext {
        protocol: COMPOSITION_PROTOCOL,
        project: snapshot.root.clone(),
        name: "Captured composition".into(),
        source: source.into(),
        graph: Some(graph),
        problem: None,
        inputs: vec![],
    };
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.composition.begin_load(context.project.clone());
            this.composition.accept_context(id, Ok((context, snapshot)));
            cx.notify();
        })
    });
    (project, handle, view)
}

fn prepared(view: &Entity<Workbench>, cx: &mut TestAppContext) -> CompositionPlan {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let request = this.composition.request().unwrap();
            let plan = CompositionPlan {
                project: request.context.project.clone(),
                draft: request.draft,
                inputs: vec![],
                files: vec![],
                tools_to_add: vec![],
                external_operations: vec!["Simulated reviewed operation; never executed".into()],
                create_blender: false,
            };
            this.composition.prepared = Some((this.composition.revision, plan.clone()));
            cx.notify();
            plan
        })
    })
}

#[gpui_kit::test]
fn composition_steps_lock_originals_and_keep_draft_across_navigation(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-package-vide", cx);
        window.click("composition-query", cx);
        window.input("promise", cx);
        window.render_frame(cx);
        window.click("composition-package-promise", cx);
        window.click("composition-next", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let state = &view.read(cx).composition;
        assert_eq!(state.step, Step::Capabilities);
        assert!(
            state
                .draft
                .as_ref()
                .unwrap()
                .graph
                .packages
                .iter()
                .any(|k| k == "vide")
        );
        assert!(
            state
                .draft
                .as_ref()
                .unwrap()
                .graph
                .packages
                .iter()
                .any(|k| k == "promise")
        );
        assert_eq!(state.draft.as_ref().unwrap().graph.dropped, [".gitignore"]);
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-cap-lint-none", cx);
        window.click("composition-cap-format-stylua", cx);
        window.click("composition-next", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert_eq!(this.composition.step, Step::Review);
            assert_eq!(
                this.composition.draft.as_ref().unwrap().graph.capabilities["lint"],
                "selene"
            );
            assert!(
                this.composition
                    .draft
                    .as_ref()
                    .unwrap()
                    .graph
                    .capabilities
                    .contains_key("format")
            );
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new(
                "Separate new project".into(),
            ));
            this.creation.dirty = true;
            this.open_project(std::path::PathBuf::from("Different browsed project"), cx);
            assert!(!this.composition.visible);
            assert!(this.composition.dirty);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.show_composition(window, cx));
        window.render_frame(cx);
        assert!(window.find("composition-project-path").visible());
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_ne!(this.project, this.composition.project);
        assert!(this.creation.dirty);
    });
}

#[gpui_kit::test]
fn composition_details_and_both_layouts_remain_present(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-query", cx);
        window.input("vide", cx);
    })
    .unwrap();
    for dimensions in [size(px(1120.), px(780.)), size(px(800.), px(580.))] {
        cx.simulate_window_resize(handle, dimensions);
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click("composition-details-vide", cx);
            window.render_frame(cx);
            assert!(window.find("composition-package-list").visible());
            let list = window.find("composition-package-list").bounds();
            let details = window.find("composition-package-details").bounds();
            if dimensions.width == px(800.) {
                assert!(
                    details.origin.y > list.origin.y,
                    "narrow details must stack"
                );
                window.scroll(
                    "composition-scroll",
                    gpui_kit::ScrollDelta::Pixels(gpui_kit::point(px(0.), px(-260.))),
                    cx,
                );
                window.render_frame(cx);
            } else {
                assert_eq!(
                    details.origin.y, list.origin.y,
                    "wide panes must be adjacent"
                );
            }
            assert!(window.find("composition-package-details").visible());
            assert!(window.find("composition-next").visible());
        })
        .unwrap();
    }
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).composition.selected_package.as_deref(),
            Some("vide")
        )
    });
}

#[gpui_kit::test]
fn composition_dirty_reload_and_close_default_to_keep(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.composition
                .draft
                .as_mut()
                .unwrap()
                .toggle_package("promise")
                .unwrap();
            this.composition.changed();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.reload_composition(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("composition-keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).composition.dirty));
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-keep-draft", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let state = &view.read(cx).composition;
        assert!(state.dirty);
        assert!(!state.dialog_open);
        assert!(
            state
                .draft
                .as_ref()
                .unwrap()
                .graph
                .packages
                .iter()
                .any(|k| k == "promise")
        );
    });
}

#[gpui_kit::test]
fn composition_watch_conflicts_and_stale_confirmation_refuse_apply(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.composition
                .draft
                .as_mut()
                .unwrap()
                .toggle_package("promise")
                .unwrap();
            this.composition.changed();
            this.composition.step = Step::Review;
        })
    });
    prepared(&view, cx);
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.watch = Some(crate::workbench::WatchStatus {
                supported: true,
                project: this.composition.project.clone(),
                state: rproj_core::operations::WatchState::Watching,
                message: "Original Watch".into(),
            });
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("composition-watch-conflict").visible());
        window.click("composition-apply", cx);
    })
    .unwrap();
    cx.update(|cx| assert!(view.read(cx).running.is_none()));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.watch.as_mut().unwrap().project = Some(std::path::PathBuf::from("Other Watch"));
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| this.confirm_composition_apply(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| view.update(cx, |this, _| this.watch_generation += 1));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-confirm-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.running.is_none());
        assert!(this.composition.prepared.is_none());
        assert!(this.composition.error.as_ref().unwrap().contains("changed"));
        assert_eq!(
            this.watch.as_ref().unwrap().project,
            Some(std::path::PathBuf::from("Other Watch"))
        );
    });
}

#[gpui_kit::test]
fn composition_events_keep_owner_and_reject_stale_results(cx: &mut TestAppContext) {
    let (_project, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let owner = this.composition.project.clone().unwrap();
            let id = this
                .operations
                .begin(Operation::ApplyComposition, owner.clone());
            this.running = Some((Operation::ApplyComposition, Cancellation::default()));
            this.activity.set_origin(Some(owner.display().to_string()));
            this.operations.expanded = false;
            this.open_project(std::path::PathBuf::from("Browsing elsewhere"), cx);
            this.sender
                .send(Event::CompositionApplied(
                    id.wrapping_add(1),
                    Err("late event".into()),
                ))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_some());
            this.sender
                .send(Event::CompositionApplied(
                    id,
                    Ok(CompositionOutcome {
                        project: owner.clone(),
                        status: CompositionStatus::Succeeded,
                        writes: WriteState::Completed,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.operations.expanded);
            assert!(
                this.activity
                    .displayed()
                    .any(|row| row.contains(&owner.display().to_string()))
            );
            assert_eq!(
                this.project,
                Some(std::path::PathBuf::from("Browsing elsewhere"))
            );
            assert_eq!(
                this.composition
                    .result
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .project,
                owner
            );
            this.sender
                .send(Event::CompositionApplied(id, Err("late failure".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.composition.result.as_ref().unwrap().is_ok());
        })
    });
}

#[gpui_kit::test]
fn composition_cancel_close_waits_and_teardown_failure_keeps_window(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            let id = this.operations.begin(
                Operation::ApplyComposition,
                this.composition.project.clone().unwrap(),
            );
            this.running = Some((Operation::ApplyComposition, Cancellation::default()));
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
        window.click("composition-cancel-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.composition.close_pending);
            assert!(this.running.as_ref().unwrap().1.is_cancelled());
            this.sender
                .send(Event::CompositionApplied(
                    id,
                    Err("teardown failure fixture".into()),
                ))
                .unwrap();
            this.drain(cx);
            assert!(this.composition.teardown_uncertain);
            assert!(!this.composition.close_pending);
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| assert!(!this.request_close(window, cx)));
    })
    .unwrap();
}

#[gpui_kit::test]
fn composition_preparation_failure_is_not_a_teardown_failure(cx: &mut TestAppContext) {
    let (_project, _handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.operations.begin(
                Operation::PrepareComposition,
                this.composition.project.clone().unwrap(),
            );
            this.running = Some((Operation::PrepareComposition, Cancellation::default()));
            this.sender
                .send(Event::CompositionPrepared(
                    id,
                    this.composition.revision,
                    Ok(CompositionPreparation::Failed(
                        "Source files changed; explicitly reload".into(),
                    )),
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.composition.teardown_uncertain);
            assert!(this.composition.prepared.is_none());
            assert!(this.composition.error.as_ref().unwrap().contains("reload"));
            assert!(this.operations.expanded);
            this.section = Section::Dependencies;
        })
    });
}

#[gpui_kit::test]
fn composition_keyboard_activation_keeps_focus_in_the_flow(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("composition-query", cx);
        window.input("promise", cx);
        window.press("tab", cx);
        assert_eq!(
            window.find("composition-package-promise").focused(),
            Some(true)
        );
        window.press("space", cx);
        for _ in 0..10 {
            if window.find("composition-next").focused() == Some(true) {
                break;
            }
            window.press("tab", cx);
        }
        assert_eq!(window.find("composition-next").focused(), Some(true));
        window.press("enter", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let state = &view.read(cx).composition;
        assert_eq!(state.step, Step::Capabilities);
        assert!(
            state
                .draft
                .as_ref()
                .unwrap()
                .graph
                .packages
                .iter()
                .any(|k| k == "promise")
        );
    });
}

#[gpui_kit::test]
fn composition_uncertain_result_keeps_navigation_and_other_watch(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let owner = this.composition.project.clone().unwrap();
            this.composition.step = Step::Review;
            this.watch = Some(crate::workbench::WatchStatus {
                supported: true,
                project: Some(std::path::PathBuf::from("Another watched project")),
                state: rproj_core::operations::WatchState::Watching,
                message: "Unchanged".into(),
            });
            let id = this
                .operations
                .begin(Operation::ApplyComposition, owner.clone());
            this.running = Some((Operation::ApplyComposition, Cancellation::default()));
            this.operations.expanded = false;
            this.sender
                .send(Event::CompositionApplied(
                    id,
                    Ok(CompositionOutcome {
                        project: owner,
                        status: CompositionStatus::Uncertain,
                        writes: WriteState::Unknown,
                        message: Some("Report missing; inspect for partial changes".into()),
                    }),
                ))
                .unwrap();
            this.drain(cx);
            assert!(!this.composition.teardown_uncertain);
            assert!(this.operations.expanded);
            assert_eq!(
                this.watch.as_ref().unwrap().project,
                Some(std::path::PathBuf::from("Another watched project"))
            );
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("composition-flow").visible());
        window.click("Overview", cx);
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(!this.composition.visible);
        assert!(this.composition.result.is_some());
        assert!(
            this.activity
                .displayed()
                .any(|row| row.contains("could not be verified"))
        );
    });
}

#[gpui_kit::test]
fn cancelled_composition_closes_only_after_result_and_other_draft_confirmation(
    cx: &mut TestAppContext,
) {
    let (_project, handle, view) = fixture(cx);
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new(
                "Keep separate draft".into(),
            ));
            this.creation.dirty = true;
            let id = this.operations.begin(
                Operation::ApplyComposition,
                this.composition.project.clone().unwrap(),
            );
            this.running = Some((Operation::ApplyComposition, Cancellation::default()));
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
        window.click("composition-cancel-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            assert!(this.composition.close_pending);
            assert!(this.running.is_some());
            this.sender
                .send(Event::CompositionApplied(
                    id,
                    Ok(CompositionOutcome {
                        project: this.composition.project.clone().unwrap(),
                        status: CompositionStatus::Cancelled,
                        writes: WriteState::MayHaveBegun,
                        message: Some("Partial changes may remain".into()),
                    }),
                ))
                .unwrap();
            this.drain(cx);
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
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.creation.dirty);
        assert!(!this.composition.close_pending);
        assert!(this.composition.result.is_some());
    });
}

#[gpui_kit::test]
fn creation_cancel_close_cannot_discard_a_dirty_composition(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let destination = std::path::PathBuf::from("New project destination");
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            this.composition
                .draft
                .as_mut()
                .unwrap()
                .toggle_package("promise")
                .unwrap();
            this.composition.changed();
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
        window.click("cancel-creation-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.sender
                .send(Event::Created(
                    id,
                    Ok(rproj_core::creation::protocol::CreationOutcome {
                        destination,
                        status: rproj_core::creation::protocol::CreationStatus::Cancelled,
                        claim: rproj_core::creation::protocol::ClaimState::Unknown,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
        })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("composition-keep-draft").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| assert!(view.read(cx).composition.dirty));
}

#[gpui_kit::test]
fn unsupported_worker_disables_only_composition_and_keeps_the_workbench(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let id = this.composition.begin_load(this.project.clone().unwrap());
            this.composition.accept_context(
                id,
                Err(
                    "Composition editing is unavailable; use a matching portable distribution"
                        .into(),
                ),
            );
            cx.notify();
        })
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("composition-error").visible());
        window.click("composition-next", cx);
        window.click("Dependencies", cx);
        window.render_frame(cx);
        assert!(window.find("package-list").visible());
    })
    .unwrap();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(this.composition.step, Step::Dependencies);
        assert!(this.composition.unavailable);
        assert!(this.section == Section::Dependencies);
    });
}

#[gpui_kit::test]
fn loading_context_cannot_be_replaced_or_discarded_in_parallel(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let (id, captured) = cx.update(|cx| {
        view.update(cx, |this, cx| {
            let captured = this.composition.project.clone().unwrap();
            let id = this.composition.begin_load(captured.clone());
            this.open_project(std::path::PathBuf::from("Another selected project"), cx);
            (id, captured)
        })
    });
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.edit_composition(window, cx);
            this.discard_composition(window, cx);
        });
    })
    .unwrap();
    cx.update(|cx| {
        let state = &view.read(cx).composition;
        assert!(state.loading);
        assert_eq!(state.context_id, id);
        assert_eq!(state.project, Some(captured));
    });
}

#[gpui_kit::test]
fn cancelled_composition_closes_after_confirmed_teardown(cx: &mut TestAppContext) {
    let (_project, handle, view) = fixture(cx);
    let id = cx.update(|cx| {
        view.update(cx, |this, _| {
            let id = this.operations.begin(
                Operation::ApplyComposition,
                this.composition.project.clone().unwrap(),
            );
            this.running = Some((Operation::ApplyComposition, Cancellation::default()));
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
        window.click("composition-cancel-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .is_ok()
    );
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.sender
                .send(Event::CompositionApplied(
                    id,
                    Ok(CompositionOutcome {
                        project: this.composition.project.clone().unwrap(),
                        status: CompositionStatus::Cancelled,
                        writes: WriteState::MayHaveBegun,
                        message: None,
                    }),
                ))
                .unwrap();
            this.drain(cx);
        })
    });
    cx.run_until_parked();
    assert!(
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .is_err()
    );
}
