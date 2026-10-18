use super::*;
use crate::test_common::project_snapshot;
use rproj_core::{
    operations::{Cancellation, WorkbenchClient},
    upgrade::UpgradeReview,
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, Instant},
};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_UPGRADE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let result = crate::background_watch::with_test_root(&root.join("watch"), || -> Result<()> {
        if std::env::var("RPROJ_TEST_UPGRADE_ACTION")? == "context" {
            fs::write(
                root.join("context.json"),
                serde_json::to_vec(&context(&std::env::current_dir()?)?)?,
            )?;
            return Ok(());
        }
        let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_UPGRADE_IPC").unwrap());
        let result = job(&ipc);
        let mode = fs::read_to_string(root.join("mode"))?;
        if mode == "fail-after" && result.is_ok() {
            let path = ipc.join("report.json");
            let mut report: UpgradeReport = read_json(&path)?;
            report.status = Some(UpgradeStatus::Failed);
            report.writes = WriteState::MayHaveBegun;
            report.message =
                Some("Fixture failure after writes; inspect possible partial changes".into());
            write_atomic(&path, &report)?;
            anyhow::bail!("fixture post-write failure");
        }
        if mode == "late-cancel" && result.is_ok() {
            let mut child = Command::new(root.join("worker.exe"))
                .arg("--sleeper")
                .spawn()?;
            fs::write(root.join("ready"), child.id().to_string())?;
            let _ = child.wait();
        }
        result
    });
    std::process::exit(i32::from(crate::internal_result_exit_code(result)));
}

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    client: WorkbenchClient,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("rproj Upgrade 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        let project = root.join("captured project 世界");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(root.join("watch")).unwrap();
        for name in [
            "start.lock",
            "owner.lock",
            "state.lock",
            "state-writer.lock",
        ] {
            fs::write(root.join("watch").join(name), "").unwrap();
        }
        fs::write(root.join("mode"), "success").unwrap();
        fs::write(
            project.join("rproj.toml"),
            "# keep bytes\nmode='expert'\npackage_workflow='none'\n",
        )
        .unwrap();
        fs::write(project.join("default.project.json"), "{}\n").unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../../tests/fixtures/upgrade_bridge.rs"),
        )
        .unwrap();
        let worker = root.join("worker.exe");
        let compiler = Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&worker)
            .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
            .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
            .output()
            .unwrap();
        assert!(
            compiler.status.success(),
            "{}",
            String::from_utf8_lossy(&compiler.stderr)
        );
        let client = WorkbenchClient::new(worker).unwrap();
        Self {
            _directory: directory,
            root,
            project,
            client,
        }
    }
    fn prepared(&self) -> UpgradeReview {
        self.client.upgrade_support().unwrap();
        match self
            .client
            .prepare_upgrade(
                self.client.upgrade_context(&self.project).unwrap(),
                &Cancellation::default(),
                |_| {},
            )
            .unwrap()
        {
            UpgradePreparation::Prepared(review) => *review,
            other => panic!("{other:?}"),
        }
    }
    fn mode(&self, mode: &str) {
        fs::write(self.root.join("mode"), mode).unwrap();
    }
    fn request(&self, review: UpgradeReview) -> tempfile::TempDir {
        let ipc = tempfile::tempdir().unwrap();
        write_atomic(
            &ipc.path().join("request.json"),
            &UpgradeRequest {
                protocol: UPGRADE_PROTOCOL,
                id: "watch-fixture".into(),
                project: self.project.clone(),
                action: UpgradeAction::Apply(review),
            },
        )
        .unwrap();
        ipc
    }
}

#[test]
fn preparation_is_read_only_and_success_is_structured() {
    let f = Fixture::new();
    let before = project_snapshot(&f.project);
    let watch = project_snapshot(&f.root.join("watch"));
    let review = f.prepared();
    assert_eq!(review.files.len(), 2);
    assert_eq!(before, project_snapshot(&f.project));
    assert_eq!(watch, project_snapshot(&f.root.join("watch")));
    let result = f
        .client
        .apply_upgrade(review, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.status, result.writes),
        (UpgradeStatus::Succeeded, WriteState::Completed)
    );
    assert!(f.project.join(".luaurc").exists());
    assert!(f.prepared().files.is_empty());
}

#[test]
fn stale_missing_target_and_tampered_contents_never_start_writes() {
    let f = Fixture::new();
    let reviewed = f.prepared();
    let mut tampered = reviewed.clone();
    tampered.files[0].contents = "unreviewed".into();
    let result = f
        .client
        .apply_upgrade(tampered, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.status, result.writes),
        (UpgradeStatus::Failed, WriteState::NotStarted)
    );
    fs::write(f.project.join(".luaurc"), "custom").unwrap();
    let before = project_snapshot(&f.project);
    let result = f
        .client
        .apply_upgrade(reviewed, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(result.writes, WriteState::NotStarted);
    assert_eq!(before, project_snapshot(&f.project));
    assert!(result.message.unwrap().contains("Refresh review"));
}

