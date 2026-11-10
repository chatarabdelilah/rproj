use super::super::ui_tests::fixture;
use super::super::{Cancellation, Operation, WatchStatus, Workbench};
use super::{Prepared, Session};

pub(in crate::workbench) fn audit_seed(s: &mut Session, root: &std::path::Path) {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/audit.txt"), "isolated audit text\r\n").unwrap();
    let p = prepared(root);
    s.request(p.raw.project.clone());
    let r = s.take_request().unwrap();
    assert_eq!(s.accept(r.id, Ok(p)), Some(false));
}
pub(in crate::workbench) fn audit_state(s: &Session) -> String {
    format!("{:?}:{:?}", s.owner, s.snapshot.as_ref().map(|p| &p.raw))
}
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::test::TestWindowExt;
use gpui_kit::{AnyWindowHandle, AppContext, Entity, TestAppContext, px, size};
use std::{path::PathBuf, sync::atomic::Ordering};

fn prepared(root: &std::path::Path) -> Prepared {
    Prepared::new(rproj_core::source_copy::collect(root).unwrap(), || false).unwrap()
}

fn seed(
    cx: &mut TestAppContext,
    root: &std::path::Path,
    handle: AnyWindowHandle,
    view: &Entity<Workbench>,
) -> String {
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/雪.txt"),
        format!("secret source\r\n\0\t\u{202e}{}", "\nline".repeat(80)),
    )
    .unwrap();
    std::fs::write(root.join("src/empty"), "").unwrap();
    let value = prepared(root);
    let text = value.raw.text.clone();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.source.request(value.raw.project.clone());
            let request = this.source.take_request().unwrap();
            this.accept_source(request.id, Ok(value), cx);
            this.show_source(window, cx);
        });
        window.render_frame(cx);
    })
    .unwrap();
    text
}

