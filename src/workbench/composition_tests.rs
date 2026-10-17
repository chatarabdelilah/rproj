use super::*;
use crate::test_common::project_snapshot;
use rproj_core::{
    creation::{CompositionPlan, PackageWorkflow},
    operations::{Cancellation, OperationEvent, WorkbenchClient},
};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_COMPOSITION_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let result = crate::background_watch::with_test_root(&root.join("watch"), || -> Result<()> {
        if std::env::var("RPROJ_TEST_COMPOSITION_ACTION")? == "context" {
            fs::write(
                root.join("context.json"),
                serde_json::to_vec(&context(&std::env::current_dir()?)?)?,
            )?;
            return Ok(());
        }
        let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_COMPOSITION_IPC").unwrap());
        job(&ipc)
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
            .prefix("rproj composition 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        let project = root.join("captured project 世界");
        fs::create_dir_all(&project).unwrap();
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
        fs::write(root.join("mode"), "success").unwrap();
        let graph = rproj_core::creation::ProjectGraph {
            mode: "expert".into(),
            package_workflow: PackageWorkflow::None,
            ..Default::default()
        };
        fs::write(
            project.join("rproj.toml"),
            format!(
                "# preserve until completion\n{}",
                toml::to_string(&graph).unwrap()
            ),
        )
        .unwrap();
        fs::write(
            project.join("default.project.json"),
            serde_json::to_vec(&crate::steps::rojo::builtin_project_template()).unwrap(),
        )
        .unwrap();
        let source = root.join("bridge.rs");
        fs::write(
            &source,
            include_str!("../../tests/fixtures/composition_bridge.rs"),
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
        fs::copy(&worker, root.join("tools/rokit.exe")).unwrap();
        let client = WorkbenchClient::new(worker).unwrap();
        Self {
            _directory: directory,
            root,
            project,
            client,
        }
    }
    fn request(&self) -> PrepareComposition {
        let context = self.client.composition_context(&self.project).unwrap();
        let mut draft = CompositionDraft::new(context.graph.clone().unwrap());
        draft.set_capability("format", Some("stylua")).unwrap();
        PrepareComposition { context, draft }
    }
    fn prepared(&self) -> CompositionPlan {
        match self
            .client
            .prepare_composition(self.request(), &Cancellation::default(), |_| {})
            .unwrap()
        {
            CompositionPreparation::Prepared(plan) => *plan,
            _ => panic!("preparation cancelled"),
        }
    }
}

#[test]
fn preparation_is_read_only_and_success_uses_structured_outcome() {
    let fixture = Fixture::new();
    let before = project_snapshot(&fixture.project);
    let watch = project_snapshot(&fixture.root.join("watch"));
    let prepared = fixture.prepared();
    assert_eq!(project_snapshot(&fixture.project), before);
    assert_eq!(project_snapshot(&fixture.root.join("watch")), watch);
    let output = Arc::new(Mutex::new(String::new()));
    let captured = output.clone();
    let result = fixture
        .client
        .apply_composition(prepared, &Cancellation::default(), move |event| {
            if let OperationEvent::Output { text, .. } = event {
                captured.lock().unwrap().push_str(&text);
            }
        })
        .unwrap();
    assert_eq!(result.status, CompositionStatus::Succeeded);
    assert_eq!(result.writes, WriteState::Completed);
    assert_eq!(result.project, fixture.project);
    assert!(output.lock().unwrap().contains("fixture Rokit stdout"));
    assert!(output.lock().unwrap().contains("fixture Rokit stderr"));
    let graph: rproj_core::creation::ProjectGraph =
        toml::from_str(&fs::read_to_string(fixture.project.join("rproj.toml")).unwrap()).unwrap();
    assert_eq!(graph.capabilities["format"], "stylua");
    assert_eq!(project_snapshot(&fixture.root.join("watch")), watch);
}

#[test]
fn stale_draft_sources_and_stale_review_targets_require_explicit_reload() {
    let fixture = Fixture::new();
    let request = fixture.request();
    fs::write(fixture.project.join("stylua.toml"), "custom = true\n").unwrap();
    let error = fixture
        .client
        .prepare_composition(request, &Cancellation::default(), |_| {})
        .unwrap_err()
        .to_string();
    assert!(error.contains("explicitly reload"), "{error}");
    let prepared = fixture.prepared();
    fs::write(fixture.project.join("stylua.toml"), "changed = true\n").unwrap();
    let before = project_snapshot(&fixture.project);
    let result = fixture
        .client
        .apply_composition(prepared, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(result.status, CompositionStatus::Failed);
    assert_eq!(result.writes, WriteState::NotStarted);
    assert_eq!(project_snapshot(&fixture.project), before);
}

#[test]
fn failed_tools_preserve_record_and_report_possible_partial_changes() {
    let fixture = Fixture::new();
    let prepared = fixture.prepared();
    let record = fs::read(fixture.project.join("rproj.toml")).unwrap();
    fs::write(fixture.root.join("mode"), "fail").unwrap();
    let result = fixture
        .client
        .apply_composition(prepared, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(result.status, CompositionStatus::Failed);
    assert_eq!(result.writes, WriteState::MayHaveBegun);
    assert_eq!(
        fs::read(fixture.project.join("rproj.toml")).unwrap(),
        record
    );
    assert!(fixture.project.join("stylua.toml").is_file());
    assert!(result.message.unwrap().contains("partial changes"));
}

#[test]
fn absent_or_invalid_reports_never_imply_success_or_no_changes() {
    let fixture = Fixture::new();
    let prepared = fixture.prepared();
    for mode in ["missing-report", "malformed-report"] {
        fs::write(fixture.root.join("mode"), mode).unwrap();
        let outcome = fixture
            .client
            .apply_composition(prepared.clone(), &Cancellation::default(), |_| {})
            .unwrap();
        assert_eq!(outcome.status, CompositionStatus::Uncertain);
        assert_eq!(outcome.writes, WriteState::Unknown);
    }
}

#[test]
fn cancellation_waits_for_owned_descendants_and_preserves_record() {
    let fixture = Fixture::new();
    let prepared = fixture.prepared();
    let record = fs::read(fixture.project.join("rproj.toml")).unwrap();
    fs::write(fixture.root.join("mode"), "cancel").unwrap();
    let client = fixture.client.clone();
    let cancel = Cancellation::default();
    let running_cancel = cancel.clone();
    let running =
        thread::spawn(move || client.apply_composition(prepared, &running_cancel, |_| {}));
    let deadline = Instant::now() + Duration::from_secs(15);
    while !fixture.root.join("ready").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(25));
    }
    let ready = fixture.root.join("ready").exists();
    cancel.cancel();
    let result = running.join().unwrap().unwrap();
    assert!(ready, "fixture did not reach a tool descendant");
    assert_eq!(result.status, CompositionStatus::Cancelled);
    assert_eq!(result.writes, WriteState::MayHaveBegun);
    assert_eq!(
        fs::read(fixture.project.join("rproj.toml")).unwrap(),
        record
    );
    let pid = fs::read_to_string(fixture.root.join("ready")).unwrap();
    let status = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .unwrap();
    assert!(
        !String::from_utf8_lossy(&status.stdout).contains("rokit.exe"),
        "descendant survived cancellation"
    );
}

#[test]
fn unavailable_worker_and_unsupported_records_remain_actionable() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join("mode"), "old-worker").unwrap();
    assert!(
        fixture
            .client
            .composition_context(&fixture.project)
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
    fs::write(fixture.root.join("mode"), "success").unwrap();
    fs::write(
        fixture.project.join("rproj.toml"),
        "packages = ['unfamiliar']\n",
    )
    .unwrap();
    let context = fixture
        .client
        .composition_context(&fixture.project)
        .unwrap();
    assert_eq!(context.graph.unwrap().packages, ["unfamiliar"]);
    assert!(context.problem.unwrap().contains("Unsupported"));
}

#[test]
fn already_cancelled_operation_never_launches_or_changes_project() {
    let fixture = Fixture::new();
    let prepared = fixture.prepared();
    let before = project_snapshot(&fixture.project);
    let token = Cancellation::default();
    token.cancel();
    let result = fixture
        .client
        .apply_composition(prepared, &token, |_| {
            panic!("cancelled operation should not launch")
        })
        .unwrap();
    assert_eq!(result.status, CompositionStatus::Cancelled);
    assert_eq!(result.writes, WriteState::NotStarted);
    assert_eq!(project_snapshot(&fixture.project), before);
}

#[test]
fn malformed_missing_and_unsupported_records_do_not_adopt_or_normalize() {
    let fixture = Fixture::new();
    for source in ["[capabilities]\nfuture='custom'\n", "packages=[\n"] {
        fs::write(fixture.project.join("rproj.toml"), source).unwrap();
        let before = project_snapshot(&fixture.project);
        let context = fixture
            .client
            .composition_context(&fixture.project)
            .unwrap();
        assert!(context.problem.is_some());
        assert_eq!(project_snapshot(&fixture.project), before);
    }
    fs::remove_file(fixture.project.join("rproj.toml")).unwrap();
    let context = fixture
        .client
        .composition_context(&fixture.project)
        .unwrap();
    assert!(context.graph.is_none());
    assert!(context.problem.unwrap().contains("Missing"));
    assert!(!fixture.project.join("rproj.toml").exists());
}
