//! Cross-workflow audit fixtures. No workers, installers or native windows run.
use super::{
    Cancellation, Event, Operation, OperationEvent, RunStatus, WatchStatus, Workbench,
    project_template, saved_setups, settings, setup, source_copy, ui_tests::fixture, upgrade,
};
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, TestAppContext, px, size};
use rproj_core::creation::{CompositionDraft, CreationDraft, ProjectGraph};
use rproj_core::operations::WatchState;
use std::{path::Path, sync::Arc};

fn seed(this: &mut Workbench, root: &Path) {
    this.creation.draft = Some(CreationDraft::new("Pending creation".into()));
    this.creation.dirty = true;
    let mut draft = CompositionDraft::new(ProjectGraph::default());
    draft.toggle_package("promise").unwrap();
    this.composition.project = Some(root.join("composition owner"));
    this.composition.draft = Some(draft);
    this.composition.changed();
    this.settings = settings::audit_seed(root.join("settings owner"));
    this.setup = setup::audit_seed(root.join("machine owner"));
    this.saved = saved_setups::audit_seed(root.join("saved owner"));
    this.template = project_template::audit_seed(root.join("template owner"));
    let review = upgrade::audit_review(root.join("upgrade owner"));
    let documents = (0..review.files.len())
        .map(|i| Arc::new(review.diff(i).unwrap()))
        .collect();
    this.upgrade.project = Some(review.project.clone());
    this.upgrade.prepared = Some(upgrade::PreparedReview { review, documents });
    this.upgrade_list.reset(this.upgrade.row_count());
    source_copy::audit_seed(&mut this.source, root);
    this.watch = Some(WatchStatus {
        supported: true,
        project: Some(root.join("Watch owner")),
        state: WatchState::Watching,
        message: "fixture".into(),
    });
}

fn retained(this: &Workbench) -> Vec<String> {
    vec![
        format!("{:?}:{}", this.creation.draft, this.creation.dirty),
        format!(
            "{:?}:{:?}:{}",
            this.composition.project, this.composition.draft, this.composition.dirty
        ),
        format!("{:?}", settings::audit_state(&this.settings)),
        format!("{:?}", setup::audit_state(&this.setup)),
        format!("{:?}", saved_setups::audit_state(&this.saved)),
        format!("{:?}", project_template::audit_state(&this.template)),
        format!(
            "{:?}:{:?}",
            this.upgrade.project,
            this.upgrade.prepared.as_ref().map(|p| &p.review)
        ),
        source_copy::audit_state(&this.source),
        format!("{:?}", this.watch),
    ]
}

fn dirty(this: &Workbench) -> [bool; 6] {
    [
        project_template::audit_state(&this.template).1,
        saved_setups::audit_state(&this.saved).1,
        setup::audit_state(&this.setup).1,
        settings::audit_state(&this.settings).1,
        this.composition.dirty,
        this.creation.dirty,
    ]
}