#[test]
fn invalid_reports_never_turn_success_logs_into_success() {
    for mode in ["missing-report", "malformed-report", "mismatch-report"] {
        let f = Fixture::new();
        let review = f.prepared();
        f.mode(mode);
        let result = f
            .client
            .apply_upgrade(review, &Cancellation::default(), |_| {})
            .unwrap();
        assert_eq!(
            (result.status, result.writes),
            (UpgradeStatus::Uncertain, WriteState::Unknown)
        );
    }
}

#[test]
fn source_errors_and_unsupported_workers_remain_actionable() {
    let f = Fixture::new();
    f.mode("old-worker");
    assert!(f.client.upgrade_support().is_err());
    f.mode("success");
    fs::write(f.project.join("rproj.toml"), "[").unwrap();
    let before = project_snapshot(&f.project);
    let result = f
        .client
        .prepare_upgrade(
            f.client.upgrade_context(&f.project).unwrap(),
            &Cancellation::default(),
            |_| {},
        )
        .unwrap();
    assert!(matches!(result, UpgradePreparation::Failed(_)));
    assert_eq!(before, project_snapshot(&f.project));
}

#[test]
fn same_project_watch_blocks_writes_and_another_owner_is_preserved() {
    let f = Fixture::new();
    let review = f.prepared();
    let before = project_snapshot(&f.project);
    crate::background_watch::with_unresponsive_fixture(&f.project, || {
        let ipc = f.request(review.clone());
        assert!(job(ipc.path()).is_err());
        let report: UpgradeReport = read_json(&ipc.path().join("report.json")).unwrap();
        assert_eq!(report.writes, WriteState::NotStarted);
        assert_eq!(before, project_snapshot(&f.project));
    });
    let other = tempfile::tempdir().unwrap();
    let other = other.path().canonicalize().unwrap();
    crate::background_watch::with_unresponsive_fixture(&other, || {
        let ipc = f.request(review);
        job(ipc.path()).unwrap();
        assert_eq!(
            crate::background_watch::structured_status()
                .unwrap()
                .project,
            Some(other.clone())
        );
    });
}

#[test]
fn post_write_failure_is_not_success_or_unchanged() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("fail-after");
    let outcome = f
        .client
        .apply_upgrade(review, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (outcome.status, outcome.writes),
        (UpgradeStatus::Failed, WriteState::MayHaveBegun)
    );
    assert!(f.project.join(".luaurc").exists());
}

#[test]
fn cancellation_waits_for_descendants_and_preserves_confirmed_completion() {
    for mode in ["cancel", "late-cancel"] {
        let f = Fixture::new();
        let review = f.prepared();
        f.mode(mode);
        let token = Cancellation::default();
        let cancel = token.clone();
        let client = f.client.clone();
        let task = thread::spawn(move || client.apply_upgrade(review, &token, |_| {}).unwrap());
        let deadline = Instant::now() + Duration::from_secs(15);
        while !f.root.join("ready").exists() {
            assert!(Instant::now() < deadline, "fixture timed out");
            thread::sleep(Duration::from_millis(10));
        }
        let pid = fs::read_to_string(f.root.join("ready")).unwrap();
        cancel.cancel();
        let result = task.join().unwrap();
        assert_eq!(
            result.status,
            if mode == "late-cancel" {
                UpgradeStatus::Succeeded
            } else {
                UpgradeStatus::Cancelled
            }
        );
        if mode == "late-cancel" {
            assert_eq!(result.writes, WriteState::Completed);
        }
        let listed = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&listed.stdout).contains("worker.exe"));
    }
}

#[test]
fn pre_cancelled_and_missing_worker_are_recoverable_without_writes() {
    let f = Fixture::new();
    let review = f.prepared();
    let before = project_snapshot(&f.project);
    let token = Cancellation::default();
    token.cancel();
    assert_eq!(
        f.client
            .apply_upgrade(review.clone(), &token, |_| {})
            .unwrap()
            .status,
        UpgradeStatus::Cancelled
    );
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let result = f
        .client
        .apply_upgrade(review, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.status, result.writes),
        (UpgradeStatus::Failed, WriteState::NotStarted)
    );
    assert_eq!(before, project_snapshot(&f.project));
}

#[test]
fn oversized_preparation_is_an_actionable_read_only_failure() {
    let f = Fixture::new();
    fs::write(
        f.project.join("default.project.json"),
        " ".repeat(IPC_LIMIT as usize),
    )
    .unwrap();
    let before = project_snapshot(&f.project);
    let result = f
        .client
        .prepare_upgrade(
            f.client.upgrade_context(&f.project).unwrap(),
            &Cancellation::default(),
            |_| {},
        )
        .unwrap();
    match result {
        UpgradePreparation::Failed(message) => assert!(message.contains("2 MiB"), "{message}"),
        other => panic!("{other:?}"),
    }
    assert_eq!(before, project_snapshot(&f.project));
}
