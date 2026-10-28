use super::{Failure, Response, Task, path};
use crate::workbench::{Workbench, creation};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Entity, ScrollDelta, TestAppContext, Window, point, px, size,
};
use rproj_core::operations::WatchState;
use rproj_core::{
    operations::{Cancellation, WatchStatus},
    saved_setups::{
        ManagementOutcome, ManagementReview, ManagementStatus, MutationState,
        protocol::ReusedSetup, reusable_graph,
    },
};
use std::path::PathBuf;

fn fixture(cx: &mut TestAppContext) -> (tempfile::TempDir, AnyWindowHandle, Entity<Workbench>) {
    let (root, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.saved = super::tests::session(root.path().canonicalize().unwrap());
            cx.notify();
        })
    });
    (root, handle, view)
}
fn edit(view: &Entity<Workbench>, cx: &mut TestAppContext) -> ManagementReview {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.saved
                .begin_edit(this.saved.selected.clone().unwrap())
                .unwrap();
            this.saved
                .edit(|d| d.set_capability("format", Some("stylua")));
            let review = super::tests::prepare(&mut this.saved);
            this.saved_list.reset(this.saved.row_count());
            cx.notify();
            review
        })
    })
}
fn running(view: &Entity<Workbench>, cx: &mut TestAppContext, task: Task) -> (u64, u64) {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let op = task.operation();
            let source = task
                .source()
                .map(path)
                .unwrap_or_else(|| PathBuf::from("Saved Setups"));
            let id = this.operations.begin(op, source);
            this.activity
                .set_origin(this.operations.latest.as_ref().map(|r| r.origin()));
            this.saved.pending = Some(task);
            this.running = Some((op, Cancellation::default()));
            let revision = this.saved.revision;
            cx.notify();
            (id, revision)
        })
    })
}
fn success(review: &ManagementReview) -> ManagementOutcome {
    let mut saved = review.source.clone();
    if let Some(bytes) = review
        .document
        .as_ref()
        .and_then(|d| d.replacement.as_ref())
    {
        saved.bytes = bytes.as_bytes().to_vec();
        saved.source_identity = "replaced".into();
    }
    ManagementOutcome {
        status: ManagementStatus::Succeeded,
        mutation: MutationState::Completed,
        source: path(&review.source),
        destination: review.destination.clone(),
        saved: Some(saved),
        message: None,
    }
}
fn reveal(w: &mut Window, cx: &mut App, scroll: &'static str, control: &'static str) {
    let viewport = w.find(scroll).bounds();
    let target = w.find(control).bounds();
    w.scroll(
        scroll,
        ScrollDelta::Pixels(point(px(0.), viewport.center().y - target.center().y)),
        cx,
    );
    w.render_frame(cx);
}

