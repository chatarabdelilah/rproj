use super::*;
use rproj_core::operations::{Cancellation, WorkbenchClient};
use std::{path::PathBuf, process::Command};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_SAVED_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_SAVED_IPC").unwrap());
    let result = job_at(&ipc, &root.join("setups"), &root.join("config.toml"));
    if fs::read_to_string(root.join("mode")).unwrap() == "fail-after" && result.is_ok() {
        // A real replacement completed, but the caller must not trust log text.
        fs::remove_file(ipc.join("report.json")).unwrap();
    }
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
            .prefix("rproj saved 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::create_dir(root.join("setups")).unwrap();
        fs::write(
            root.join("setups/Example.toml"),
            "# preserved\npackage_workflow='none'\nteam='日本語'\n",
        )
        .unwrap();
        fs::write(root.join("mode"), "success").unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../../../tests/fixtures/saved_setups_bridge.rs"),
        )
        .unwrap();
        let worker = root.join("worker.exe");
        let result = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(&worker)
            .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
            .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
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
    fn loaded(&self) -> LoadedSetup {
        self.client
            .load_saved_setup("Example".into(), &Cancellation::default(), |_| {})
            .unwrap()
    }
    fn prepare(&self, action: SetupAction) -> PreparedSetup {
        self.client
            .prepare_saved_setup(
                self.loaded().source,
                action,
                &Cancellation::default(),
                |_| {},
            )
            .unwrap()
    }
    fn save_review(&self) -> ManagementReview {
        let loaded = self.loaded();
        let mut graph = loaded.graph.unwrap();
        graph.capabilities.insert("format".into(), "stylua".into());
        self.prepare(SetupAction::Save(graph)).review
    }
}

#[test]
fn owned_worker_browse_prepare_save_rename_duplicate_delete_and_reuse() {
    let f = Fixture::new();
    f.client.saved_setups_support().unwrap();
    let listing = f
        .client
        .saved_setups(&Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(listing.entries.len(), 1);
    assert!(listing.environment_problem.is_some());
    let original = f.loaded();
    let review = f.save_review();
    assert_eq!(f.loaded(), original);
    let saved = f.client.manage_saved_setup(review.clone(), |_| {}).unwrap();
    assert_eq!(
        (saved.status, saved.mutation),
        (ManagementStatus::Succeeded, MutationState::Completed)
    );
    let saved = saved.saved.unwrap();
    assert_eq!(
        saved.bytes,
        review.document.unwrap().replacement.unwrap().as_bytes()
    );
    let reused = f
        .client
        .reuse_saved_setup(saved.clone(), &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(reused.source, saved);
    let dup = f.prepare(SetupAction::Duplicate("Copy 世界".into())).review;
    assert_eq!(
        f.client.manage_saved_setup(dup, |_| {}).unwrap().status,
        ManagementStatus::Succeeded
    );
    let rename = f.prepare(SetupAction::Rename("Renamed".into())).review;
    assert_eq!(
        f.client.manage_saved_setup(rename, |_| {}).unwrap().status,
        ManagementStatus::Succeeded
    );
    assert!(!f.root.join("setups/Example.toml").exists());
    let renamed = f
        .client
        .load_saved_setup("Renamed".into(), &Cancellation::default(), |_| {})
        .unwrap();
    let delete = f
        .client
        .prepare_saved_setup(
            renamed.source,
            SetupAction::Delete,
            &Cancellation::default(),
            |_| {},
        )
        .unwrap();
    assert_eq!(
        f.client
            .manage_saved_setup(delete.review, |_| {})
            .unwrap()
            .status,
        ManagementStatus::Succeeded
    );
    assert!(!f.root.join("setups/Renamed.toml").exists());
    assert!(!f.root.join("config.toml").exists());
}

#[test]
fn missing_corrupt_mismatched_and_contradictory_reports_never_confirm_success() {
    let f = Fixture::new();
    for mode in [
        "missing-report",
        "malformed-report",
        "mismatch-report",
        "exit-disagrees",
    ] {
        f.mode("success");
        let review = f.save_review();
        f.mode(mode);
        let outcome = f.client.manage_saved_setup(review.clone(), |_| {}).unwrap();
        assert_eq!(
            (outcome.status, outcome.mutation),
            (ManagementStatus::Uncertain, MutationState::Unknown),
            "{mode}"
        );
        assert!(outcome.saved.is_none());
    }
}

#[test]
fn failed_start_old_worker_and_stale_sources_remain_recoverable() {
    let f = Fixture::new();
    let review = f.save_review();
    fs::write(
        f.root.join("setups/Example.toml"),
        "externally_changed=true",
    )
    .unwrap();
    let result = f.client.manage_saved_setup(review.clone(), |_| {}).unwrap();
    assert_eq!(
        (result.status, result.mutation),
        (ManagementStatus::Failed, MutationState::NotStarted)
    );
    f.mode("old-worker");
    assert!(f.client.saved_setups_support().is_err());
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let result = f.client.manage_saved_setup(review, |_| {}).unwrap();
    assert_eq!(
        (result.status, result.mutation),
        (ManagementStatus::Failed, MutationState::NotStarted)
    );
}

#[test]
fn completion_loss_after_real_write_requires_explicit_reload() {
    let f = Fixture::new();
    let review = f.save_review();
    f.mode("fail-after");
    let result = f.client.manage_saved_setup(review.clone(), |_| {}).unwrap();
    assert_eq!(result.status, ManagementStatus::Uncertain);
    assert!(result.saved.is_none());
    assert_eq!(
        fs::read(f.root.join("setups/Example.toml")).unwrap(),
        review.document.unwrap().replacement.unwrap().as_bytes()
    );
}

#[test]
fn owned_completion_drains_output_and_terminates_descendants() {
    let f = Fixture::new();
    let review = f.save_review();
    f.mode("descendant");
    let start = std::time::Instant::now();
    assert_eq!(
        f.client.manage_saved_setup(review, |_| {}).unwrap().status,
        ManagementStatus::Succeeded
    );
    assert!(start.elapsed() < std::time::Duration::from_secs(15));
    let pid = fs::read_to_string(f.root.join("descendant.pid")).unwrap();
    let listed = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&listed.stdout).contains("worker.exe"));
}

