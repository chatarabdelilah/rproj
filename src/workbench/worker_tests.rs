//! These tests use the production dispatcher and exit reporting in a test-build
//! child process. The bridge and Lute stub are fixtures, not shipped CLI or real
//! Lute/Roblox acceptance. Only the child receives fixture environment settings.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use rproj_core::operations::{
    Cancellation, Operation, OperationEvent, OperationOutcome, WorkbenchClient,
};

use crate::test_common::project_snapshot;

const DRIVER_ROOT: &str = "RPROJ_TEST_WORKBENCH_ROOT";
const DRIVER_OPERATION: &str = "RPROJ_TEST_WORKBENCH_OPERATION";

#[test]
fn isolated_worker_driver() {
    let Some(root) = std::env::var_os(DRIVER_ROOT) else {
        return;
    };
    let root = PathBuf::from(root);
    assert!(root.is_absolute() && root.is_dir());
    let operation = std::env::var_os(DRIVER_OPERATION).expect("fixture operation");
    crate::background_watch::with_test_root(&root, || {
        let result = super::parse_operation([operation].into_iter()).and_then(super::dispatch);
        std::process::exit(i32::from(crate::internal_result_exit_code(result)));
    });
}

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    watch: PathBuf,
    client: WorkbenchClient,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("rproj isolated workbench ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        let project = root.join("project with spaces");
        let watch = root.join("watch");
        fs::create_dir_all(project.join(".lute")).unwrap();
        fs::create_dir_all(project.join("src")).unwrap();
        fs::create_dir_all(root.join("tools")).unwrap();
        fs::create_dir_all(&watch).unwrap();
        fs::write(
            project.join("rproj.toml"),
            "mode = 'expert'\npackage_workflow = 'none'\n[capabilities]\ntest = 'testez'\n",
        )
        .unwrap();
        fs::write(
            project.join("default.project.json"),
            r#"{"name":"Isolated worker fixture","tree":{}}"#,
        )
        .unwrap();
        fs::write(project.join(".lute/check.luau"), "-- preserved check\n").unwrap();
        fs::write(project.join("src/init.luau"), "return 'preserved'\n").unwrap();
        // Seed the ordinary lock files so both file contents and directory
        // membership can be compared, even when a command acquires the guard.
        for name in [
            "start.lock",
            "owner.lock",
            "state.lock",
            "state-writer.lock",
        ] {
            fs::write(watch.join(name), b"").unwrap();
        }
        fs::write(watch.join("watch.log"), b"preserved historical output\n").unwrap();
        fs::write(root.join("lute-exit-code"), "0").unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../tests/fixtures/workbench_bridge.rs"),
        )
        .unwrap();
        let bridge = root.join("worker-bridge.exe");
        let compiled = Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(&bridge)
            .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
            .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "fixture compiler: {}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        fs::copy(&bridge, root.join("tools/lute.exe")).unwrap();
        let client = WorkbenchClient::new(bridge).unwrap();
        Self {
            _directory: directory,
            root,
            project,
            watch,
            client,
        }
    }

    fn run(&self, operation: Operation, tool_exit_code: i32) -> Captured {
        fs::write(self.root.join("lute-exit-code"), tool_exit_code.to_string()).unwrap();
        let project_before = project_snapshot(&self.project);
        let events = Arc::new(Mutex::new(Vec::new()));
        let collected = events.clone();
        let outcome = thread::scope(|scope| {
            let cancellation = Cancellation::default();
            let deadline_cancellation = cancellation.clone();
            let (finished, wait) = mpsc::channel();
            let deadline = scope.spawn(move || {
                if matches!(
                    wait.recv_timeout(Duration::from_secs(20)),
                    Err(mpsc::RecvTimeoutError::Timeout)
                ) {
                    deadline_cancellation.cancel();
                    true
                } else {
                    false
                }
            });
            let outcome = self
                .client
                .run(operation, &self.project, &cancellation, move |event| {
                    collected.lock().unwrap().push(event)
                });
            // Stop and join before unwrapping errors. On panic, dropping the
            // sender wakes the timer and the scope still joins its thread.
            let _ = finished.send(());
            assert!(
                !deadline.join().unwrap(),
                "isolated {operation:?} worker exceeded its 20-second deadline"
            );
            outcome
        })
        .unwrap();
        assert_eq!(project_snapshot(&self.project), project_before);
        let events = events.lock().unwrap();
        assert!(matches!(events.first(), Some(OperationEvent::Started)));
        assert!(matches!(
            events.last(),
            Some(OperationEvent::Finished { outcome: finished }) if *finished == outcome
        ));
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, OperationEvent::Started))
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, OperationEvent::Finished { .. }))
                .count(),
            1
        );
        let mut captured = Captured {
            outcome,
            stdout: String::new(),
            stderr: String::new(),
        };
        for event in events.iter() {
            if let OperationEvent::Output { stderr, text } = event {
                if *stderr {
                    captured.stderr.push_str(text);
                } else {
                    captured.stdout.push_str(text);
                }
            }
        }
        captured
    }

    fn start_lock(&self) -> fs::File {
        fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.watch.join("start.lock"))
            .unwrap()
    }
}

