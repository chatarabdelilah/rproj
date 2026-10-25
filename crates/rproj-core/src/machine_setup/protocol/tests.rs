use super::*;
use crate::machine_setup::PathIdentity;

fn request() -> SetupRequest {
    let root = std::env::temp_dir().join("setup-protocol-fixture");
    let selection = Selection {
        root: root.join("projects"),
        apps: Default::default(),
        tools: Default::default(),
        plugins: Default::default(),
        extensions: Default::default(),
    };
    let context = SetupContext {
        config_path: root.join("config.toml"),
        original: None,
        saved: selection.clone(),
        identities: vec![PathIdentity {
            path: root,
            directory: true,
            identity: None,
        }],
        detected: Default::default(),
    };
    SetupRequest {
        protocol: SETUP_PROTOCOL,
        id: "operation-one".into(),
        action: SetupAction::Apply(SetupReview::new(context, selection).unwrap()),
    }
}

fn succeeded(request: &SetupRequest) -> SetupReport {
    let SetupAction::Apply(review) = &request.action else {
        panic!()
    };
    let mut report = SetupReport::initial(request);
    report.status = Some(SetupStatus::Succeeded);
    report.effects = WriteState::Completed;
    report.statuses = vec![Status::Installed; review.items.len()];
    let config = toml::toml! {
        roblox_projects_root = (review.selection.root.to_str().unwrap())
        selected_system_apps = []
        selected_rokit_tools = []
        selected_studio_plugins = []
        selected_vscode_extensions = []
        last_checked = "fixture"
    };
    report.result = Some(SetupResult {
        outcome: Outcome::Completed,
        statuses: report.statuses.clone(),
        changes_may_have_begun: true,
        persistence: Persistence::Saved {
            contents: toml::to_string(&config).unwrap(),
        },
    });
    report
}

#[test]
fn report_roundtrip_and_late_stop_preserve_confirmed_completion() {
    let request = request();
    let mut report = succeeded(&request);
    report.stop_acknowledged = true;
    report.check(&request).unwrap();
    let encoded = serde_json::to_vec(&report).unwrap();
    assert_eq!(
        serde_json::from_slice::<SetupReport>(&encoded).unwrap(),
        report
    );
}

#[test]
fn mismatched_and_contradictory_completion_is_rejected() {
    let request = request();
    for mutate in [
        |r: &mut SetupReport| r.id.push('x'),
        |r: &mut SetupReport| r.protocol += 1,
        |r: &mut SetupReport| r.machine = None,
        |r: &mut SetupReport| r.effects = WriteState::NotStarted,
        |r: &mut SetupReport| r.result = None,
        |r: &mut SetupReport| r.statuses[0] = Status::Running,
        |r: &mut SetupReport| r.result.as_mut().unwrap().outcome = Outcome::Cancelled,
        |r: &mut SetupReport| r.result.as_mut().unwrap().persistence = Persistence::NotSaved,
        |r: &mut SetupReport| {
            r.result.as_mut().unwrap().persistence = Persistence::Saved {
                contents: "last_checked='false baseline'".into(),
            }
        },
        |r: &mut SetupReport| {
            r.statuses[0] = Status::Failed;
            r.result.as_mut().unwrap().statuses[0] = Status::Failed;
        },
    ] {
        let mut report = succeeded(&request);
        mutate(&mut report);
        assert!(report.check(&request).is_err(), "accepted {report:?}");
    }
}

#[test]
fn stop_requires_matching_operation_and_only_apply_accepts_effects() {
    let mut request = request();
    let control = StopAfterItem {
        protocol: SETUP_PROTOCOL,
        id: request.id.clone(),
    };
    control.check(&request).unwrap();
    request.id.push('x');
    assert!(control.check(&request).is_err());
    request.action = SetupAction::Context;
    let mut report = SetupReport::initial(&request);
    report.effects = WriteState::MayHaveBegun;
    assert!(report.check(&request).is_err());
}

#[test]
fn bounded_atomic_reports_leave_previous_contents_on_oversize() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("report.json");
    let request = request();
    let report = succeeded(&request);
    write_atomic(&path, &report).unwrap();
    let previous = std::fs::read(&path).unwrap();
    let mut huge = report.clone();
    huge.message = Some("x".repeat(IPC_LIMIT as usize));
    assert!(write_atomic(&path, &huge).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), previous);
    std::fs::write(&path, vec![b' '; IPC_LIMIT as usize + 1]).unwrap();
    assert!(read_json::<SetupReport>(&path).is_err());
}

#[cfg(windows)]
#[test]
fn report_replacement_tolerates_a_brief_windows_reader_lock() {
    use std::os::windows::fs::OpenOptionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("report.json");
    write_atomic(&path, &1).unwrap();
    let reader = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(75));
        drop(reader);
    });
    write_atomic(&path, &2).unwrap();
    release.join().unwrap();
    assert_eq!(read_json::<i32>(&path).unwrap(), 2);
}
