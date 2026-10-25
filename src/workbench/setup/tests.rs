use super::*;
use rproj_core::{
    machine_setup::Item,
    operations::{SetupEvent, SetupStop, WorkbenchClient},
};
use std::{fs, process::Command, time::Instant};

fn output(event: SetupEvent) {
    if let SetupEvent::Output(event) = event {
        eprintln!("{event:?}");
    }
}

struct Fake {
    root: PathBuf,
}
impl Fake {
    fn mode(&self) -> String {
        fs::read_to_string(self.root.join("mode")).unwrap()
    }
}
impl Backend for Fake {
    fn context(&self) -> Result<SetupContext> {
        checked::test_capture(
            &self.root.join("config/settings.toml"),
            &self.root.join("projects 世界"),
        )
    }
    fn prepare(&self, context: &SetupContext, selection: &Selection) -> Result<SetupReview> {
        ensure!(
            *context == self.context()?,
            "Captured setup changed; reload explicitly"
        );
        ensure!(
            self.mode() != "prerequisite",
            "Fixture prerequisite is missing"
        );
        checked::prepare(context, selection)
    }
    fn apply(
        &self,
        review: &SetupReview,
        stop: &(dyn Fn() -> bool + Sync),
        emit: Arc<dyn Fn(Event) + Send + Sync>,
        mutation: &mut dyn FnMut() -> Result<()>,
    ) -> Result<SetupResult> {
        ensure!(
            self.prepare(&review.context, &review.selection)? == *review,
            "Changed review"
        );
        checked::test_execute(
            review,
            &self.root.join("setup.lock"),
            stop,
            emit,
            mutation,
            |item, selection, reporter| {
                let mode = self.mode();
                fs::write(self.root.join("installer-started"), item.label())?;
                if mode == "fatal" {
                    anyhow::bail!("fake bootstrap failure");
                }
                if mode == "wait" && *item == Item::Rokit {
                    let deadline = Instant::now() + Duration::from_secs(15);
                    while !self.root.join("release-item").exists() && Instant::now() < deadline {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    ensure!(
                        self.root.join("release-item").exists(),
                        "Fixture active item timed out"
                    );
                    fs::write(self.root.join("active-finished"), "normal completion")?;
                    reporter.message(
                        crate::steps::execution::MessageKind::Detail,
                        "Active item finished after owner disconnection",
                    );
                }
                if *item == Item::ProjectsFolder {
                    fs::create_dir_all(&selection.root)?;
                    if mode == "save-failure" {
                        fs::create_dir_all(review.context.config_path.parent().unwrap())?;
                        fs::write(&review.context.config_path, "last_checked='external'")?;
                    }
                }
                if mode == "warning" && matches!(item, Item::Tool(_)) {
                    anyhow::bail!("fake optional tool failure");
                }
                if mode == "manual" && matches!(item, Item::Tool(_)) {
                    reporter.message(
                        crate::steps::execution::MessageKind::Manual,
                        "Manual fixture step",
                    );
                }
                Ok(())
            },
        )
    }
}

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_SETUP_ROOT") else {
        return;
    };
    let root = PathBuf::from(root).canonicalize().unwrap();
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_SETUP_IPC").unwrap());
    let result = crate::background_watch::with_test_root(&root.join("watch"), || {
        job_with(&ipc, &Fake { root: root.clone() })
    });
    std::process::exit(i32::from(crate::internal_result_exit_code(result)));
}

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    client: WorkbenchClient,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("rproj Setup 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::create_dir(root.join("watch")).unwrap();
        for name in [
            "start.lock",
            "owner.lock",
            "state.lock",
            "state-writer.lock",
        ] {
            fs::write(root.join("watch").join(name), "").unwrap();
        }
        fs::write(root.join("mode"), "success").unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../../tests/fixtures/setup_bridge.rs"),
        )
        .unwrap();
        let worker = root.join("worker.exe");
        let compiled = Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&worker)
            .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
            .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let client = WorkbenchClient::new(worker).unwrap();
        Self {
            _directory: directory,
            root,
            client,
        }
    }
    fn mode(&self, mode: &str) {
        fs::write(self.root.join("mode"), mode).unwrap();
    }
    fn prepared(&self) -> SetupReview {
        self.client.setup_support().unwrap();
        let context = self.client.setup_context(output).unwrap();
        self.client
            .prepare_setup(context.clone(), context.saved.clone(), output)
            .unwrap()
    }
    fn request(&self, action: SetupAction) -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        write_atomic(
            &directory.path().join("request.json"),
            &SetupRequest {
                protocol: SETUP_PROTOCOL,
                id: "fixture".into(),
                action,
            },
        )
        .unwrap();
        directory
    }
}

