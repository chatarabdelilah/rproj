use super::*;
use rproj_core::{
    operations::{Cancellation, WorkbenchClient},
    tool_configuration::SettingsReview,
};
use serde_json::json;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn driver() {
    let Some(root) = std::env::var_os("RPROJ_TEST_SETTINGS_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let ipc = PathBuf::from(std::env::var_os("RPROJ_TEST_SETTINGS_IPC").unwrap());
    let result = crate::background_watch::with_test_root(&root.join("watch"), || {
        let result = job(&ipc);
        if fs::read_to_string(root.join("mode")).unwrap() == "fail-after" && result.is_ok() {
            let path = ipc.join("report.json");
            let mut report: SettingsReport = read_json(&path)?;
            report.status = Some(SettingsStatus::Uncertain);
            report.writes = WriteState::MayHaveBegun;
            report.loaded = None;
            report.message =
                Some("Fixture lost completion after replacement; Reload from file".into());
            write_atomic(&path, &report)?;
            anyhow::bail!("fixture completion failure");
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
            .prefix("rproj Settings 日本語 ")
            .tempdir()
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        let project = root.join("captured project 世界");
        fs::create_dir(&project).unwrap();
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
            include_str!("../../../tests/fixtures/settings_bridge.rs"),
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
    fn mode(&self, mode: &str) {
        fs::write(self.root.join("mode"), mode).unwrap();
    }
    fn prepared(&self) -> SettingsReview {
        self.client.settings_support().unwrap();
        let mut loaded = self
            .client
            .settings_context(
                &self.project,
                "stylua-vscode",
                &Cancellation::default(),
                |_| {},
            )
            .unwrap();
        loaded.draft.set(0, json!(true));
        self.client
            .prepare_settings(
                loaded.context,
                loaded.draft,
                &Cancellation::default(),
                |_| {},
            )
            .unwrap()
    }
    fn request(&self, review: SettingsReview) -> tempfile::TempDir {
        let ipc = tempfile::tempdir().unwrap();
        write_atomic(
            &ipc.path().join("request.json"),
            &SettingsRequest {
                protocol: SETTINGS_PROTOCOL,
                id: "settings-fixture".into(),
                project: self.project.clone(),
                tool: review.context.tool.clone(),
                action: SettingsAction::Save(review),
            },
        )
        .unwrap();
        ipc
    }
}

#[test]
fn preparation_is_read_only_and_success_returns_exact_baseline_without_setup() {
    let f = Fixture::new();
    let review = f.prepared();
    assert_eq!(fs::read_dir(&f.project).unwrap().count(), 0);
    let outcome = f.client.save_settings(review.clone(), |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.writes),
        (SettingsStatus::Succeeded, WriteState::Completed)
    );
    let saved = outcome.saved.unwrap();
    assert_eq!(saved.context.original, review.contents);
    assert!(!saved.draft.dirty());
    assert_eq!(saved.draft.current[0], Some(json!(true)));
}

#[test]
fn explicit_null_crosses_the_real_worker_boundary_as_a_present_value() {
    let f = Fixture::new();
    fs::create_dir(f.project.join(".vscode")).unwrap();
    fs::write(
        f.project.join(".vscode/settings.json"),
        "{\"editor.formatOnSave\":null,\"custom\":[1,2]}",
    )
    .unwrap();
    let loaded = f
        .client
        .settings_context(
            &f.project,
            "stylua-vscode",
            &Cancellation::default(),
            |_| {},
        )
        .unwrap();
    assert_eq!(loaded.draft.current[0], Some(serde_json::Value::Null));
    assert_eq!(loaded.draft.current[1], None);
    let review = f.prepared();
    assert_eq!(review.draft.current[0], Some(serde_json::Value::Null));
    assert_eq!(
        f.client.save_settings(review, |_| {}).unwrap().status,
        SettingsStatus::Succeeded
    );
}

#[test]
fn stale_files_and_tampered_output_never_start_writes() {
    let f = Fixture::new();
    let review = f.prepared();
    let mut changed = review.clone();
    changed.contents = Some("{}\n".into());
    let outcome = f.client.save_settings(changed, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.writes),
        (SettingsStatus::Failed, WriteState::NotStarted)
    );
    fs::create_dir(f.project.join(".vscode")).unwrap();
    fs::write(f.project.join(".vscode/settings.json"), "{\"custom\":true}").unwrap();
    let outcome = f.client.save_settings(review, |_| {}).unwrap();
    assert_eq!(outcome.writes, WriteState::NotStarted);
    assert_eq!(
        fs::read_to_string(f.project.join(".vscode/settings.json")).unwrap(),
        "{\"custom\":true}"
    );
}

#[test]
fn malformed_mismatched_missing_and_contradictory_reports_are_uncertain() {
    for mode in [
        "missing-report",
        "malformed-report",
        "mismatch-report",
        "exit-disagrees",
    ] {
        let f = Fixture::new();
        let review = f.prepared();
        f.mode(mode);
        let outcome = f.client.save_settings(review.clone(), |_| {}).unwrap();
        assert_eq!(
            (outcome.status, outcome.writes),
            (SettingsStatus::Uncertain, WriteState::Unknown)
        );
        assert!(outcome.saved.is_none());
    }
}

#[test]
fn old_worker_and_missing_executable_are_recoverable() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("old-worker");
    assert!(f.client.settings_support().is_err());
    fs::remove_file(f.root.join("worker.exe")).unwrap();
    let outcome = f.client.save_settings(review, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.writes),
        (SettingsStatus::Failed, WriteState::NotStarted)
    );
}

