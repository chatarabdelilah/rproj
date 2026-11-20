use super::{
    Cancellation, Comparison, Entry, Local, Observation, Operation, Plan, Section, State,
    Workbench, choices, key, px,
};
use gpui_kit::AppContext;
use gpui_kit::{TestAppContext, test::TestWindowExt};
use rproj_core::operations::{WatchState, WatchStatus};

fn review(this: &mut Workbench) -> Plan {
    let owner = this.project.clone().unwrap();
    let local = Local::Tool {
        alias: "editor".into(),
        declaration: "sample/rojo@1.0.0".into(),
        executable: None,
    };
    let observed = Entry {
        local: local.clone(),
        observed: true,
        comparison: Comparison {
            status: rproj_core::updates::Status::NewerAvailable,
            latest_stable: Some("1.1.0".into()),
            within_requirement: None,
            within_requirement_status: None,
            reason: None,
        },
    };
    let plan = Plan {
        selection: choices(&owner, &observed).remove(0),
        inputs: "captured".into(),
        outputs: "reviewed".into(),
        changes: vec!["editor: 1.0.0 → 1.1.0".into()],
        blockers: vec![],
    };
    let project = this.updates.projects.entry(owner).or_default();
    project.inventory(vec![local.clone()]);
    project.fresh_check = true;
    project.observations.insert(
        key(&local),
        Observation {
            entry: observed,
            time: 42,
        },
    );
    this.section = Section::Updates;
    this.updates.apply.support = Some(Ok(()));
    this.updates.apply.plan = Some(plan.clone());
    plan
}

#[gpui_kit::test]
fn confirmation_defaults_keep_and_stale_watch_cannot_apply(cx: &mut TestAppContext) {
    let (_root, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            let p = review(this);
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some(p.selection.project.clone()),
                state: WatchState::Stopping,
                message: String::new(),
            });
            assert!(!this.can_apply_update(&p));
            this.watch.as_mut().unwrap().state = WatchState::Stopped;
            assert!(this.can_apply_update(&p));
            this.confirm_reviewed_update(w, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("update-keep-reviewing").visible());
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            assert!(this.running.is_none());
            assert!(!this.updates.apply.dialog);
            assert!(this.updates_focus.is_focused(w));
            this.confirm_reviewed_update(w, cx);
            this.watch_generation = this.watch_generation.wrapping_add(1);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("update-confirm", cx);
        assert!(view.read(cx).running.is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn completion_after_navigation_retains_origin_and_rejects_obsolete_events(cx: &mut TestAppContext) {
    let (_root, _, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            let p = review(this);
            let owner = p.selection.project.clone();
            let id = this
                .operations
                .begin(Operation::ApplyReviewedUpdate, owner.clone());
            this.updates.apply.active = Some((id, p.selection.clone()));
            this.running = Some((Operation::ApplyReviewedUpdate, Cancellation::default()));
            this.creation.dirty = true;
            this.operations.expanded = false;
            this.activity_height = px(215.);
            this.close_project(cx);
            this.finish_reviewed_update(id + 1, Ok(State::Applied), cx);
            assert!(this.running.is_some());
            this.finish_reviewed_update(id, Ok(State::Applied), cx);
            this.finish_reviewed_update(id, Ok(State::Uncertain("obsolete".into())), cx);
            assert!(this.running.is_none() && this.project.is_none() && this.creation.dirty);
            assert_eq!(this.updates.apply.confirmed, Some(id));
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(215.));
            assert!(
                this.updates.projects[&owner]
                    .summary
                    .contains("Verified editor 1.1.0")
            );
        })
    });
}

#[gpui_kit::test]
fn uncertain_completion_keeps_deferred_close_open(cx: &mut TestAppContext) {
    let (_root, handle, view) = crate::workbench::ui_tests::fixture(cx);
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, cx| {
            let p = review(this);
            let id = this
                .operations
                .begin(Operation::ApplyReviewedUpdate, p.selection.project.clone());
            this.updates.apply.active = Some((id, p.selection));
            this.running = Some((Operation::ApplyReviewedUpdate, Cancellation::default()));
            assert_eq!(this.reviewed_update_close(w, cx), Some(false));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("update-keep-open").visible());
        w.click("update-close-after", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, _, cx| {
        view.update(cx, |this, cx| {
            let id = this.updates.apply.active.as_ref().unwrap().0;
            assert!(this.updates.apply.close_after);
            this.finish_reviewed_update(id, Ok(State::Uncertain("Missing report".into())), cx);
            assert_eq!(this.updates.apply.confirmed, None);
            assert!(this.operations.expanded);
        })
    })
    .unwrap();
}