#[test]
fn read_only_preparation_and_success_save_exact_baseline() {
    let f = Fixture::new();
    let review = f.prepared();
    assert!(!review.context.config_path.exists());
    assert!(!review.selection.root.exists());
    assert!(!f.root.join("installer-started").exists());
    let report = f
        .client
        .apply_setup(review.clone(), &SetupStop::default(), output)
        .unwrap();
    assert_eq!(
        (report.status, report.effects),
        (Some(SetupStatus::Succeeded), WriteState::Completed)
    );
    let Persistence::Saved { contents } = report.result.unwrap().persistence else {
        panic!("not saved")
    };
    assert_eq!(
        fs::read_to_string(review.context.config_path).unwrap(),
        contents
    );
}

#[test]
fn stop_is_acknowledged_during_active_item_without_terminating_it_or_saving() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("wait");
    let stop = SetupStop::default();
    let send = stop.clone();
    let root = f.root.clone();
    let report = f
        .client
        .apply_setup(review.clone(), &stop, move |event| {
            if let SetupEvent::Progress(report) = event {
                if report.active == Some(0) {
                    send.request();
                }
                if report.stop_acknowledged && report.active == Some(0) {
                    assert!(!root.join("active-finished").exists());
                    fs::write(root.join("release-item"), "finish normally").unwrap();
                }
            }
        })
        .unwrap();
    assert_eq!(report.status, Some(SetupStatus::Stopped));
    assert!(report.stop_acknowledged && f.root.join("active-finished").exists());
    assert_eq!(report.statuses, vec![Status::Installed, Status::Skipped]);
    assert!(!review.context.config_path.exists());
    assert!(!review.selection.root.exists());
}

#[test]
fn stale_prerequisite_and_tampered_reviews_do_not_install() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("prerequisite");
    let report = f
        .client
        .apply_setup(review.clone(), &SetupStop::default(), |_| {})
        .unwrap();
    assert_eq!(
        (report.status, report.effects),
        (Some(SetupStatus::Failed), WriteState::NotStarted)
    );
    f.mode("success");
    let mut tampered = review.clone();
    tampered.items.reverse();
    assert!(
        f.client
            .apply_setup(tampered, &SetupStop::default(), |_| {})
            .is_err()
    );
    fs::create_dir(&review.selection.root).unwrap();
    let report = f
        .client
        .apply_setup(review, &SetupStop::default(), |_| {})
        .unwrap();
    assert_eq!(report.effects, WriteState::NotStarted);
    assert!(!f.root.join("installer-started").exists());
}

#[test]
fn fatal_optional_and_persistence_failures_remain_distinct() {
    for mode in ["fatal", "warning", "manual", "save-failure"] {
        let f = Fixture::new();
        let mut review = f.prepared();
        review.selection.tools.insert("rojo".into());
        review = SetupReview::new(review.context, review.selection).unwrap();
        f.mode(mode);
        let report = f
            .client
            .apply_setup(review, &SetupStop::default(), |_| {})
            .unwrap();
        let result = report.result.unwrap();
        match mode {
            "fatal" => {
                assert_eq!(report.status, Some(SetupStatus::Failed));
                assert_eq!(result.persistence, Persistence::NotSaved);
            }
            "save-failure" => {
                assert_eq!(report.status, Some(SetupStatus::Failed));
                assert!(matches!(result.persistence, Persistence::Failed { .. }));
            }
            _ => {
                assert_eq!(report.status, Some(SetupStatus::Succeeded));
                assert_eq!(result.outcome, Outcome::Warnings);
                assert!(matches!(result.persistence, Persistence::Saved { .. }));
            }
        }
    }
}

#[test]
fn reports_never_trust_logs_and_owned_descendants_finish_normally() {
    for mode in [
        "missing-report",
        "malformed-report",
        "mismatch-report",
        "exit-disagrees",
        "descendant",
    ] {
        let f = Fixture::new();
        let review = f.prepared();
        f.mode(mode);
        let report = f
            .client
            .apply_setup(review, &SetupStop::default(), |_| {})
            .unwrap();
        if mode == "descendant" {
            assert_eq!(report.status, Some(SetupStatus::Succeeded));
            assert!(
                f.root.join("descendant-finished").exists(),
                "owned descendant was killed or completion returned too soon"
            );
        } else {
            assert_eq!(
                (report.status, report.effects),
                (Some(SetupStatus::Uncertain), WriteState::Unknown)
            );
        }
    }
}

#[test]
fn any_watch_blocks_apply_but_not_preparation() {
    let f = Fixture::new();
    let review = f.prepared();
    let project = tempfile::tempdir().unwrap();
    crate::background_watch::with_unresponsive_fixture(project.path(), || {
        let ipc = f.request(SetupAction::Apply(review.clone()));
        assert!(
            job_with(
                ipc.path(),
                &Fake {
                    root: f.root.clone()
                }
            )
            .is_err()
        );
        let report: SetupReport = read_json(&ipc.path().join("report.json")).unwrap();
        assert_eq!(report.effects, WriteState::NotStarted);
        assert!(!f.root.join("installer-started").exists());
        Fake {
            root: f.root.clone(),
        }
        .prepare(&review.context, &review.selection)
        .unwrap();
    });
}

