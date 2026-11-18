use super::*;
use crate::creation::{
    PackageWorkflow,
    graph::{JestBackend, TestRunner},
};
fn request() -> Request {
    let project = std::env::current_dir().unwrap();
    Request {
        protocol: TEST_PROTOCOL,
        id: "one".into(),
        project: project.clone(),
        action: Action::Run {
            context: TestContext {
                project,
                name: "Fixture".into(),
                runner: TestRunner::TestEz,
                backend: JestBackend::Studio,
                workflow: PackageWorkflow::None,
                inputs: Default::default(),
            },
            arguments: vec!["literal\r\n保存".into()],
        },
    }
}
#[test]
fn missing_malformed_mismatched_and_contradictory_reports_are_uncertain() {
    let request = request();
    let missing = resolve_report(
        &request,
        &OperationOutcome::Succeeded,
        Err(anyhow::anyhow!("missing")),
    )
    .unwrap();
    assert_eq!(missing.status, Some(Status::Uncertain));
    assert_eq!(missing.effects, WriteState::Unknown);
    let mut wrong = Report::initial(&request).unwrap();
    wrong.id = "other".into();
    assert_eq!(
        resolve_report(
            &request,
            &OperationOutcome::Failed { code: Some(1) },
            Ok(wrong)
        )
        .unwrap()
        .status,
        Some(Status::Uncertain)
    );
    let mut report = Report::initial(&request).unwrap();
    report.status = Some(Status::Failed);
    assert_eq!(
        resolve_report(&request, &OperationOutcome::Succeeded, Ok(report))
            .unwrap()
            .status,
        Some(Status::Uncertain)
    );
    let malformed = serde_json::from_str::<Report>("{").map_err(anyhow::Error::from);
    assert_eq!(
        resolve_report(&request, &OperationOutcome::Cancelled, malformed)
            .unwrap()
            .status,
        Some(Status::Uncertain)
    );
}
#[test]
fn cancelled_owned_exit_retains_acknowledgement_and_confirmed_success_wins_race() {
    let request = request();
    let mut report = Report::initial(&request).unwrap();
    report.accepted = true;
    report.effects = WriteState::MayHaveBegun;
    let cancelled =
        resolve_report(&request, &OperationOutcome::Cancelled, Ok(report.clone())).unwrap();
    assert_eq!(cancelled.status, Some(Status::Cancelled));
    assert!(cancelled.accepted);
    assert_eq!(cancelled.effects, WriteState::MayHaveBegun);
    report.status = Some(Status::Succeeded);
    report.effects = WriteState::Completed;
    report.runner_exit = Some(0);
    assert_eq!(
        resolve_report(&request, &OperationOutcome::Cancelled, Ok(report))
            .unwrap()
            .status,
        Some(Status::Succeeded)
    );
}
#[test]
fn pre_start_refusal_preserves_unsubmitted_state_and_real_exit_code() {
    let request = request();
    let mut report = Report::initial(&request).unwrap();
    report.status = Some(Status::Failed);
    report.message = Some("Refresh".into());
    let refused = resolve_report(
        &request,
        &OperationOutcome::Failed { code: Some(1) },
        Ok(report),
    )
    .unwrap();
    assert!(!refused.accepted);
    assert_eq!(refused.effects, WriteState::NotStarted);
    let mut report = Report::initial(&request).unwrap();
    report.accepted = true;
    report.effects = WriteState::MayHaveBegun;
    report.status = Some(Status::Failed);
    report.runner_exit = Some(7);
    assert_eq!(
        resolve_report(
            &request,
            &OperationOutcome::Failed { code: Some(1) },
            Ok(report)
        )
        .unwrap()
        .runner_exit,
        Some(7)
    );
}

#[test]
fn missing_read_only_context_report_does_not_warn_about_mutations() {
    let mut request = request();
    request.action = Action::Context;
    let report = resolve_report(
        &request,
        &OperationOutcome::Succeeded,
        Err(anyhow::anyhow!("missing")),
    )
    .unwrap();
    assert_eq!(report.effects, WriteState::NotStarted);
    let message = report.message.unwrap();
    assert!(message.contains("read-only"));
    assert!(!message.contains("changes may remain"));
}
