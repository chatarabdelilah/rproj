use super::*;
use rproj_core::{
    operations::{OperationEvent, WorkbenchClient},
    project_template::builtin_project_template,
};
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn tools(root: &Path) -> global_rojo::MachineTools {
    global_rojo::MachineTools {
        rokit_root: root.join("global-rokit"),
        search: vec![root.join("bin")],
    }
}
fn template_path(root: &Path) -> PathBuf {
    root.join("templates/default.project.json")
}

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_TEMPLATE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_TEMPLATE_IPC").unwrap());
    let result = job_at(&ipc, &template_path(&root), &|| tools(&root).resolve());
    std::process::exit(i32::from(crate::internal_result_exit_code(result)));
}

struct Fixture {
    _storage: tempfile::TempDir,
    root: PathBuf,
    client: WorkbenchClient,
}
impl Fixture {
    fn new() -> Self {
        let storage = tempfile::Builder::new()
            .prefix("rproj template 日本語 ")
            .tempdir()
            .unwrap();
        let root = storage.path().canonicalize().unwrap();
        fs::write(root.join("fixture-mode"), "normal").unwrap();
        let worker = root.join("worker.exe");
        fs::write(
            root.join("bridge.rs"),
            include_str!("../../../tests/fixtures/template_bridge.rs"),
        )
        .unwrap();
        let output = Command::new("rustc")
            .arg("--edition=2024")
            .arg(root.join("bridge.rs"))
            .arg("-o")
            .arg(&worker)
            .env("RPROJ_TEST_WORKER_BINARY", std::env::current_exe().unwrap())
            .env("RPROJ_TEST_WORKER_VERSION", env!("CARGO_PKG_VERSION"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let client = WorkbenchClient::new(worker).unwrap();
        Self {
            _storage: storage,
            root,
            client,
        }
    }
    fn mode(&self, value: &str) {
        fs::write(self.root.join("fixture-mode"), value).unwrap();
    }
    fn install_stub(&self) {
        fs::create_dir(self.root.join("bin")).unwrap();
        fs::copy(self.root.join("worker.exe"), self.root.join("bin/rojo.exe")).unwrap();
    }
    fn load(&self) -> LoadedTemplate {
        self.client.project_template_context(|_| {}).unwrap()
    }
    fn prepare(&self, source: TemplateSource, action: TemplateAction) -> Result<PreparedTemplate> {
        self.client.prepare_project_template(source, action, |_| {})
    }
    fn apply(&self, prepared: PreparedTemplate) -> TemplateOutcome {
        self.client
            .apply_project_template(prepared, |_| {})
            .unwrap()
    }
}
fn changed() -> TemplateAction {
    let mut value = builtin_project_template();
    value["servePort"] = 40001.into();
    TemplateAction::Save(value)
}
#[test]
fn eight_variants_use_the_machine_tool_and_save_the_exact_reviewed_bytes() {
    let f = Fixture::new();
    f.client.project_template_support().unwrap();
    let loaded = f.load();
    assert!(loaded.source.original.is_none() && loaded.problem.is_none());
    assert!(
        f.prepare(loaded.source.clone(), changed())
            .unwrap_err()
            .to_string()
            .contains("Rojo")
    );
    assert!(!f.root.join("templates").exists() && !tools(&f.root).rokit_root.exists());
    f.install_stub();
    let prepared = f.prepare(loaded.source, changed()).unwrap();
    assert!(!f.root.join("templates").exists());
    let lines = fs::read_to_string(f.root.join("validations")).unwrap();
    let actual: BTreeSet<&str> = lines.lines().collect();
    let expected: BTreeSet<String> = [
        "plain",
        "plain-tests",
        "wally",
        "wally-server",
        "wally-tests",
        "wally-tests-server",
        "jest",
        "jest-server",
    ]
    .into_iter()
    .flat_map(|label| ["sourcemap", "build"].map(|cmd| format!("{label} {cmd}")))
    .collect();
    assert_eq!(actual.len(), 16);
    assert!(expected.iter().all(|line| actual.contains(line.as_str())));
    let bytes = prepared.review.replacement.clone().unwrap();
    f.mode("descendant");
    let lines = Arc::new(Mutex::new(Vec::new()));
    let captured = lines.clone();
    let saved = f
        .client
        .apply_project_template(prepared, move |event| {
            if let OperationEvent::Output { text, .. } = event {
                captured.lock().unwrap().push(text)
            }
        })
        .unwrap();
    assert_eq!(
        (saved.status, saved.mutation),
        (TemplateStatus::Succeeded, TemplateMutation::Completed)
    );
    assert_eq!(
        saved.saved.unwrap().original.as_deref(),
        Some(bytes.as_str())
    );
    assert_eq!(fs::read_to_string(template_path(&f.root)).unwrap(), bytes);
    assert!(f.root.join("descendant-completed").is_file());
    assert!(
        lines
            .lock()
            .unwrap()
            .iter()
            .any(|s| s.contains("final descendant output"))
    );
}

#[test]
fn noop_and_reset_need_no_rojo_and_malformed_input_is_repairable() {
    let f = Fixture::new();
    let source = f.load().source;
    for action in [
        TemplateAction::Save(builtin_project_template()),
        TemplateAction::Reset,
    ] {
        let plan = f.prepare(source.clone(), action).unwrap();
        assert!(plan.rojo.is_none());
        assert_eq!(f.apply(plan).saved, Some(source.clone()));
        assert!(!f.root.join("templates").exists());
    }
    fs::create_dir(f.root.join("templates")).unwrap();
    fs::write(template_path(&f.root), "{broken").unwrap();
    let loaded = f.load();
    assert!(loaded.problem.is_some());
    assert_eq!(loaded.source.original.as_deref(), Some("{broken"));
    let reset = f.prepare(loaded.source, TemplateAction::Reset).unwrap();
    assert!(template_path(&f.root).exists());
    let out = f.apply(reset);
    assert_eq!(out.status, TemplateStatus::Succeeded);
    assert!(out.saved.unwrap().original.is_none());
    assert!(!template_path(&f.root).exists());
}

#[test]
fn stale_source_tool_and_forged_reviews_cannot_write() {
    let f = Fixture::new();
    f.install_stub();
    let source = f.load().source;
    let prepared = f.prepare(source.clone(), changed()).unwrap();
    let mut forged = prepared.clone();
    forged.rojo.as_mut().unwrap().executable = f.root.join("never-launch.exe");
    assert_eq!(f.apply(forged).mutation, TemplateMutation::NotStarted);
    assert!(!f.root.join("templates").exists());
    f.mode("reject-rojo");
    let rejected = f.apply(prepared.clone());
    assert_eq!(rejected.status, TemplateStatus::Failed);
    assert!(rejected.message.unwrap().contains("rejected"));
    f.mode("normal");
    let mut foreign = source;
    foreign.path.set_file_name("foreign.json");
    assert!(
        f.prepare(foreign, changed())
            .unwrap_err()
            .to_string()
            .contains("different machine template")
    );
    fs::create_dir(f.root.join("templates")).unwrap();
    fs::write(template_path(&f.root), "external").unwrap();
    let stale = f.apply(prepared);
    assert_eq!(stale.status, TemplateStatus::Failed);
    assert_eq!(
        fs::read_to_string(template_path(&f.root)).unwrap(),
        "external"
    );
}

#[test]
fn missing_or_untrustworthy_reports_require_reload_even_when_exit_succeeds() {
    let f = Fixture::new();
    let reset = f.prepare(f.load().source, TemplateAction::Reset).unwrap();
    for mode in ["missing", "malformed", "mismatched", "bad-exit"] {
        f.mode(mode);
        let out = f.apply(reset.clone());
        assert_eq!(
            (out.status, out.mutation),
            (TemplateStatus::Uncertain, TemplateMutation::Unknown)
        );
        assert!(out.saved.is_none());
    }
    f.mode("old-worker");
    assert!(f.client.project_template_support().is_err());
    f.mode("normal");
    f.install_stub();
    let prepared = f.prepare(f.load().source, changed()).unwrap();
    f.mode("lost-after-write");
    let out = f.apply(prepared);
    assert_eq!(
        (out.status, out.mutation),
        (TemplateStatus::Uncertain, TemplateMutation::Unknown)
    );
    assert!(template_path(&f.root).exists() && out.saved.is_none());
}

#[test]
fn global_pin_uses_installed_storage_and_rejects_an_unsafe_or_missing_pin() {
    let f = Fixture::new();
    f.install_stub();
    let machine = tools(&f.root);
    fs::create_dir(&machine.rokit_root).unwrap();
    let manifest = machine.rokit_root.join("rokit.toml");
    fs::write(&manifest, "[tools]\nrojo='GitHubOrg/RoJo@1.2.3'").unwrap();
    assert!(
        machine.resolve().is_err(),
        "a missing global pin must not fall through to PATH"
    );
    let installed = machine
        .rokit_root
        .join("tool-storage/githuborg/rojo/1.2.3/rojo.exe");
    fs::create_dir_all(installed.parent().unwrap()).unwrap();
    fs::copy(f.root.join("worker.exe"), &installed).unwrap();
    let captured = machine.resolve().unwrap();
    assert_eq!(captured.executable, installed);
    let prepared = f.prepare(f.load().source, changed()).unwrap();
    fs::write(
        &manifest,
        "# externally changed\n[tools]\nrojo='GitHubOrg/RoJo@1.2.3'",
    )
    .unwrap();
    let out = f.apply(prepared);
    assert_eq!(
        (out.status, out.mutation),
        (TemplateStatus::Failed, TemplateMutation::NotStarted)
    );
    assert!(!template_path(&f.root).exists());
    for invalid in [
        "[",
        "[tools]\nrojo='../rojo@1.2.3'",
        "[tools]\nrojo='owner/../rojo@1.2.3'",
        "[tools]\nrojo=1",
        "[tools]\nrojo='owner/rojo@latest'",
        "[tools]\nrojo='a/b@1.0.0'\nRoJo='c/d@2.0.0'",
    ] {
        fs::write(&manifest, invalid).unwrap();
        assert!(machine.resolve().is_err(), "{invalid}");
    }
}

#[test]
fn oversized_input_or_report_is_refused_without_template_mutation() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("templates")).unwrap();
    let path = template_path(&f.root);
    fs::write(&path, vec![b'x'; IPC_LIMIT as usize + 1]).unwrap();
    assert!(
        f.client
            .project_template_context(|_| {})
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
    let small_enough_file = "x".repeat(IPC_LIMIT as usize - 5);
    fs::write(&path, &small_enough_file).unwrap();
    assert!(
        f.client
            .project_template_context(|_| {})
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), small_enough_file);
    fs::write(&path, "{}").unwrap();
    let source = f.load().source;
    let mut huge = builtin_project_template();
    huge["description"] = "x".repeat(IPC_LIMIT as usize).into();
    assert!(
        f.prepare(source, TemplateAction::Save(huge))
            .unwrap_err()
            .to_string()
            .contains("2 MiB")
    );
    assert_eq!(fs::read_to_string(path).unwrap(), "{}");
}

