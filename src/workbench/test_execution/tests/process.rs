use super::*;
use rproj_core::{
    operations::{Cancellation, OperationEvent, WorkbenchClient},
    test_execution::TestContext,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, OnceLock},
};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_ARGUMENT_ROOT") else {
        return;
    };
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_ARGUMENT_IPC").unwrap());
    let result =
        crate::background_watch::with_test_root(&PathBuf::from(root).join("watch"), || job(&ipc));
    std::process::exit(i32::from(crate::internal_result_exit_code(result)));
}

fn binaries() -> &'static Path {
    static BINARIES: OnceLock<tempfile::TempDir> = OnceLock::new();
    BINARIES
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            for (name, source) in [
                (
                    "worker",
                    include_str!("../../../../tests/fixtures/test_arguments_bridge.rs"),
                ),
                (
                    "lute",
                    include_str!("../../../../tests/fixtures/test_arguments_runner.rs"),
                ),
            ] {
                let path = root.path().join(format!("{name}.rs"));
                fs::write(&path, source).unwrap();
                let output = Command::new("rustc")
                    .arg("--edition=2024")
                    .arg(path)
                    .arg("-o")
                    .arg(root.path().join(format!("{name}.exe")))
                    .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
                    .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            root
        })
        .path()
}

struct Fixture {
    _root: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    client: WorkbenchClient,
}
impl Fixture {
    fn new() -> Self {
        let (directory, _, request) = fixture();
        let project = request.project;
        let root_dir = tempfile::Builder::new()
            .prefix("test worker 保存 ")
            .tempdir()
            .unwrap();
        let root = root_dir.path().canonicalize().unwrap();
        // Keep the project under the worker's owned fixture directory.
        let destination = root.join("project 日本語 space");
        fs::create_dir(&destination).unwrap();
        for name in ["rproj.toml", "default.project.json"] {
            fs::copy(project.join(name), destination.join(name)).unwrap();
        }
        drop(directory);
        fs::create_dir(root.join("tools")).unwrap();
        fs::copy(binaries().join("worker.exe"), root.join("worker.exe")).unwrap();
        fs::copy(binaries().join("lute.exe"), root.join("tools/lute.exe")).unwrap();
        fs::write(root.join("mode"), "success").unwrap();
        let client = WorkbenchClient::new(root.join("worker.exe")).unwrap();
        Self {
            _root: root_dir,
            root,
            project: destination,
            client,
        }
    }
    fn mode(&self, mode: &str) {
        fs::write(self.root.join("mode"), mode).unwrap();
    }
    fn context(&self) -> TestContext {
        self.client
            .test_context(&self.project, &Cancellation::default(), |_| {})
            .unwrap()
    }
    fn assert_reaped(&self, executable: &str) {
        let pid = fs::read_to_string(self.root.join("descendant.pid")).unwrap();
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains(executable));
    }
}

#[test]
fn exact_arguments_and_runner_exit_survive_owned_processes() {
    let f = Fixture::new();
    let context = f.context();
    let arguments = vec![
        "".into(),
        "保存 space".into(),
        "\"quoted\"\\".into(),
        "CR\rLF\nTAB\t".into(),
        "$(no shell); &".into(),
    ];
    let output = Arc::new(Mutex::new(Vec::new()));
    let collected = output.clone();
    let result = f
        .client
        .run_custom_tests(
            context.clone(),
            arguments.clone(),
            &Cancellation::default(),
            move |event| collected.lock().unwrap().push(format!("{event:?}")),
        )
        .unwrap();
    assert_eq!(
        (result.status, result.accepted, result.runner_exit),
        (Status::Succeeded, Some(true), Some(0))
    );
    let mut expected = Vec::new();
    for value in std::iter::once("test".to_owned()).chain(arguments) {
        expected.extend_from_slice(&(value.len() as u64).to_le_bytes());
        expected.extend_from_slice(value.as_bytes());
    }
    assert_eq!(fs::read(f.root.join("arguments.bin")).unwrap(), expected);
    let output = output.lock().unwrap().join("\n");
    assert!(output.contains("runner stdout") && output.contains("runner stderr"));
    f.mode("runner-failure");
    let result = f
        .client
        .run_custom_tests(context, vec![], &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.status, result.accepted, result.runner_exit),
        (Status::Failed, Some(true), Some(7))
    );
}

#[test]
fn stale_and_unavailable_requests_are_not_accepted() {
    let f = Fixture::new();
    let context = f.context();
    fs::write(f.project.join("rokit.toml"), "[tools]\n").unwrap();
    let result = f
        .client
        .run_custom_tests(context.clone(), vec![], &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.status, result.accepted, result.effects),
        (Status::Failed, Some(false), WriteState::NotStarted)
    );
    assert!(!f.root.join("arguments.bin").exists());
    f.mode("old-worker");
    assert!(f.client.test_arguments_support().is_err());
    let result = f
        .client
        .run_custom_tests(context, vec![], &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(result.accepted, Some(false));
}