#[gpui_kit::test]
fn copied_snapshot_is_exact_after_disk_changes_and_failed_refresh(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let text = seed(cx, root.path(), handle, &view);
    std::fs::write(root.path().join("src/雪.txt"), "new disk contents").unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.click("source-copy", cx);
        assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), text);
        view.update(cx, |this, cx| {
            assert!(this.source.status.starts_with("Copy requested"));
            let owner = this.source.owner.clone().unwrap();
            this.source.request(owner);
            let request = this.source.take_request().unwrap();
            this.accept_source(request.id, Err("permission denied".into()), cx);
            assert!(this.source.failed_refresh);
            assert!(this.source.copyable());
        });
        window.render_frame(cx);
        window.click("source-copy", cx);
        assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), text);
        assert!(view.read(cx).source.status.contains("Previous snapshot"));
        assert!(
            view.read(cx)
                .activity
                .lines
                .iter()
                .all(|s| !s.contains("secret source"))
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn latest_queued_request_is_bounded_and_stale_results_cannot_replace_review(
    cx: &mut TestAppContext,
) {
    let (root, handle, view) = fixture(cx);
    seed(cx, root.path(), handle, &view);
    cx.update(|cx| {
        view.update(cx, |this, _| {
            let source = &mut this.source;
            source.request("first".into());
            let first = source.take_request().unwrap();
            source.request("second".into());
            source.request("latest".into());
            assert!(first.abandoned.load(Ordering::Relaxed));
            assert!(source.take_request().is_none());
            assert_eq!(
                source.queued.as_ref().unwrap().project,
                PathBuf::from("latest")
            );
            assert!(
                source
                    .accept(first.id + 100, Ok(prepared(root.path())))
                    .is_none()
            );
            assert!(source.active.is_some());
            assert!(source.accept(first.id, Ok(prepared(root.path()))).is_none());
            assert!(source.snapshot.is_none());
            let latest = source.take_request().unwrap();
            assert_eq!(latest.project, PathBuf::from("latest"));
            assert_eq!(source.accept(latest.id, Err("missing".into())), Some(true));
            assert!(!source.busy());
            assert!(!source.copyable());
        })
    });
}

#[gpui_kit::test]
fn partial_and_empty_states_are_explicit_without_worker_or_record(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    seed(cx, root.path(), handle, &view);
    std::fs::write(root.path().join("src/binary"), [255]).unwrap();
    let partial = prepared(root.path());
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.client = None;
            this.snapshot = None;
            this.client_error = Some("worker not installed".into());
            this.source.request(partial.raw.project.clone());
            let r = this.source.take_request().unwrap();
            this.accept_source(r.id, Ok(partial), cx);
        });
        window.render_frame(cx);
        assert!(window.find("source-review").visible());
        assert!(window.find("source-skips").visible());
        window.click("source-copy", cx);
        assert!(view.read(cx).source.status.contains("1 skipped"));
        assert!(
            !cx.read_from_clipboard()
                .unwrap()
                .text()
                .unwrap()
                .contains("binary")
        );
    })
    .unwrap();
    let empty = tempfile::tempdir().unwrap();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.source.request(empty.path().into());
            let r = this.source.take_request().unwrap();
            this.accept_source(r.id, Ok(prepared(empty.path())), cx);
            assert!(this.source.status.contains("No src"));
            assert!(!this.source.copyable());
        });
        window.render_frame(cx);
        assert!(window.find("source-copy").visible());
        assert!(window.try_find("source-preview").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn navigation_retains_review_watch_drafts_and_operation_stream_ownership(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let text = seed(cx, root.path(), handle, &view);
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            let project = this.source.owner.clone().unwrap();
            this.watch = Some(WatchStatus {
                state: rproj_core::operations::WatchState::Watching,
                project: Some(project.clone()),
                supported: true,
                message: "Watching".into(),
            });
            this.creation.draft = Some(rproj_core::creation::CreationDraft::new("Retained".into()));
            let id = this.operations.begin(Operation::Check, project.clone());
            this.running = Some((Operation::Check, Cancellation::default()));
            this.activity.set_origin(Some("ongoing checks".into()));
            this.activity.append_stream(false, "part");
            this.source_notice("Collected 2 files", false);
            this.activity.append_stream(false, "ial\n");
            let last = this.activity.lines.len() - 1;
            assert_eq!(this.activity.lines[last], "partial");
            assert_eq!(
                this.activity.origins[last].as_deref(),
                Some("ongoing checks")
            );
            assert_eq!(this.operations.latest.as_ref().unwrap().id, id);
            assert!(this.source_activity_summary(false).contains("Running"));
            this.enter_project(root.path().join("another"));
            this.close_project(cx);
            assert!(this.source.visible);
            assert_eq!(this.source.owner.as_ref(), Some(&project));
            assert_eq!(
                this.watch.as_ref().unwrap().project.as_ref(),
                Some(&project)
            );
            assert_eq!(this.creation.draft.as_ref().unwrap().name, "Retained");
            this.copy_source_snapshot(this.source.revision, cx);
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), text);
            this.running = None;
            this.creation.draft = None;
            assert!(
                this.request_close(window, cx),
                "read-only review adds no close prompt"
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn replacement_defaults_to_keep_and_escape_restores_focus(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    seed(cx, root.path(), handle, &view);
    let owner = root.path().canonicalize().unwrap();
    cx.update_window(handle, |_, window, cx| {
        view.update(cx, |this, cx| {
            this.project = Some(root.path().join("other"));
            this.source.focus.focus(window, cx);
            this.edit_source(window, cx);
        });
        window.render_frame(cx);
        assert!(window.find("source-keep-review").visible());
        window.press("escape", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        let source = &view.read(cx).source;
        assert_eq!(source.owner.as_ref(), Some(&owner));
        assert!(!source.dialog_open);
        assert!(source.focus.is_focused(window));
    })
    .unwrap();
}

#[gpui_kit::test]
fn wide_narrow_themes_and_file_navigation_preserve_full_snapshot(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    let text = seed(cx, root.path(), handle, &view);
    for (width, height, mode, layout) in [
        (1120., 780., ThemeMode::Light, "source-adjacent"),
        (800., 580., ThemeMode::Dark, "source-stacked"),
    ] {
        cx.simulate_window_resize(handle, size(px(width), px(height)));
        cx.update_window(handle, |_, window, cx| {
            Theme::change(mode, Some(window), cx);
            window.render_frame(cx);
            assert!(window.find(layout).visible());
            view.update(cx, |this, cx| this.source.files_focus.focus(window, cx));
            window.press("end", cx);
            let source = &view.read(cx).source;
            assert_eq!(source.selected, 1);
            assert_eq!(source.snapshot.as_ref().unwrap().raw.text, text);
            assert_eq!(
                source.rows.logical_scroll_top().item_ix,
                source.snapshot.as_ref().unwrap().file_rows[1]
            );
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn completion_respects_collapse_and_drop_abandons_reads(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    seed(cx, root.path(), handle, &view);
    cx.update(|cx| {
        view.update(cx, |this, cx| {
            this.source.request(root.path().canonicalize().unwrap());
            let request = this.source.take_request().unwrap();
            this.operations.expanded = false;
            this.activity_height = px(333.);
            this.accept_source(request.id, Ok(prepared(root.path())), cx);
            assert!(!this.operations.expanded);
            assert_eq!(this.activity_height, px(333.));
            this.source.request(root.path().into());
            let request = this.source.take_request().unwrap();
            let old = std::mem::replace(&mut this.source, Session::new(cx));
            drop(old);
            assert!(request.abandoned.load(Ordering::Relaxed));
        })
    });
}

#[gpui_kit::test]
fn background_collection_needs_no_worker_and_never_copies_automatically(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/plain.txt"), "background text").unwrap();
    cx.update_window(handle, |_, window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("unchanged".into()));
        view.update(cx, |this, cx| {
            this.client = None;
            this.prepare_source(root.path().into(), cx);
            assert!(this.running.is_none());
            assert!(!this.source.copyable());
            this.close_project(cx);
            let event = this
                .receiver
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            let super::super::Event::SourcePrepared(id, result) = event else {
                panic!("unexpected event")
            };
            this.accept_source(id, *result, cx);
            assert!(this.source.copyable());
            assert!(this.project.is_none());
            assert!(this.operations.latest.is_none());
            this.show_source(window, cx);
        });
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "unchanged"
        );
        window.render_frame(cx);
        assert!(window.find("source-copy").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn stale_copy_and_replacement_requests_keep_the_current_snapshot(cx: &mut TestAppContext) {
    let (root, handle, view) = fixture(cx);
    seed(cx, root.path(), handle, &view);
    cx.update_window(handle, |_, window, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("unchanged".into()));
        view.update(cx, |this, cx| {
            this.copy_source_snapshot(this.source.revision + 1, cx);
            this.project = Some(root.path().join("destination"));
            this.edit_source(window, cx);
        });
        window.render_frame(cx);
        window.press("enter", cx);
        assert!(
            !view.read(cx).source.dialog_open,
            "default confirmation keeps the review"
        );
        view.update(cx, |this, cx| {
            this.edit_source(window, cx);
            this.project = Some(root.path().join("changed while dialog open"));
        });
        window.render_frame(cx);
        window.click("source-replace-review", cx);
        assert_eq!(
            view.read(cx).source.owner.as_ref(),
            Some(&root.path().canonicalize().unwrap())
        );
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "unchanged"
        );
    })
    .unwrap();
}