#[gpui_kit::test]
fn saved_browser_wide_narrow_search_and_disabled_reuse(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    for dimensions in [size(px(1120.), px(780.)), size(px(800.), px(580.))] {
        cx.simulate_window_resize(handle, dimensions);
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(
                w.try_find(if dimensions.width == px(800.) {
                    "saved-stacked"
                } else {
                    "saved-adjacent"
                })
                .is_some()
            );
            assert!(w.find("saved-flow").visible());
            reveal(w, cx, "saved-details", "saved-edit");
            assert!(w.find("saved-edit").visible());
            reveal(w, cx, "saved-details", "saved-use");
            w.click("saved-use", cx);
            assert!(view.read(cx).saved.pending.is_none());
            assert!(view.read(cx).creation.draft.is_none());
        })
        .unwrap();
    }
    cx.update_window(handle, |_, w, cx| {
        w.click("saved-search", cx);
        w.input("not-a-setup", cx);
        w.render_frame(cx);
        assert!(w.try_find("saved-item-Example").is_none());
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).saved.selected.as_ref().unwrap().source.name,
            "Example"
        )
    });
}
#[gpui_kit::test]
fn saved_review_defaults_keep_and_dialogs_restore_focus(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    edit(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("saved-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("saved-keep-reviewing").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert_eq!(w.find("saved-flow").focused(), Some(true));
        w.click("saved-editor-reload", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("saved-keep-draft").visible());
        w.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(view.read(cx).saved.dirty());
        assert!(view.read(cx).running.is_none());
    });
}
#[gpui_kit::test]
fn saved_editor_steps_keyboard_and_required_files(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    edit(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("saved-step-Capabilities", cx);
        w.render_frame(cx);
        reveal(w, cx, "saved-editor-scroll", "saved-cap-format-none");
        w.click("saved-cap-format-none", cx);
        w.render_frame(cx);
        assert!(!view.read(cx).saved.dirty());
        for _ in 0..100 {
            if w.find("saved-cap-format-stylua").focused() == Some(true) {
                break;
            }
            w.press("tab", cx);
        }
        assert_eq!(w.find("saved-cap-format-stylua").focused(), Some(true));
        w.press("space", cx);
        assert!(view.read(cx).saved.dirty());
        w.click("saved-step-Files", cx);
        w.render_frame(cx);
        assert!(w.try_find("saved-file-src").is_some());
    })
    .unwrap();
}
#[gpui_kit::test]
fn saved_navigation_keeps_draft_and_watch_owner(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    edit(&view, cx);
    let watched = root.path().join("watched");
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some(watched.clone()),
                state: WatchState::Watching,
                message: String::new(),
            })
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("Overview", cx);
    })
    .unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            assert!(this.saved.dirty());
            assert_eq!(this.saved.name(), "Example");
            assert_eq!(this.watch.as_ref().unwrap().project, Some(watched));
        })
    });
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| this.show_saved(w, cx));
        w.render_frame(cx);
        assert!(
            w.find("saved-draft-owner")
                .label()
                .unwrap()
                .contains("Example")
        );
    })
    .unwrap();
}
#[gpui_kit::test]
fn saved_events_ignore_stale_ids_and_revisions_and_preserve_activity(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    let review = edit(&view, cx);
    let (id, rev) = running(&view, cx, Task::Manage(Box::new(review.clone())));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.operations.expanded = false;
            this.activity_height = px(240.);
            this.activity.append("saved output marker");
            this.accept_saved(id + 1, rev, Ok(Response::Managed(success(&review))), cx);
            assert!(this.running.is_some());
            this.accept_saved(id, rev + 1, Ok(Response::Managed(success(&review))), cx);
            assert!(this.running.is_some());
            this.close_project(cx);
            this.accept_saved(id, rev, Ok(Response::Managed(success(&review))), cx);
            assert!(this.running.is_none());
            assert!(!this.saved.dirty());
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(240.));
            assert!(
                this.activity
                    .displayed()
                    .any(|s| s.contains("Example.toml") && s.contains("saved output marker"))
            );
            this.accept_saved(
                id,
                rev,
                Err(Failure {
                    message: "late failure".into(),
                    teardown: false,
                    unconfirmed: false,
                }),
                cx,
            );
            assert!(!this.operations.expanded);
        })
    });
}
#[gpui_kit::test]
fn saved_uncertainty_and_teardown_keep_window_open_and_retain_edits(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let review = edit(&view, cx);
    let (id, rev) = running(&view, cx, Task::Manage(Box::new(review.clone())));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.saved.close_pending = true;
            let mut result = success(&review);
            result.status = ManagementStatus::Uncertain;
            result.mutation = MutationState::Unknown;
            result.saved = None;
            this.accept_saved(id, rev, Ok(Response::Managed(result)), cx);
            assert!(this.saved.dirty());
            assert!(!this.saved.editable());
            assert_eq!(this.saved.last_confirmed, Some((id, false)));
        })
    });
    let (id, rev) = running(&view, cx, Task::Manage(Box::new(review)));
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.saved.close_pending = true;
            this.accept_saved(
                id,
                rev,
                Err(Failure {
                    message: "Owned teardown unconfirmed".into(),
                    teardown: true,
                    unconfirmed: false,
                }),
                cx,
            );
            assert!(this.teardown_uncertain());
            assert!(!this.request_close(w, cx));
        });
    })
    .unwrap();
}
#[gpui_kit::test]
fn saved_close_completion_race_defaults_keep_then_protects_creation_draft(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let review = edit(&view, cx);
    let (id, rev) = running(&view, cx, Task::Manage(Box::new(review.clone())));
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new("Keep me".into()));
            this.creation.dirty = true;
            assert!(!this.request_close(w, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_saved(id, rev, Ok(Response::Managed(success(&review))), cx)
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("saved-keep-open").visible());
        w.click("saved-close-after", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("keep-draft").visible());
        w.press("escape", cx);
    })
    .unwrap();
}
#[gpui_kit::test]
fn saved_handoff_protects_creation_and_keeps_other_drafts(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let mut loaded = cx.update(|cx| view.read(cx).saved.selected.clone().unwrap());
    loaded.source.bytes = b"package_workflow='none'\nmetadata='preserved'\n".to_vec();
    let reused = ReusedSetup {
        graph: reusable_graph(&loaded.source.bytes).unwrap(),
        source: loaded.source,
    };
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new("Keep me".into()));
            this.creation.dirty = true;
            this.offer_saved_handoff(reused.clone(), w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("saved-keep-creation").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(
            view.read(cx).creation.draft.as_ref().unwrap().name,
            "Keep me"
        )
    });
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.offer_saved_handoff(reused.clone(), w, cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("saved-replace-creation", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(this.creation.step, creation::Step::Project);
        assert!(this.creation.draft.as_ref().unwrap().name.is_empty());
        assert_eq!(this.creation.selected_setup.as_deref(), Some("Example"));
        assert_eq!(
            this.creation.setup_source.as_ref().unwrap().bytes,
            Some(reused.source.bytes)
        );
        assert!(this.creation.dirty);
        assert!(!this.saved.visible);
    });
}
#[gpui_kit::test]
fn saved_complete_diff_and_long_paths_fit_both_themes_with_activity(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    edit(&view, cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.saved.show_diff = true;
            this.operations.expanded = true;
            this.activity_height = px(144.);
            cx.notify();
        })
    });
    for dimensions in [size(px(1120.), px(780.)), size(px(800.), px(580.))] {
        cx.simulate_window_resize(handle, dimensions);
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            cx.update_window(handle, |_, w, cx| {
                Theme::change(mode, Some(w), cx);
                w.render_frame(cx);
                assert!(w.find("saved-flow").visible());
                assert!(w.find("saved-diff").visible());
                assert!(w.find("saved-apply").visible());
                let diff = w.find("saved-diff").bounds();
                assert!(diff.size.height > px(0.));
            })
            .unwrap();
        }
    }
}

