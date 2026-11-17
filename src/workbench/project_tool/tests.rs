use super::*;
use rproj_core::{
    operations::{Cancellation, OperationEvent, WorkbenchClient},
    project_tool_setup::{Plan, Step, StepState},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_PROJECT_TOOL_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_PROJECT_TOOL_IPC").unwrap());
    let result = crate::background_watch::with_test_root(&root.join("watch"), || job(&ipc));
    std::process::exit(i32::from(crate::internal_result_exit_code(result)));
}

fn binaries() -> &'static Path {
    static TOOLS: OnceLock<tempfile::TempDir> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            let root = tempfile::tempdir().unwrap();
            for (name, source) in [
                (
                    "worker",
                    include_str!("../../../tests/fixtures/project_tool_bridge.rs"),
                ),
                (
                    "rokit",
                    include_str!("../../../tests/fixtures/project_tool_rokit.rs"),
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
    _directory: tempfile::TempDir,
    root: PathBuf,
    project: PathBuf,
    client: WorkbenchClient,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::Builder::new()
            .prefix("rproj Tool 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        let project = root.join("project space 世界");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("default.project.json"), "{}").unwrap();
        fs::create_dir(root.join("watch")).unwrap();
        for name in [
            "start.lock",
            "owner.lock",
            "state.lock",
            "state-writer.lock",
        ] {
            fs::write(root.join("watch").join(name), "").unwrap();
        }
        fs::create_dir(root.join("tools")).unwrap();
        fs::copy(binaries().join("worker.exe"), root.join("worker.exe")).unwrap();
        fs::copy(binaries().join("rokit.exe"), root.join("tools/rokit.exe")).unwrap();
        fs::write(root.join("mode"), "success").unwrap();
        let client = WorkbenchClient::new(root.join("worker.exe")).unwrap();
        Self {
            _directory: directory,
            root,
            project,
            client,
        }
    }
    fn mode(&self, mode: &str) {
        fs::write(self.root.join("mode"), mode).unwrap();
    }
    fn plan(&self) -> Plan {
        self.client.project_tool_support().unwrap();
        self.client
            .prepare_project_tool(&self.project, "asphalt", &Cancellation::default(), |_| {})
            .unwrap()
    }
    fn ipc(&self, plan: Plan) -> tempfile::TempDir {
        let ipc = tempfile::tempdir().unwrap();
        write_atomic(
            &ipc.path().join("request.json"),
            &Request {
                protocol: PROJECT_TOOL_PROTOCOL,
                id: "fixture".into(),
                project: self.project.clone(),
                tool: plan.tool.clone(),
                action: Action::Execute(plan),
            },
        )
        .unwrap();
        ipc
    }
}

#[test]
fn prepare_is_read_only_and_setup_returns_captured_per_step_outcomes() {
    let f = Fixture::new();
    let before = crate::test_common::project_snapshot(&f.root);
    let plan = f.plan();
    assert_eq!(before, crate::test_common::project_snapshot(&f.root));
    let outcome = f.client.setup_project_tool(plan.clone(), |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.effects),
        (Status::Succeeded, WriteState::Completed)
    );
    assert_eq!(outcome.project, f.project);
    assert_eq!(outcome.outcome.unwrap().steps.len(), plan.steps.len());
    assert!(f.project.join("asphalt.toml").is_file());
    assert!(!f.project.join("rproj.toml").exists());
}

#[test]
fn failed_add_after_pin_is_failed_while_independent_files_are_reported() {
    let f = Fixture::new();
    let plan = f.plan();
    f.mode("failed-after-pin");
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.effects),
        (Status::Failed, WriteState::MayHaveBegun)
    );
    let steps = outcome.outcome.unwrap().steps;
    assert!(steps.iter().any(|s| s.state == StepState::Failed));
    assert!(
        steps
            .iter()
            .any(|s| s.label == "Create asphalt.toml" && s.state == StepState::Completed)
    );
    assert!(f.project.join("asphalt.toml").is_file());
}

#[test]
fn stale_missing_targets_and_tampered_effects_refuse_before_mutation() {
    let f = Fixture::new();
    let plan = f.plan();
    let mut tampered = plan.clone();
    tampered.steps.push(Step::CreateFile {
        relative: "../escape".into(),
        contents: "bad".into(),
    });
    let outcome = f.client.setup_project_tool(tampered, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.effects),
        (Status::Failed, WriteState::NotStarted)
    );
    fs::write(f.project.join("asphalt.toml"), "other writer").unwrap();
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(outcome.effects, WriteState::NotStarted);
    assert!(!f.root.join("tool-calls").exists());
    assert_eq!(
        fs::read_to_string(f.project.join("asphalt.toml")).unwrap(),
        "other writer"
    );
}

#[test]
fn absent_prerequisites_and_unsupported_workers_are_recoverable() {
    let f = Fixture::new();
    fs::remove_file(f.root.join("tools/rokit.exe")).unwrap();
    let plan = f.plan();
    assert!(!plan.prerequisites.is_empty());
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.effects),
        (Status::Failed, WriteState::NotStarted)
    );
    f.mode("old-worker");
    assert!(f.client.project_tool_support().is_err());
}