#[gpui_kit::test]
fn audit_all_retained_sessions_survive_navigation_during_checks(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    cx.update(|cx| view.update(cx, |this, _| seed(this, root.path())));
    for (width, height, mode) in [
        (1120., 780., ThemeMode::Light),
        (1120., 780., ThemeMode::Dark),
        (800., 580., ThemeMode::Light),
        (800., 580., ThemeMode::Dark),
    ] {
        cx.simulate_window_resize(handle, size(px(width), px(height)));
        cx.update_window(handle, |_, w, cx| {
            Theme::change(mode, Some(w), cx);
            let baseline = retained(view.read(cx));
            view.update(cx, |this, _| {
                assert_eq!(dirty(this), [true; 6]);
                this.operations
                    .begin(Operation::Check, root.path().join("Check owner"));
                this.running = Some((Operation::Check, Cancellation::default()));
                this.operations.expanded = false;
                this.activity_height = px(290.);
            });
            for page in 0..9 {
                view.update(cx, |this, cx| match page {
                    0 => this.show_creation(w, cx),
                    1 => this.show_composition(w, cx),
                    2 => this.show_settings(w, cx),
                    3 => this.show_setup(w, cx),
                    4 => this.show_saved(w, cx),
                    5 => project_template::audit_show(this, w, cx),
                    6 => this.show_upgrade(w, cx),
                    7 => this.show_source(w, cx),
                    _ => this.show_catalog(None, w, cx),
                });
                w.render_frame(cx);
                let this = view.read(cx);
                assert_eq!(retained(this), baseline, "page {page}");
                assert!(this.running.is_some());
                assert!(!this.operations.expanded);
                assert_eq!(this.activity_height, px(290.));
                let visibility = [
                    this.creation.visible,
                    this.composition.visible,
                    this.settings.visible,
                    this.setup.visible,
                    this.saved.visible,
                    this.template.visible,
                    this.upgrade.visible,
                    this.source.visible,
                    this.library.visible,
                ];
                assert_eq!(visibility.iter().filter(|&&v| v).count(), 1, "page {page}");
                assert!(visibility[page]);
            }
            view.update(cx, |this, cx| {
                this.enter_project(root.path().join("browsed project"));
                this.close_project(cx);
                assert!(this.library.visible);
                assert_eq!(retained(this), baseline);
                this.running = None;
            });
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn audit_six_dirty_drafts_require_individual_close_decisions(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    cx.update(|cx| view.update(cx, |this, _| seed(this, root.path())));
    let decisions = [
        ("template-keep", "template-confirm"),
        ("saved-keep-draft", "saved-discard-continue"),
        ("setup-keep-draft", "setup-replace-draft"),
        ("settings-keep-draft", "settings-replace-draft"),
        ("composition-keep-draft", "composition-discard-draft"),
        ("keep-draft", "discard-draft"),
    ];
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            this.show_catalog(None, w, cx);
            assert!(!this.request_close(w, cx));
        });
    })
    .unwrap();
    for (index, (keep, discard)) in decisions.into_iter().enumerate() {
        cx.run_until_parked();
        cx.update_window(handle, |_, w, cx| {
            w.render_frame(cx);
            assert!(w.find(keep).visible(), "draft {index}");
            w.press("escape", cx);
            view.update(cx, |this, cx| {
                let flags = dirty(this);
                assert!(flags[index..].iter().all(|&d| d));
                assert!(!this.request_close(w, cx));
            });
            w.render_frame(cx);
            w.click(discard, cx);
        })
        .unwrap();
        // Leave the final draft intact so the headless fixture remains inspectable.
        if index == 4 {
            break;
        }
    }
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("keep-draft").visible());
        w.press("enter", cx);
        assert_eq!(
            dirty(view.read(cx)),
            [false, false, false, false, false, true]
        );
        assert_eq!(
            view.read(cx).watch.as_ref().unwrap().state,
            WatchState::Watching
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn audit_unavailable_worker_actions_and_stale_events_preserve_original_operation(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            seed(this, root.path());
            let baseline = retained(this);
            let owner = root.path().join("captured Check");
            let id = this.operations.begin(Operation::Check, owner.clone());
            this.running = Some((Operation::Check, Cancellation::default()));
            this.activity.set_origin(Some("captured Check".into()));
            for op in [Operation::Test, Operation::StartWatch, Operation::StopWatch] {
                this.start(op, cx);
            }
            this.confirm_watch_switch(w, cx);
            assert_eq!(this.operations.latest.as_ref().unwrap().id, id);
            assert!(this.pending_switch.is_none());
            this.sender
                .send(Event::Progress(
                    id + 1,
                    OperationEvent::Output {
                        stderr: false,
                        text: "stale output".into(),
                    },
                ))
                .unwrap();
            this.sender
                .send(Event::Complete(id + 1, Err("stale failure".into())))
                .unwrap();
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "current output\n".into(),
                    },
                ))
                .unwrap();
            this.drain(cx);
            assert!(this.operations.latest.as_ref().unwrap().status == RunStatus::Running);
            assert_eq!(this.operations.latest.as_ref().unwrap().project, owner);
            assert!(
                this.activity
                    .displayed()
                    .any(|l| l.contains("current output"))
            );
            assert!(!this.activity.displayed().any(|l| l.contains("stale")));
            assert_eq!(retained(this), baseline);
            this.sender
                .send(Event::Complete(id, Err("fixture failure".into())))
                .unwrap();
            this.drain(cx);
            assert!(this.running.is_none());
            let outcome = this.operations.latest.as_ref().unwrap().status.clone();
            let lines: Vec<_> = this.activity.displayed().collect();
            this.sender
                .send(Event::Complete(id, Err("duplicate failure".into())))
                .unwrap();
            this.sender
                .send(Event::Progress(
                    id,
                    OperationEvent::Output {
                        stderr: false,
                        text: "late output".into(),
                    },
                ))
                .unwrap();
            this.drain(cx);
            assert_eq!(this.operations.latest.as_ref().unwrap().status, outcome);
            assert_eq!(this.operations.latest.as_ref().unwrap().id, id);
            assert_eq!(this.activity.displayed().collect::<Vec<_>>(), lines);
        });
    })
    .unwrap();
}