#[gpui_kit::test]
fn saved_reload_requirements_remain_owned_across_other_results(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    let review = edit(&view, cx);
    let first = path(&review.source);
    let mut other = review.clone();
    other.source.name = "Other".into();
    let second = path(&other.source);
    for operation in [review.clone(), other.clone()] {
        let (id, revision) = running(&view, cx, Task::Manage(Box::new(operation.clone())));
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                let mut outcome = success(&operation);
                outcome.status = ManagementStatus::Uncertain;
                outcome.mutation = MutationState::Unknown;
                outcome.saved = None;
                this.accept_saved(id, revision, Ok(Response::Managed(outcome)), cx);
            })
        });
    }
    let (id, revision) = running(&view, cx, Task::Manage(Box::new(other.clone())));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.accept_saved(id, revision, Ok(Response::Managed(success(&other))), cx);
            assert!(this.saved.reload_sources.contains(&first));
            assert!(!this.saved.reload_sources.contains(&second));
            assert!(this.saved.dirty());
            assert!(!this.saved.editable());
            assert_eq!(this.saved.name(), "Example");
        })
    });
}

#[gpui_kit::test]
fn saved_dirty_management_and_replacement_default_keep(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    edit(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            let selected = this.saved.selected.clone().unwrap();
            this.request_saved(super::controller::Intent::Name(selected, true), w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("saved-keep-draft").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        assert!(view.read(cx).saved.dirty());
        assert!(view.read(cx).saved.naming.is_none());
        view.update(cx, |this, cx| {
            let mut other = this.saved.selected.clone().unwrap();
            other.source.name = "Other".into();
            this.request_saved(super::controller::Intent::Edit(other), w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(view.read(cx).saved.name(), "Example");
        assert!(view.read(cx).saved.dirty());
    });
}

#[gpui_kit::test]
fn saved_management_has_no_cancel_and_activity_remains_searchable(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let review = edit(&view, cx);
    running(&view, cx, Task::Manage(Box::new(review)));
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.activity.append("keep marker\nother output");
            this.activity.flush();
            cx.notify();
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("cancel-operation").is_none());
        assert!(!view.read(cx).running.as_ref().unwrap().0.cancellable());
        w.click("output-filter", cx);
        w.input("not present", cx);
        w.render_frame(cx);
        assert_eq!(w.find("output-empty").label(), Some("No search matches."));
    })
    .unwrap();
}