#[test]
fn preparation_cancellation_keeps_draft_but_missing_project_requires_reload() {
    let f = Fixture::new();
    let review = f.prepared();
    let token = Cancellation::default();
    token.cancel();
    let error = f
        .client
        .prepare_settings(review.context.clone(), review.draft.clone(), &token, |_| {})
        .unwrap_err();
    assert!(error.to_string().contains("cancelled"));
    assert!(!error.to_string().contains("Reload from file"));
    fs::remove_dir(&f.project).unwrap();
    let error = f
        .client
        .prepare_settings(
            review.context,
            review.draft,
            &Cancellation::default(),
            |_| {},
        )
        .unwrap_err();
    assert!(error.to_string().contains("Reload from file"));
    assert!(!f.project.exists());
}

#[test]
fn same_project_watch_blocks_save_and_other_project_watch_is_untouched() {
    let f = Fixture::new();
    let review = f.prepared();
    crate::background_watch::with_unresponsive_fixture(&f.project, || {
        let ipc = f.request(review.clone());
        assert!(job(ipc.path()).is_err());
        let report: SettingsReport = read_json(&ipc.path().join("report.json")).unwrap();
        assert_eq!(report.writes, WriteState::NotStarted);
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
fn completion_loss_after_replacement_keeps_the_result_uncertain() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("fail-after");
    let outcome = f.client.save_settings(review, |_| {}).unwrap();
    assert_eq!(
        (outcome.status, outcome.writes),
        (SettingsStatus::Uncertain, WriteState::MayHaveBegun)
    );
    assert!(f.project.join(".vscode/settings.json").exists());
    assert!(outcome.saved.is_none());
}

#[test]
fn normal_completion_drains_streams_and_cleans_up_descendants() {
    let f = Fixture::new();
    let review = f.prepared();
    f.mode("descendant");
    let started = std::time::Instant::now();
    let outcome = f.client.save_settings(review, |_| {}).unwrap();
    assert_eq!(outcome.status, SettingsStatus::Succeeded);
    assert!(started.elapsed() < std::time::Duration::from_secs(15));
    let pid = fs::read_to_string(f.root.join("descendant.pid")).unwrap();
    let listed = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid.trim()), "/NH"])
        .output()
        .unwrap();
    assert!(!String::from_utf8_lossy(&listed.stdout).contains("worker.exe"));
}

#[test]
fn oversized_context_fails_read_only_and_pre_cancelled_preparation_never_writes() {
    let f = Fixture::new();
    let token = Cancellation::default();
    token.cancel();
    assert!(
        f.client
            .settings_context(&f.project, "stylua", &token, |_| {})
            .is_err()
    );
    let text = format!("# {}\n", "x".repeat(IPC_LIMIT as usize));
    fs::write(f.project.join("stylua.toml"), &text).unwrap();
    let error = f
        .client
        .settings_context(&f.project, "stylua", &Cancellation::default(), |_| {})
        .unwrap_err();
    assert!(format!("{error:#}").contains("2 MiB"));
    assert_eq!(
        fs::read_to_string(f.project.join("stylua.toml")).unwrap(),
        text
    );
}

#[test]
fn oversized_save_and_invalid_target_are_rejected_before_spawn() {
    let f = Fixture::new();
    let mut review = f.prepared();
    review.contents = Some("x".repeat(IPC_LIMIT as usize));
    let outcome = f.client.save_settings(review, |_| {}).unwrap();
    assert_eq!(outcome.status, SettingsStatus::Failed);
    assert_eq!(outcome.writes, WriteState::NotStarted);
    assert!(outcome.message.unwrap().contains("2 MiB"));
    assert_eq!(fs::read_dir(&f.project).unwrap().count(), 0);
    assert!(
        f.client
            .settings_context(
                &f.project,
                "../other-file",
                &Cancellation::default(),
                |_| {}
            )
            .is_err()
    );
    assert_eq!(fs::read_dir(&f.project).unwrap().count(), 0);
}