#[test]
fn replaced_project_and_missing_parent_races_leave_the_new_owner_unchanged() {
    for replace_root in [false, true] {
        let f = Fixture::new();
        let plan = f.plan();
        if replace_root {
            fs::rename(&f.project, f.root.join("old-project")).unwrap();
            fs::create_dir(&f.project).unwrap();
            fs::write(f.project.join("default.project.json"), "{}").unwrap();
        } else {
            fs::create_dir(f.project.join("assets")).unwrap();
        }
        let before = crate::test_common::project_snapshot(&f.project);
        let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
        assert_eq!(
            (outcome.status, outcome.effects),
            (Status::Failed, WriteState::NotStarted)
        );
        assert_eq!(before, crate::test_common::project_snapshot(&f.project));
        assert!(!f.root.join("tool-calls").exists());
    }
}

#[test]
fn pre_cancelled_preparation_and_oversized_inputs_create_nothing() {
    let f = Fixture::new();
    let cancellation = Cancellation::default();
    cancellation.cancel();
    assert!(
        f.client
            .prepare_project_tool(&f.project, "asphalt", &cancellation, |_| {})
            .unwrap_err()
            .to_string()
            .contains("cancelled")
    );
    fs::write(
        f.project.join("rokit.toml"),
        format!("# {}", "x".repeat(IPC_LIMIT as usize)),
    )
    .unwrap();
    let before = crate::test_common::project_snapshot(&f.project);
    let error = f
        .client
        .prepare_project_tool(&f.project, "asphalt", &Cancellation::default(), |_| {})
        .unwrap_err();
    assert!(format!("{error:#}").contains("2 MiB"));
    assert_eq!(before, crate::test_common::project_snapshot(&f.project));
    assert!(!f.root.join("tool-calls").exists());
}

#[test]
fn corrupt_missing_mismatched_and_contradictory_reports_cannot_claim_success() {
    for mode in [
        "missing-report",
        "malformed-report",
        "mismatch-report",
        "exit-disagrees",
    ] {
        let f = Fixture::new();
        let plan = f.plan();
        f.mode(mode);
        let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
        assert_eq!(
            (outcome.status, outcome.effects),
            (Status::Uncertain, WriteState::Unknown),
            "{mode}"
        );
        assert!(outcome.outcome.is_none());
    }
}

#[test]
fn oversized_requests_and_missing_worker_fail_before_effects() {
    let f = Fixture::new();
    let mut plan = f.plan();
    let original = plan.clone();
    plan.notes.push("x".repeat(IPC_LIMIT as usize));
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.effects),
        (Status::Failed, WriteState::NotStarted)
    );
    assert!(outcome.message.unwrap().contains("2 MiB"));
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let outcome = f.client.setup_project_tool(original, |_| {}).unwrap();
    assert_eq!(outcome.effects, WriteState::NotStarted);
    assert!(!f.project.join("rokit.toml").exists());
}

#[test]
fn same_project_watch_blocks_execution_and_other_watch_remains_untouched() {
    let f = Fixture::new();
    // Use an existing pin so the direct in-process fixture needs no tool lookup.
    fs::write(
        f.project.join("rokit.toml"),
        "[tools]\nasphalt='owner/fork@1'\n",
    )
    .unwrap();
    let plan = checked::prepare(&f.project, "asphalt").unwrap();
    crate::background_watch::with_unresponsive_fixture(&f.project, || {
        let ipc = f.ipc(plan.clone());
        assert!(job(ipc.path()).is_err());
        let report: Report = read_json(&ipc.path().join("report.json")).unwrap();
        assert_eq!(report.effects, WriteState::NotStarted);
    });
    let other = tempfile::tempdir().unwrap();
    let other = other.path().canonicalize().unwrap();
    crate::background_watch::with_unresponsive_fixture(&other, || {
        let ipc = f.ipc(plan);
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
fn confirmed_exit_drains_pipes_and_reaps_descendants() {
    let f = Fixture::new();
    let plan = f.plan();
    f.mode("descendant");
    let started = std::time::Instant::now();
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(outcome.status, Status::Succeeded);
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
    let pid = fs::read_to_string(f.root.join("descendant.pid")).unwrap();
    let listed = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
        .output()
        .unwrap();
    assert!(
        listed.status.success(),
        "tasklist failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(!String::from_utf8_lossy(&listed.stdout).contains("worker.exe"));
}

#[test]
fn serialized_guard_rejects_competing_mutation_while_support_stays_readable() {
    let f = Fixture::new();
    let plan = f.plan();
    let (started, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let wait = std::sync::Mutex::new(wait);
    let client = f.client.clone();
    let project = f.project.clone();
    let worker = std::thread::spawn(move || {
        client.prepare_project_tool(
            &project,
            "asphalt",
            &Cancellation::default(),
            move |event| {
                if matches!(event, OperationEvent::Started) {
                    started.send(()).unwrap();
                    wait.lock()
                        .unwrap()
                        .recv_timeout(std::time::Duration::from_secs(10))
                        .unwrap();
                }
            },
        )
    });
    ready
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let outcome = f.client.setup_project_tool(plan, |_| {}).unwrap();
    assert_eq!(outcome.effects, WriteState::NotStarted);
    assert!(outcome.message.unwrap().contains("Another operation"));
    f.client.project_tool_support().unwrap();
    release.send(()).unwrap();
    worker.join().unwrap().unwrap();
}