#[test]
fn missing_worker_after_preparation_is_a_recoverable_pre_spawn_failure() {
    let f = Fixture::new();
    let prepared = f.prepare(f.load().source, TemplateAction::Reset).unwrap();
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let out = f
        .client
        .apply_project_template(prepared, |_| {
            panic!("A worker that did not spawn cannot emit lifecycle events")
        })
        .unwrap();
    assert_eq!(
        (out.status, out.mutation),
        (TemplateStatus::Failed, TemplateMutation::NotStarted)
    );
    assert!(out.saved.is_none() && !template_path(&f.root).exists());
}

#[test]
fn path_lookup_skips_known_project_aware_launchers_and_relative_directories() {
    let f = Fixture::new();
    let mut machine = tools(&f.root);
    let launcher = f.root.join(".aftman/bin");
    fs::create_dir_all(&launcher).unwrap();
    fs::copy(f.root.join("worker.exe"), launcher.join("rojo.exe")).unwrap();
    machine.search = vec![PathBuf::from("."), launcher];
    assert!(machine.resolve().is_err());
    assert!(!machine.rokit_root.exists());
}

#[test]
fn relative_machine_destination_matches_the_absolute_source_loaded_by_context() {
    let ipc = tempfile::tempdir().unwrap();
    let destination = Path::new(".")
        .join(ipc.path().file_name().unwrap())
        .join("default.project.json");
    assert!(!destination.exists());
    let source = template_checked::capture(&destination, Some(IPC_LIMIT)).unwrap();
    let request = TemplateRequest {
        protocol: TEMPLATE_PROTOCOL,
        id: "relative-fixture".into(),
        action: TemplateJob::Prepare {
            source,
            action: TemplateAction::Reset,
        },
    };
    write_atomic(&ipc.path().join("request.json"), &request).unwrap();
    job_at(ipc.path(), &destination, &|| {
        panic!("Reset needs no validator")
    })
    .unwrap();
    let report: TemplateReport = read_json(&ipc.path().join("report.json")).unwrap();
    report.check(&request).unwrap();
    assert_eq!(report.status, Some(TemplateStatus::Succeeded));
    assert!(!destination.exists());
}