#[test]
fn corrupt_reports_and_disagreeing_exit_never_prove_success() {
    let f = Fixture::new();
    let context = f.context();
    for mode in [
        "missing-report",
        "malformed-report",
        "mismatch-report",
        "exit-disagrees",
    ] {
        f.mode(mode);
        let events = Arc::new(Mutex::new(Vec::new()));
        let capture = events.clone();
        let result = f
            .client
            .run_custom_tests(
                context.clone(),
                vec![],
                &Cancellation::default(),
                move |event| capture.lock().unwrap().push(event),
            )
            .unwrap();
        assert_eq!(
            (result.status, result.accepted, result.effects),
            (Status::Uncertain, None, WriteState::Unknown),
            "{mode}"
        );
        assert!(matches!(
            events.lock().unwrap().last(),
            Some(OperationEvent::Finished {
                outcome: rproj_core::operations::OperationOutcome::Failed { .. }
            })
        ));
    }
}

#[test]
fn cancellation_waits_for_runner_descendants_and_pipe_drain() {
    let f = Fixture::new();
    let context = f.context();
    f.mode("cancel");
    let cancellation = Cancellation::default();
    let cancel = cancellation.clone();
    let started = std::time::Instant::now();
    let result = f
        .client
        .run_custom_tests(context, vec![], &cancellation, move |event| {
            if format!("{event:?}").contains("cancel-ready") {
                cancel.cancel();
            }
        })
        .unwrap();
    assert_eq!(
        (result.status, result.accepted),
        (Status::Cancelled, Some(true))
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    f.assert_reaped("lute.exe");
}

#[test]
fn normal_completion_reaps_descendants_before_returning() {
    let f = Fixture::new();
    let context = f.context();
    f.mode("descendant");
    let started = std::time::Instant::now();
    let result = f
        .client
        .run_custom_tests(context, vec![], &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(result.status, Status::Succeeded);
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    f.assert_reaped("worker.exe");
}

#[test]
fn competing_run_is_refused_without_acceptance() {
    let f = Fixture::new();
    let context = f.context();
    let (started, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let wait = Mutex::new(wait);
    let client = f.client.clone();
    let project = f.project.clone();
    let worker = std::thread::spawn(move || {
        client.test_context(&project, &Cancellation::default(), move |event| {
            if matches!(event, OperationEvent::Started) {
                started.send(()).unwrap();
                wait.lock()
                    .unwrap()
                    .recv_timeout(std::time::Duration::from_secs(10))
                    .unwrap();
            }
        })
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let result = f
        .client
        .run_custom_tests(context, vec![], &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(
        (result.accepted, result.effects),
        (Some(false), WriteState::NotStarted)
    );
    assert!(result.message.unwrap().contains("Another operation"));
    f.client.test_arguments_support().unwrap();
    release.send(()).unwrap();
    worker.join().unwrap().unwrap();
}

#[test]
fn cancelling_stalled_support_probe_reaps_children_and_releases_guard() {
    let f = Fixture::new();
    let context = f.context();
    f.mode("support-stall");
    let cancellation = Cancellation::default();
    let cancel = cancellation.clone();
    let ready = f.root.join("probe-ready");
    let trigger = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ready.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        cancel.cancel();
        assert!(ready.exists(), "probe did not start");
    });
    let started = std::time::Instant::now();
    let result = f
        .client
        .run_custom_tests(context, vec![], &cancellation, |_| {})
        .unwrap();
    trigger.join().unwrap();
    assert_eq!(
        (result.status, result.accepted, result.effects),
        (Status::Cancelled, Some(false), WriteState::NotStarted)
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    f.assert_reaped("worker.exe");
    f.mode("success");
    f.context();
}

#[test]
fn stalled_support_probe_has_a_deadline_without_user_cancellation() {
    let f = Fixture::new();
    f.mode("support-stall");
    let started = std::time::Instant::now();
    let error = f.client.test_arguments_support().unwrap_err();
    assert!(error.to_string().contains("timed out"));
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    f.assert_reaped("worker.exe");
}

#[test]
fn spawn_refusal_returns_a_completion_even_without_finished_event() {
    let f = Fixture::new();
    let context = f.context();
    let worker = f.root.join("worker.exe");
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let result = f
        .client
        .run_custom_tests(context, vec![], &Cancellation::default(), move |event| {
            if matches!(event, OperationEvent::Started) {
                fs::remove_file(&worker).unwrap();
            }
            captured.lock().unwrap().push(event);
        })
        .unwrap();
    assert_eq!(
        (result.status, result.accepted, result.effects),
        (Status::Failed, Some(false), WriteState::NotStarted)
    );
    assert_eq!(events.lock().unwrap().len(), 1);
    assert!(matches!(events.lock().unwrap()[0], OperationEvent::Started));
}