#[gpui_kit::test]
fn saved_browsing_does_not_replace_explicit_reload(cx: &mut TestAppContext) {
    let (_root, _handle, view) = fixture(cx);
    let loaded = cx.update(|cx| view.read(cx).saved.selected.clone().unwrap());
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.saved.reload_sources.insert(path(&loaded.source));
        })
    });
    for purpose in [
        super::LoadPurpose::Browse,
        super::LoadPurpose::Reload {
            replace_draft: false,
        },
    ] {
        let (id, revision) = running(&view, cx, Task::Load(loaded.source.name.clone(), purpose));
        cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.accept_saved(id, revision, Ok(Response::Loaded(loaded.clone())), cx);
                assert_eq!(
                    this.saved.reload_sources.contains(&path(&loaded.source)),
                    matches!(purpose, super::LoadPurpose::Browse)
                );
            })
        });
    }
}

#[gpui_kit::test]
fn saved_failed_edit_preserves_error_and_previous_review(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            let mut missing = this.saved.selected.clone().unwrap();
            this.saved.begin_edit(missing.clone()).unwrap();
            let original = super::tests::prepare(&mut this.saved);
            missing.source.name = "Broken".into();
            missing.graph = None;
            this.request_saved(super::controller::Intent::Edit(missing), w, cx);
            assert_eq!(this.saved.error.as_deref(), Some("No readable composition"));
            assert_eq!(this.saved.name(), "Example");
            assert_eq!(this.saved.prepared.as_ref().unwrap().review, original);
            assert!(this.saved.pending.is_none());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn saved_resume_restores_diff_rows_after_preparation_while_browsing(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    let review = edit(&view, cx);
    let (id, revision) = running(
        &view,
        cx,
        Task::Prepare(review.source.clone(), review.action.clone()),
    );
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.saved.editing = false;
            this.saved.show_diff = true;
            let rows = std::sync::Arc::new(
                review
                    .document
                    .as_ref()
                    .unwrap()
                    .diff(&review.source.name)
                    .unwrap(),
            );
            this.accept_saved(
                id,
                revision,
                Ok(Response::Prepared(super::Prepared { review, rows })),
                cx,
            );
            assert_eq!(this.saved.row_count(), 0);
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("saved-resume", cx);
        w.render_frame(cx);
        assert!(w.find("saved-row-0").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn saved_close_review_choice_opens_hidden_editor(cx: &mut TestAppContext) {
    let (_root, handle, view) = fixture(cx);
    edit(&view, cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.saved.visible = false;
            assert!(!this.request_close(w, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("saved-review-draft", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(view.read(cx).saved.visible);
        assert!(view.read(cx).saved.dirty());
        assert!(w.find("saved-draft-owner").visible());
    })
    .unwrap();
}
