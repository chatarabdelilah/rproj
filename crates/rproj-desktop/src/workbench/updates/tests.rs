use super::{Comparison, Entry, Group, Local, Report, Session};
use gpui_kit::{AppContext, TestAppContext, px, size};
use std::time::Instant;

#[cfg(windows)]
#[gpui_kit::test]
fn explicit_worker_check_finishes_for_closed_project_with_watch_and_collapsed_activity(
    cx: &mut TestAppContext,
) {
    use crate::workbench::{Operation, Section};
    use rproj_core::operations::{WatchState, WatchStatus, WorkbenchClient};
    let (root, handle, view) = super::super::ui_tests::fixture(cx);
    let owner = root.path().canonicalize().unwrap();
    let local = tool("1.0.0");
    let mut report = result(local.clone(), true);
    report.project = owner.clone();
    let json = serde_json::to_string(&report).unwrap();
    let source = root.path().join("worker.rs");
    let worker = root.path().join("worker.exe");
    std::fs::write(
        &source,
        format!(
            r#"fn main() {{
        let args:Vec<_>=std::env::args().skip(1).collect();
        if args==["--version"] {{println!("rproj {}");return;}}
        assert_eq!(args,["--rproj-workbench","updates-v1"]);
        println!("{{}}",{json:?});
    }}"#,
            env!("CARGO_PKG_VERSION")
        ),
    )
    .unwrap();
    assert!(
        std::process::Command::new("rustc")
            .arg(source)
            .arg("-o")
            .arg(&worker)
            .status()
            .unwrap()
            .success()
    );
    let client = WorkbenchClient::new(worker).unwrap();
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, _| {
            this.section = Section::Updates;
            this.updates
                .projects
                .entry(owner.clone())
                .or_default()
                .inventory(vec![local.clone()]);
            this.updates.support = Some(Ok(()));
            this.client = Some(client);
            this.watch = Some(WatchStatus {
                supported: true,
                project: Some(owner.clone()),
                state: WatchState::Watching,
                message: "Watching".into(),
            });
            this.last_poll = Instant::now();
        });
        w.render_frame(cx);
        w.click("check-updates", cx);
        view.update(cx, |this, cx| {
            assert!(this.updates.projects[&owner].pending.is_some());
            this.operations.expanded = false;
            this.activity_height = px(217.);
            this.close_project(cx);
            this.running = Some((
                Operation::Check,
                rproj_core::operations::Cancellation::default(),
            ));
        });
    })
    .unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(3);
    loop {
        let done = cx.update(|cx| {
            view.update(cx, |this, cx| {
                this.drain(cx);
                this.updates.projects[&owner].pending.is_none()
            })
        });
        if done {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "sample worker completion timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    cx.update(|cx| {
        let this = view.read(cx);
        assert!(this.project.is_none());
        assert_eq!(this.watch.as_ref().unwrap().project.as_ref(), Some(&owner));
        assert_eq!(
            this.updates.projects[&owner]
                .current(&local)
                .0
                .unwrap()
                .time,
            42
        );
        assert!(!this.operations.expanded);
        assert_eq!(this.activity_height, px(217.));
        assert!(
            this.activity
                .origins
                .iter()
                .flatten()
                .any(|s| s.contains(&owner.display().to_string()) && s.contains("Updates"))
        );
        assert!(matches!(this.running, Some((Operation::Check, _))));
    });
}
use gpui_kit::test::TestWindowExt;
use rproj_core::updates::Status;