#[test]
fn unsupported_worker_prespawn_failure_and_oversized_request_are_recoverable() {
    let f = Fixture::new();
    let review = f.prepared();
    let mut huge = review.clone();
    huge.context.original = Some("x".repeat(IPC_LIMIT as usize));
    assert!(
        f.client
            .apply_setup(huge, &SetupStop::default(), |_| {})
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
    f.mode("old-worker");
    assert!(f.client.setup_support().is_err());
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let error = f
        .client
        .apply_setup(review, &SetupStop::default(), |_| {})
        .unwrap_err();
    assert!(!rproj_core::operations::owned_teardown_unconfirmed(&error));
}

#[test]
fn a_stop_after_confirmed_save_cannot_rewrite_success() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("late-stop");
    let stop = SetupStop::default();
    let send = stop.clone();
    let report = f
        .client
        .apply_setup(review, &stop, move |event| {
            if let SetupEvent::Output(rproj_core::operations::OperationEvent::Output {
                text, ..
            }) = event
                && text.contains("fixture saved before late Stop")
            {
                send.request();
            }
        })
        .unwrap();
    assert!(stop.requested());
    assert_eq!(report.status, Some(SetupStatus::Succeeded));
    assert!(f.root.join("descendant-finished").exists());
}

#[test]
fn pre_requested_stop_skips_every_item_without_saving() {
    let f = Fixture::new();
    let review = f.prepared();
    let stop = SetupStop::default();
    stop.request();
    let report = f.client.apply_setup(review.clone(), &stop, |_| {}).unwrap();
    assert_eq!(
        (report.status, report.effects),
        (Some(SetupStatus::Stopped), WriteState::NotStarted)
    );
    assert!(!f.root.join("installer-started").exists());
    assert!(!review.context.config_path.exists());
}

#[test]
fn production_rejects_foreign_configuration_before_probes_or_installers() {
    let f = Fixture::new();
    let review = f.prepared();
    assert!(Machine.prepare(&review.context, &review.selection).is_err());
    assert!(
        Machine
            .apply(&review, &|| false, Arc::new(|_| {}), &mut || panic!(
                "foreign mutation"
            ))
            .is_err()
    );
    assert!(!f.root.join("installer-started").exists());
}

#[test]
fn malformed_and_oversized_configurations_refuse_read_only_context() {
    let f = Fixture::new();
    let path = f.root.join("config/settings.toml");
    fs::create_dir(path.parent().unwrap()).unwrap();
    for contents in ["bad = [".to_owned(), "#".repeat(IPC_LIMIT as usize + 1)] {
        fs::write(&path, &contents).unwrap();
        assert!(f.client.setup_context(output).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), contents);
        assert!(!f.root.join("installer-started").exists());
    }
}

#[test]
fn failed_stop_delivery_does_not_kill_active_installer_or_claim_teardown() {
    let f = Fixture::new();
    let review = f.prepared();
    fs::remove_file(f.root.join("bridge-finished")).unwrap();
    f.mode("wait");
    let stop = SetupStop::default();
    let send = stop.clone();
    let directory = Arc::new(Mutex::new(None));
    let captured = directory.clone();
    let started = Instant::now();
    let error = f
        .client
        .apply_setup(review, &stop, move |event| {
            if let SetupEvent::Progress(report) = event
                && report.active == Some(0)
            {
                let mut captured = captured.lock().unwrap();
                if captured.is_none() {
                    let ipc = std::env::temp_dir().join(&report.id);
                    assert!(report.id.starts_with("rproj-machine-setup-"));
                    fs::create_dir(ipc.join("stop.json")).unwrap();
                    *captured = Some(ipc);
                    send.request();
                }
            }
        })
        .unwrap_err();
    assert!(rproj_core::operations::owned_teardown_unconfirmed(&error));
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "Runtime shutdown waited for the active installer"
    );
    assert!(!f.root.join("active-finished").exists());
    // Explicitly release the finite fixture after proving the runner did not kill it.
    fs::write(f.root.join("release-item"), "finish normally").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f.root.join("bridge-finished").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(f.root.join("active-finished").exists());
    assert!(f.root.join("bridge-finished").exists());
    let ipc = directory.lock().unwrap().take().unwrap();
    assert!(
        ipc.join("request.json").exists(),
        "unconfirmed operation storage was removed early"
    );
    let report: SetupReport = read_json(&ipc.join("report.json")).unwrap();
    assert_eq!(
        report.status,
        Some(SetupStatus::Succeeded),
        "Disconnected logging interrupted the worker"
    );
    fs::remove_dir_all(ipc).unwrap();
}
