use std::{fs, path::PathBuf, process::Command};

use rproj_core::operations::WorkbenchClient;

fn cli() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rproj"))
}

#[test]
fn matching_cli_worker_serves_the_shared_catalog() {
    let client = WorkbenchClient::new(cli()).unwrap();
    let catalog = client.catalog().unwrap();
    let profile = catalog
        .iter()
        .find(|entry| entry.package == "lm-loleris/profilestore")
        .unwrap();
    assert_eq!(profile.key, "profilestore");
    assert!(profile.docs_url.starts_with("https://"));
    assert!(profile.example.contains("ServerScriptService"));
    assert!(!profile.caveats.is_empty());
}

#[test]
fn projects_context_is_fresh_versioned_and_requires_an_absolute_path() {
    use rproj_core::project_discovery::PROJECTS_CONTEXT_VERSION;
    let fixture = tempfile::tempdir().unwrap();
    let script = fixture.path().join("worker.rs");
    fs::write(&script, r#"
        fn main() {
            let args: Vec<_> = std::env::args_os().skip(1).collect();
            if args == ["--version"] { println!("rproj {}", env!("FIXTURE_VERSION")); return; }
            assert_eq!(args, ["--rproj-workbench", "projects-context"]);
            print!("{}", std::fs::read_to_string(std::env::current_exe().unwrap().with_extension("reply")).unwrap());
        }"#).unwrap();
    let worker = fixture
        .path()
        .join(format!("worker{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new("rustc")
        .arg(&script)
        .arg("-o")
        .arg(&worker)
        .env("FIXTURE_VERSION", env!("CARGO_PKG_VERSION"))
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let client = WorkbenchClient::new(worker.clone()).unwrap();
    let reply = worker.with_extension("reply");
    for name in ["First", "世界 Second"] {
        let path = fixture.path().join(name);
        fs::write(
            &reply,
            serde_json::to_vec(
                &serde_json::json!({"protocol":PROJECTS_CONTEXT_VERSION, "root":path}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(client.projects_root().unwrap(), path);
        assert!(!path.exists());
    }
    for response in [
        serde_json::json!({"protocol":PROJECTS_CONTEXT_VERSION,"root":"relative"}),
        serde_json::json!({"protocol":999, "root":fixture.path()}),
    ] {
        fs::write(&reply, response.to_string()).unwrap();
        assert!(client.projects_root().is_err());
    }
}

#[cfg(windows)]
#[test]
fn real_bundled_worker_prepares_upgrade_read_only() {
    use rproj_core::{operations::Cancellation, upgrade::protocol::UpgradePreparation};
    let project = tempfile::Builder::new()
        .prefix("rproj real Upgrade 世界 ")
        .tempdir()
        .unwrap();
    let record = "mode='expert'\npackage_workflow='none'\n";
    fs::write(project.path().join("rproj.toml"), record).unwrap();
    fs::write(project.path().join("default.project.json"), "{}\n").unwrap();
    let client = WorkbenchClient::new(cli()).unwrap();
    client.upgrade_support().unwrap();
    let context = client.upgrade_context(project.path()).unwrap();
    let prepared = client
        .prepare_upgrade(context, &Cancellation::default(), |_| {})
        .unwrap();
    let UpgradePreparation::Prepared(review) = prepared else {
        panic!("Expected a real worker review: {prepared:?}")
    };
    assert_eq!(review.files.len(), 2);
    assert!(review.diff(0).is_some());
    assert_eq!(
        fs::read_to_string(project.path().join("rproj.toml")).unwrap(),
        record
    );
    assert_eq!(
        fs::read_to_string(project.path().join("default.project.json")).unwrap(),
        "{}\n"
    );
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 2);
}

#[cfg(windows)]
#[test]
fn real_bundled_settings_worker_prepares_without_record_setup_or_executable() {
    use rproj_core::operations::Cancellation;
    let project = tempfile::Builder::new()
        .prefix("rproj real Settings 世界 ")
        .tempdir()
        .unwrap();
    let client = WorkbenchClient::new(cli()).unwrap();
    client.settings_support().unwrap();
    let loaded = client
        .settings_context(project.path(), "stylua", &Cancellation::default(), |_| {})
        .unwrap();
    let mut draft = loaded.draft;
    let target = loaded.context.target().unwrap();
    draft.set(0, target.settings[0].kind.default_value());
    let review = client
        .prepare_settings(loaded.context, draft, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
    assert!(!review.diff().unwrap().is_empty());
    assert!(review.contents.is_some());
    // Save is covered by the owned bridge with isolated Watch storage. The real
    // bundled CLI has no test-root override, so this boundary check stays read-only.
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
}

#[test]
fn differently_versioned_worker_is_rejected_before_operations() {
    let fixture = tempfile::Builder::new()
        .prefix("rproj worker version ")
        .tempdir()
        .unwrap();
    let source = fixture.path().join("worker.rs");
    fs::write(&source, "fn main() { println!(\"rproj 9999.0.0\"); }").unwrap();
    let worker = fixture
        .path()
        .join(format!("worker{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new("rustc")
        .arg(&source)
        .arg("-o")
        .arg(&worker)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let error = WorkbenchClient::new(worker).err().unwrap();
    assert!(error.to_string().contains("versions differ"), "{error:#}");
}

#[cfg(windows)]
#[test]
fn worker_check_reports_missing_prerequisite_through_owned_operation_events() {
    use std::sync::{Arc, Mutex};

    use rproj_core::operations::{Cancellation, Operation, OperationEvent, OperationOutcome};

    let project = tempfile::Builder::new()
        .prefix("rproj worker project ")
        .tempdir()
        .unwrap();
    let manifest = project.path().join("default.project.json");
    let original = br#"{"name":"Worker fixture","tree":{}}"#;
    fs::write(&manifest, original).unwrap();
    let client = WorkbenchClient::new(cli()).unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let outcome = client
        .run(
            Operation::Check,
            project.path(),
            &Cancellation::default(),
            move |event| captured.lock().unwrap().push(event),
        )
        .unwrap();
    assert_eq!(outcome, OperationOutcome::Failed { code: Some(1) });
    let events = events.lock().unwrap();
    assert!(matches!(events.first(), Some(OperationEvent::Started)));
    assert!(matches!(
        events.last(),
        Some(OperationEvent::Finished {
            outcome: OperationOutcome::Failed { code: Some(1) }
        })
    ));
    let stderr: String = events
        .iter()
        .filter_map(|event| match event {
            OperationEvent::Output { stderr: true, text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(stderr.contains("No .lute/check.luau"), "{stderr}");
    assert_eq!(fs::read(&manifest).unwrap(), original);
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 1);
}