fn tool(pin: &str) -> Local {
    Local::Tool {
        alias: "rojo".into(),
        declaration: format!("sample/rojo@{pin}"),
        executable: None,
    }
}
fn result(local: Local, observed: bool) -> Report {
    Report {
        schema_version: 1,
        project: "Skyline".into(),
        name: None,
        checked_at_unix_seconds: 42,
        diagnostics: vec![],
        entries: vec![Entry {
            local,
            observed,
            comparison: if observed {
                Comparison {
                    status: Status::NewerAvailable,
                    latest_stable: Some("2.0.0".into()),
                    within_requirement: None,
                    within_requirement_status: None,
                    reason: None,
                }
            } else {
                Comparison::unknown("Offline")
            },
        }],
    }
}
#[test]
fn retained_reports_respect_input_identity_concurrent_projects_and_obsolete_results() {
    let mut session = Session::default();
    let local = tool("1.0.0");
    let skyline = session.projects.entry("Skyline".into()).or_default();
    skyline.inventory(vec![local.clone()]);
    skyline.pending = Some((1, skyline.entries.clone()));
    session
        .projects
        .entry("Lantern".into())
        .or_default()
        .pending = Some((2, vec![]));
    let skyline = session
        .projects
        .get_mut(std::path::Path::new("Skyline"))
        .unwrap();
    assert!(skyline.complete(2, Err("obsolete".into())).is_none());
    skyline
        .complete(1, Ok(result(local.clone(), true)))
        .unwrap();
    assert!(skyline.complete(1, Err("duplicate".into())).is_none());
    assert_eq!(skyline.current(&local).0.unwrap().time, 42);
    skyline.pending = Some((3, skyline.entries.clone()));
    skyline
        .complete(3, Ok(result(local.clone(), false)))
        .unwrap();
    assert!(matches!(
        skyline.current(&local),
        (Some(_), Some("Offline"), false)
    ));
    skyline.pending = Some((4, skyline.entries.clone()));
    skyline.complete(4, Err("Worker failed".into())).unwrap();
    assert_eq!(skyline.last_success, Some(42));
    let changed = tool("1.1.0");
    skyline.inventory(vec![changed.clone()]);
    assert!(matches!(skyline.current(&changed), (None, None, true)));
    skyline.pending = Some((5, vec![local]));
    let (msg, failed) = skyline.complete(5, Ok(result(changed, true))).unwrap();
    assert!(failed && msg.contains("changed"));
    assert_eq!(
        session.projects[std::path::Path::new("Lantern")]
            .pending
            .as_ref()
            .unwrap()
            .0,
        2
    );
    assert!(Session::default().projects.is_empty());
}

#[gpui_kit::test]
fn both_entry_points_show_local_rows_without_worker_or_automatic_check(cx: &mut TestAppContext) {
    let (root, handle, view) = super::super::ui_tests::fixture(cx);
    let inventory = rproj_core::updates::inventory(root.path()).unwrap();
    cx.update_window(handle, |_, w, cx| {
        view.update(cx, |this, _| {
            this.updates
                .projects
                .entry(inventory.project.clone())
                .or_default()
                .inventory(inventory.entries.clone());
            this.updates.support = Some(Err("Matching worker required".into()));
            this.last_poll = Instant::now();
        });
        w.render_frame(cx);
        w.click("Dependencies", cx);
        w.render_frame(cx);
        w.click("view-updates", cx);
        w.render_frame(cx);
        assert!(w.find("updates-adjacent").visible());
        assert!(w.find("update-row-dependencies:Signal").visible());
        w.click("update-row-dependencies:Unfamiliar", cx);
        w.click("check-updates", cx);
        assert_eq!(view.read(cx).updates.sequence, 0);
    })
    .unwrap();
    cx.simulate_window_resize(handle, size(px(800.), px(580.)));
    cx.update_window(handle, |_, w, cx| {
        w.render_frame(cx);
        assert!(w.find("updates-stacked").visible());
        assert_eq!(
            view.read(cx).updates.projects[&inventory.project]
                .selected
                .as_deref(),
            Some("dependencies:Unfamiliar")
        );
        w.click("Tools", cx);
        w.render_frame(cx);
        w.click("view-updates", cx);
        w.render_frame(cx);
        assert!(view.read(cx).updates.projects[&inventory.project].group == Group::Tools);
        assert_eq!(view.read(cx).updates.sequence, 0);
    })
    .unwrap();
}