struct Captured {
    outcome: OperationOutcome,
    stdout: String,
    stderr: String,
}

impl Captured {
    fn assert_tool_ran(&self, args: &[&str], project: &Path) {
        let output = format!("fixture Lute stdout: {args:?}");
        let lines: Vec<_> = self
            .stdout
            .lines()
            .filter(|line| line.starts_with("fixture Lute stdout: "))
            .collect();
        assert_eq!(lines, [output.as_str()], "{}", self.stdout);
        assert_eq!(
            self.stderr.matches("fixture Lute stderr").count(),
            1,
            "{}",
            self.stderr
        );
        let reported = self
            .stdout
            .lines()
            .find_map(|line| line.strip_prefix("fixture project: "))
            .expect("fixture tool reports its working directory");
        assert_eq!(Path::new(reported).canonicalize().unwrap(), project);
    }
}

#[test]
fn shared_dispatcher_reports_pass_fail_and_both_streams_without_changing_fixtures() {
    let fixture = Fixture::new();
    let watch_before = project_snapshot(&fixture.watch);
    for (operation, args, failed_code, error) in [
        (
            Operation::Check,
            &["run", "check"][..],
            1,
            "lute run check failed",
        ),
        (
            Operation::Test,
            &["test"][..],
            7,
            "lute reported test failures (exit code 7)",
        ),
    ] {
        let passed = fixture.run(operation, 0);
        passed.assert_tool_ran(args, &fixture.project);
        assert_eq!(passed.outcome, OperationOutcome::Succeeded);
        assert_eq!(project_snapshot(&fixture.watch), watch_before);

        let failed = fixture.run(operation, 7);
        failed.assert_tool_ran(args, &fixture.project);
        assert_eq!(
            failed.outcome,
            OperationOutcome::Failed {
                code: Some(failed_code)
            }
        );
        assert!(failed.stderr.contains(error), "{}", failed.stderr);
        assert_eq!(project_snapshot(&fixture.watch), watch_before);

        // A failed worker must release the real shared mutation guard too.
        fixture.start_lock().try_lock().unwrap();
    }
}

#[test]
fn shared_dispatcher_refuses_a_held_watch_guard_and_recovers_after_release() {
    let fixture = Fixture::new();
    let watch_before = project_snapshot(&fixture.watch);
    for (operation, args) in [
        (Operation::Check, &["run", "check"][..]),
        (Operation::Test, &["test"][..]),
    ] {
        let held = fixture.start_lock();
        held.try_lock().unwrap();
        let refused = fixture.run(operation, 0);
        assert_eq!(refused.outcome, OperationOutcome::Failed { code: Some(1) });
        assert!(
            refused.stderr.contains("Background Watch is starting"),
            "{}",
            refused.stderr
        );
        assert!(!refused.stdout.contains("fixture Lute stdout"));
        assert!(!refused.stderr.contains("fixture Lute stderr"));
        drop(held);
        assert_eq!(project_snapshot(&fixture.watch), watch_before);

        let passed = fixture.run(operation, 0);
        passed.assert_tool_ran(args, &fixture.project);
        assert_eq!(passed.outcome, OperationOutcome::Succeeded);
        assert_eq!(project_snapshot(&fixture.watch), watch_before);
    }
}