#[test]
fn oversized_requests_and_pre_cancelled_reads_do_not_spawn_mutations() {
    let f = Fixture::new();
    let review = f.save_review();
    let token = Cancellation::default();
    token.cancel();
    assert!(
        f.client
            .load_saved_setup("Example".into(), &token, |_| {})
            .is_err()
    );
    let mut too_large = review;
    too_large.source.bytes = vec![b'x'; IPC_LIMIT as usize];
    let result = f.client.manage_saved_setup(too_large, |_| {}).unwrap();
    assert_eq!(
        (result.status, result.mutation),
        (ManagementStatus::Failed, MutationState::NotStarted)
    );
    assert!(result.message.unwrap().contains("2 MiB"));
    assert_eq!(fs::read_dir(f.root.join("setups")).unwrap().count(), 1);
}

#[test]
fn saved_setup_reads_and_mutations_share_the_serial_operation_guard() {
    let f = Fixture::new();
    let review = f.save_review();
    let (started, observed) = std::sync::mpsc::channel();
    let gate = std::sync::Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    let worker_gate = gate.clone();
    std::thread::scope(|scope| {
        let task = scope.spawn(|| {
            f.client.manage_saved_setup(review, move |event| {
                if matches!(event, rproj_core::operations::OperationEvent::Started) {
                    started.send(()).unwrap();
                    let (lock, wake) = &*worker_gate;
                    let guard = lock.lock().unwrap();
                    drop(wake.wait_while(guard, |ready| !*ready).unwrap());
                }
            })
        });
        observed
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let error = f
            .client
            .saved_setups(&Cancellation::default(), |_| {})
            .unwrap_err();
        assert!(error.to_string().contains("Another operation"));
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        assert_eq!(
            task.join().unwrap().unwrap().status,
            ManagementStatus::Succeeded
        );
    });
}
