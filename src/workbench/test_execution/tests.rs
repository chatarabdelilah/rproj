use super::*;
use crate::{
    config::{PackageWorkflow, project_file},
    graph::ProjectGraph,
};
use std::fs;

#[cfg(windows)]
mod process;

fn fixture() -> (tempfile::TempDir, tempfile::TempDir, Request) {
    let root = tempfile::Builder::new()
        .prefix("worker 保存 space ")
        .tempdir()
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut graph = ProjectGraph {
        package_workflow: PackageWorkflow::None,
        ..Default::default()
    };
    graph.choose("test", Some("testez"));
    project_file::save_to(&graph, root.path()).unwrap();
    fs::write(root.path().join("default.project.json"), r#"{"tree":{}}"#).unwrap();
    let request = Request {
        protocol: TEST_PROTOCOL,
        id: "operation".into(),
        project: root.path().canonicalize().unwrap(),
        action: Action::Context,
    };
    (root, directory, request)
}

#[test]
fn context_job_is_read_only_and_has_matching_report() {
    let (root, directory, request) = fixture();
    write_atomic(&directory.path().join("request.json"), &request).unwrap();
    job(directory.path()).unwrap();
    let report: Report = read_json(&directory.path().join("report.json")).unwrap();
    report.check(&request).unwrap();
    assert_eq!(report.status, Some(Status::Succeeded));
    assert!(!report.accepted);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn stale_execution_reports_refusal_without_acceptance() {
    let (root, directory, mut request) = fixture();
    let context = checked::prepare(root.path()).unwrap();
    fs::write(root.path().join("rokit.toml"), "[tools]").unwrap();
    request.action = Action::Run {
        context,
        arguments: vec![],
    };
    write_atomic(&directory.path().join("request.json"), &request).unwrap();
    #[cfg(windows)]
    {
        let watch = tempfile::tempdir().unwrap();
        crate::background_watch::with_test_root(watch.path(), || {
            assert!(job(directory.path()).is_err())
        });
    }
    #[cfg(not(windows))]
    assert!(job(directory.path()).is_err());
    let report: Report = read_json(&directory.path().join("report.json")).unwrap();
    report.check(&request).unwrap();
    assert_eq!(report.status, Some(Status::Failed));
    assert!(!report.accepted);
    assert_eq!(report.effects, WriteState::NotStarted);
}

#[test]
fn malformed_request_never_claims_a_successful_operation() {
    let (_, directory, _) = fixture();
    fs::write(directory.path().join("request.json"), "{").unwrap();
    assert!(job(directory.path()).is_err());
    assert!(!directory.path().join("report.json").exists());
}

#[test]
fn malformed_record_diagnostics_do_not_disclose_source_values() {
    let (root, directory, request) = fixture();
    fs::write(
        root.path().join("rproj.toml"),
        "packages = [DUMMY_SECRET_VALUE]",
    )
    .unwrap();
    write_atomic(&directory.path().join("request.json"), &request).unwrap();
    let error = job(directory.path()).unwrap_err();
    assert!(!format!("{error:#}").contains("DUMMY_SECRET_VALUE"));
    let text = fs::read_to_string(directory.path().join("report.json")).unwrap();
    assert!(!text.contains("DUMMY_SECRET_VALUE"));
    assert!(text.contains("Malformed rproj.toml"));
}
