use super::{Lane, inspect_target};
#[test]
fn revision_lanes_reject_old_results_and_serialize_repeated_requests() {
    let mut lane = Lane::default();
    lane.invalidate();
    let first = lane.start().unwrap();
    lane.invalidate();
    lane.invalidate();
    assert!(lane.start().is_none());
    assert_eq!(lane.finish(first + 99), None);
    assert_eq!(lane.finish(first), Some(false));
    let latest = lane.start().unwrap();
    assert_eq!(lane.finish(latest), Some(true));
}
#[test]
fn open_rechecks_disappeared_folders_and_preserves_malformed_records_as_diagnostics() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap().join("Lantern");
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("rproj.toml"), "bad = [").unwrap();
    let snapshot = inspect_target(&path).unwrap();
    assert_eq!(snapshot.files.rproj.state, rproj_core::FileState::Invalid);
    std::fs::remove_file(path.join("rproj.toml")).unwrap();
    assert!(inspect_target(&path).is_err());
    std::fs::remove_dir(&path).unwrap();
    assert!(inspect_target(&path).is_err());
}

use crate::workbench::{Operation, Section, Workbench, projects::Scan, ui_tests::fixture};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AppContext, Entity, TestAppContext, px, size};
use rproj_core::{
    operations::{Cancellation, WatchState, WatchStatus},
    project_discovery::discover,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn recorded(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("rproj.toml"), "bad = [").unwrap();
    fs::write(
        path.join("default.project.json"),
        r#"{"name":"Lantern","tree":{}}"#,
    )
    .unwrap();
    path.canonicalize().unwrap()
}
fn pump(cx: &mut TestAppContext, view: &Entity<Workbench>, ready: impl Fn(&Workbench) -> bool) {
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        if cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.drain(cx);
                ready(this)
            })
        }) {
            break;
        }
        assert!(Instant::now() < until, "browser result timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn seed(cx: &mut TestAppContext, view: &Entity<Workbench>, root: &Path, launch: &Path) {
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.browser.visible = true;
            this.browser.scan.invalidate();
            let id = this.browser.scan.start().unwrap();
            this.accept_projects_scan(
                id,
                Scan {
                    root: Some(root.into()),
                    found: discover(&Ok(root.into()), Some(launch)),
                },
                cx,
            );
        })
    });
    pump(cx, view, |this| this.browser.detail.is_some());
}

#[gpui_kit::test]
fn preview_and_fresh_open_keep_watch_drafts_activity_and_the_original_operation(
    cx: &mut TestAppContext,
) {
    let (work, handle, view) = fixture(cx);
    let active = work.path().canonicalize().unwrap();
    let roots = tempfile::tempdir().unwrap();
    let lantern = recorded(roots.path(), "Lantern");
    let outside = tempfile::tempdir().unwrap();
    let relay = recorded(outside.path(), "Relay");
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.section = Section::Tools;
            this.project = Some(active.clone());
            this.watch = Some(WatchStatus {
                supported: true,
                state: WatchState::Watching,
                project: Some(active.clone()),
                message: "Skyline".into(),
            });
            this.creation.dirty = true;
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new("Orchard".into()));
            this.running = Some((Operation::Check, Cancellation::default()));
            this.operations.begin(Operation::Check, active.clone());
            this.activity.append("captured on Skyline");
            this.preferences.recent_projects.clear();
            cx.notify();
        })
    });
    seed(cx, &view, roots.path(), &relay);
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("projects-adjacent").is_some());
        assert_eq!(view.read(cx).browser.selected.as_ref(), Some(&lantern));
        assert_eq!(
            view.read(cx)
                .browser
                .detail
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .files
                .rproj
                .state,
            rproj_core::FileState::Invalid
        );
        assert!(view.read(cx).preferences.recent_projects.is_empty());
        w.click("project-row-1", cx);
    })
    .unwrap();
    pump(cx, &view, |this| this.browser.detail.is_some());
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("projects-stacked").is_some());
        assert_eq!(view.read(cx).project.as_ref(), Some(&active));
        assert_eq!(view.read(cx).browser.selected.as_ref(), Some(&relay));
        view.update(cx, |this, cx| this.open_previewed_project(cx));
    })
    .unwrap();
    pump(cx, &view, |this| !this.browser.visible);
    cx.update(|cx| {
        let this = view.read(cx);
        assert_eq!(this.section, Section::Overview);
        assert_eq!(this.project.as_ref(), Some(&relay));
        assert_eq!(this.snapshot.as_ref().unwrap().root, relay);
        assert_eq!(this.preferences.recent_projects.first(), Some(&relay));
        assert_eq!(this.watch.as_ref().unwrap().project.as_ref(), Some(&active));
        assert!(this.creation.dirty && this.creation.draft.is_some());
        assert_eq!(this.operations.latest.as_ref().unwrap().project, active);
        assert!(
            this.activity
                .lines
                .iter()
                .any(|l| l == "captured on Skyline")
        );
    });
}

#[gpui_kit::test]
fn removed_preview_and_stale_open_never_replace_the_workspace(cx: &mut TestAppContext) {
    let (work, handle, view) = fixture(cx);
    let active = work.path().canonicalize().unwrap();
    let root = tempfile::tempdir().unwrap();
    let lantern = recorded(root.path(), "Lantern");
    cx.update(|cx| {
        view.update(cx, |this, _| {
            this.project = Some(active.clone());
            this.section = Section::Checks;
            this.preferences.recent_projects.clear();
        })
    });
    seed(cx, &view, root.path(), &lantern);
    let old = inspect_target(&lantern).unwrap();
    let fingerprint = serde_json::to_string(&old).unwrap();
    fs::remove_dir_all(&lantern).unwrap();
    cx.update(|cx| view.update(cx, |this, cx| this.open_previewed_project(cx)));
    pump(cx, &view, |this| {
        matches!(this.browser.detail, Some(Err(_)))
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.try_find("projects-preview-error").is_some());
        view.update(cx, |this, cx| {
            let id = this.browser.opening.start().unwrap();
            this.back_from_projects(w, cx);
            this.accept_browser_open(id, lantern.clone(), Ok((old, fingerprint)), cx);
            assert_eq!(this.project.as_ref(), Some(&active));
            assert_eq!(this.section, Section::Checks);
            assert!(this.preferences.recent_projects.is_empty());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn empty_view_and_back_restore_focus_and_search_survives_reentry(cx: &mut TestAppContext) {
    let (work, handle, view) = fixture(cx);
    let path = work.path().canonicalize().unwrap();
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.close_project(cx);
            this.section = Section::Tools;
            this.browser.launch = Ok(path.clone());
            this.client = None;
            this.client_error = Some("Unavailable worker".into());
            cx.notify();
        })
    });
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        w.click("browse-empty", cx);
    })
    .unwrap();
    pump(cx, &view, |this| this.browser.detail.is_some());
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        view.update(cx, |this, cx| {
            this.browser
                .query
                .update(cx, |input, cx| input.replace_all("no match", w, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(view.read(cx).browser.indices.is_empty());
        w.click("projects-back", cx);
        w.render_frame(cx);
        w.render_frame(cx);
        assert_eq!(w.find("browse-empty-focus").focused(), Some(true));
        w.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.browser.visible);
        assert_eq!(this.browser.query.read(cx).value().as_ref(), "no match");
        assert!(this.project.is_none());
    });
}
