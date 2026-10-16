use super::*;
use rproj_core::{
    creation::PackageWorkflow,
    operations::{Cancellation, OperationEvent, WorkbenchClient},
};
use std::{
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_CREATION_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let storage = Storage {
        config: root.join("machine.toml"),
        template: root.join("template.json"),
        setups: root.join("setups"),
    };
    let result = crate::background_watch::with_test_root(&root.join("watch"), || -> Result<()> {
        let action = std::env::var("RPROJ_TEST_CREATION_ACTION")?;
        if action == "catalog" {
            fs::write(
                root.join("catalog.json"),
                serde_json::to_vec(&crate::workbench::catalog_entries()?)?,
            )?;
            return Ok(());
        }
        if action == "status" {
            fs::write(
                root.join("status.json"),
                serde_json::to_vec(&rproj_core::operations::WatchStatus {
                    supported: false,
                    project: None,
                    state: rproj_core::operations::WatchState::Stopped,
                    message: "Isolated creation fixture; real Watch is unchanged.".into(),
                })?,
            )?;
            return Ok(());
        }
        if action == "context" {
            fs::write(
                root.join("context.json"),
                serde_json::to_vec(&context_at(&storage)?)?,
            )?;
            return Ok(());
        }
        let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_CREATION_IPC").unwrap());
        if fs::read_to_string(root.join("mode"))? == "cancel-before" {
            let request: CreationRequest = read_json(&ipc.join("request.json"))?;
            write_atomic(
                &ipc.join("report.json"),
                &CreationReport::initial(request.id),
            )?;
            fs::write(root.join("ready"), "before claim")?;
            thread::sleep(Duration::from_secs(60));
        }
        job_at(&ipc, &storage)
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
            .prefix("rproj creation 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("tools")).unwrap();
        fs::create_dir_all(root.join("watch")).unwrap();
        for name in [
            "start.lock",
            "owner.lock",
            "state.lock",
            "state-writer.lock",
        ] {
            fs::write(root.join("watch").join(name), "").unwrap();
        }
        fs::write(root.join("watch/watch.log"), "untouched watch log").unwrap();
        fs::write(root.join("mode"), "success").unwrap();
        let config = GlobalConfig {
            roblox_projects_root: Some(root.join("projects")),
            last_checked: Some("fixture".into()),
            ..Default::default()
        };
        fs::write(root.join("machine.toml"), toml::to_string(&config).unwrap()).unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../tests/fixtures/creation_bridge.rs"),
        )
        .unwrap();
        let worker = root.join("worker.exe");
        let compiled = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
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
        fs::copy(&worker, root.join("tools/git.exe")).unwrap();
        let client = WorkbenchClient::new(worker).unwrap();
        Self {
            _directory: directory,
            root,
            client,
        }
    }
    fn plan(&self, name: &str) -> PreparedCreation {
        let mut draft = CreationDraft::new(name.into());
        draft.set_workflow(PackageWorkflow::None);
        self.prepare(draft)
    }
    fn prepare(&self, draft: CreationDraft) -> PreparedCreation {
        let context = self.client.creation_context().unwrap();
        match self
            .client
            .prepare_creation(
                PrepareCreation {
                    draft,
                    sources: context.sources(None).unwrap(),
                },
                &Cancellation::default(),
                |_| {},
            )
            .unwrap()
        {
            PreparationOutcome::Prepared(plan) => *plan,
            PreparationOutcome::Cancelled => panic!("unexpected preparation cancellation"),
        }
    }
}

#[test]
fn named_setup_saves_to_captured_storage_and_late_collisions_preserve_existing_bytes() {
    let fixture = Fixture::new();
    let mut draft = CreationDraft::new("Saved project".into());
    draft.set_workflow(PackageWorkflow::None);
    draft.save_setup = Some("Fixture setup".into());
    draft.graph.dropped.push("rproj.toml".into());
    let plan = fixture.prepare(draft.clone());
    let outcome = fixture
        .client
        .create_project(plan, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(outcome.status, CreationStatus::Succeeded);
    let saved = fixture.root.join("setups/Fixture setup.toml");
    let graph: crate::graph::ProjectGraph =
        toml::from_str(&fs::read_to_string(saved).unwrap()).unwrap();
    assert_eq!(graph, draft.graph);
    assert!(!outcome.destination.join("rproj.toml").exists());
    let context = fixture.client.creation_context().unwrap();
    assert!(
        context
            .setups
            .iter()
            .any(|s| s.name == "Fixture setup" && s.graph.as_ref() == Some(&graph))
    );
    draft.name = "Setup race project".into();
    draft.save_setup = Some("Race setup".into());
    let plan = fixture.prepare(draft);
    fs::write(fixture.root.join("mode"), "save-race").unwrap();
    let outcome = fixture
        .client
        .create_project(plan, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(outcome.status, CreationStatus::Failed);
    assert_eq!(outcome.claim, ClaimState::Claimed);
    assert_eq!(
        fs::read(fixture.root.join("setups/Race setup.toml")).unwrap(),
        b"preserved concurrent setup"
    );
}

#[test]
fn owned_creation_executes_the_real_adapter_and_refuses_reuse_or_changed_sources() {
    let fixture = Fixture::new();
    let config_before = fs::read(fixture.root.join("machine.toml")).unwrap();
    let watch_before = crate::test_common::project_snapshot(&fixture.root.join("watch"));
    let plan = fixture.plan("Created 日本語 project");
    assert!(!plan.plan.destination.exists());
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let outcome = fixture
        .client
        .create_project(plan.clone(), &Cancellation::default(), move |event| {
            if let OperationEvent::Output { text, .. } = event {
                captured.lock().unwrap().push(text);
            }
        })
        .unwrap();
    assert_eq!(outcome.status, CreationStatus::Succeeded);
    assert_eq!(outcome.claim, ClaimState::Claimed);
    assert!(plan.plan.destination.join("default.project.json").is_file());
    assert!(plan.plan.destination.join("src").is_dir());
    assert!(events.lock().unwrap().join("").contains("is ready"));
    let before = crate::test_common::project_snapshot(&plan.plan.destination);
    let reused = fixture
        .client
        .create_project(plan, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(reused.status, CreationStatus::Failed);
    assert_eq!(reused.claim, ClaimState::NotClaimed);
    assert_eq!(
        crate::test_common::project_snapshot(&reused.destination),
        before
    );
    let plan = fixture.plan("Stale");
    fs::write(fixture.root.join("template.json"), "{}").unwrap();
    let stale = fixture
        .client
        .create_project(plan, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(stale.status, CreationStatus::Failed);
    assert!(!stale.destination.exists());
    assert_eq!(
        fs::read(fixture.root.join("machine.toml")).unwrap(),
        config_before
    );
    assert_eq!(
        crate::test_common::project_snapshot(&fixture.root.join("watch")),
        watch_before
    );
}

#[test]
fn creation_failure_after_claim_is_partial_and_invalid_reports_never_succeed() {
    let fixture = Fixture::new();
    let plan = fixture.plan("Partial");
    fs::write(fixture.root.join("mode"), "fail").unwrap();
    let failed = fixture
        .client
        .create_project(plan.clone(), &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(failed.status, CreationStatus::Failed);
    assert_eq!(failed.claim, ClaimState::Claimed);
    assert!(failed.destination.is_dir());
    for mode in ["missing-report", "wrong-report", "false-success"] {
        fs::write(fixture.root.join("mode"), mode).unwrap();
        assert!(
            fixture
                .client
                .create_project(plan.clone(), &Cancellation::default(), |_| {})
                .is_err()
        );
    }
    fs::write(fixture.root.join("mode"), "old-worker").unwrap();
    assert!(fixture.client.creation_context().is_err());
}

#[test]
fn already_cancelled_preparation_is_typed_and_starts_no_project() {
    let fixture = Fixture::new();
    let context = fixture.client.creation_context().unwrap();
    let token = Cancellation::default();
    token.cancel();
    let draft = CreationDraft::new("Cancelled before prepare".into());
    assert_eq!(
        fixture
            .client
            .prepare_creation(
                PrepareCreation {
                    draft,
                    sources: context.sources(None).unwrap()
                },
                &token,
                |_| panic!("no worker must start")
            )
            .unwrap(),
        PreparationOutcome::Cancelled
    );
    assert!(!context.environment.projects_root.exists());
}

#[test]
fn creation_cancellation_drains_owned_descendants_before_returning_its_claim_state() {
    let fixture = Fixture::new();
    for mode in ["cancel-before", "cancel-after"] {
        fs::write(fixture.root.join("mode"), "success").unwrap();
        let plan = fixture.plan(mode);
        fs::write(fixture.root.join("mode"), mode).unwrap();
        let _ = fs::remove_file(fixture.root.join("ready"));
        let token = Cancellation::default();
        let request = token.clone();
        let root = &fixture.root;
        let start = Instant::now();
        let outcome = thread::scope(|scope| {
            let waiter = scope.spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                while !root.join("ready").exists() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(20));
                }
                let ready = root.join("ready").exists();
                request.cancel();
                ready
            });
            let outcome = fixture.client.create_project(plan, &token, |_| {});
            assert!(
                waiter.join().unwrap(),
                "fixture did not reach the cancellation boundary"
            );
            outcome.unwrap()
        });
        assert_eq!(outcome.status, CreationStatus::Cancelled);
        assert_eq!(
            outcome.claim,
            if mode == "cancel-before" {
                ClaimState::NotClaimed
            } else {
                ClaimState::Claimed
            }
        );
        assert_eq!(outcome.destination.exists(), mode == "cancel-after");
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "owned descendants retained output pipes"
        );
    }
}
