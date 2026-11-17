use super::*;
use crate::project_tool_setup::StepResult;

fn request() -> Request {
    let project = std::env::current_dir().unwrap();
    Request {
        protocol: PROJECT_TOOL_PROTOCOL,
        id: "one".into(),
        project: project.clone(),
        tool: "asphalt".into(),
        action: Action::Execute(Plan {
            project,
            tool: "asphalt".into(),
            inputs: Default::default(),
            steps: vec![Step::CreateFile {
                relative: "asphalt.toml".into(),
                contents: "example".into(),
            }],
            prerequisites: vec![],
            notes: vec![],
        }),
    }
}

fn completed(request: &Request) -> Report {
    let Action::Execute(plan) = &request.action else {
        unreachable!()
    };
    let mut report = Report::initial(request);
    report.status = Some(Status::Succeeded);
    report.effects = WriteState::Completed;
    report.outcome = Some(Outcome {
        project: request.project.clone(),
        tool: request.tool.clone(),
        effects_may_have_begun: true,
        steps: vec![StepResult {
            label: plan.steps[0].label(),
            state: StepState::Completed,
            detail: String::new(),
        }],
    });
    report
}

#[test]
fn report_correlates_operation_owner_and_each_reviewed_step() {
    let request = request();
    let valid = completed(&request);
    valid.check(&request).unwrap();
    let change: [fn(&mut Report); 8] = [
        |r| r.id.push('x'),
        |r| r.tool = "tungsten".into(),
        |r| r.project.push("other"),
        |r| r.effects = WriteState::NotStarted,
        |r| r.status = Some(Status::Cancelled),
        |r| r.outcome.as_mut().unwrap().steps.clear(),
        |r| r.outcome.as_mut().unwrap().steps[0].label = "other".into(),
        |r| r.outcome.as_mut().unwrap().steps[0].state = StepState::Failed,
    ];
    for mutate in change {
        let mut report = valid.clone();
        mutate(&mut report);
        assert!(report.check(&request).is_err());
    }
}

#[test]
fn known_failure_retains_partial_steps_but_uncertainty_cannot_claim_completion() {
    let request = request();
    let mut report = completed(&request);
    report.status = Some(Status::Failed);
    report.effects = WriteState::MayHaveBegun;
    report.outcome.as_mut().unwrap().steps[0].state = StepState::Failed;
    report.check(&request).unwrap();
    report.outcome = None;
    assert!(report.check(&request).is_err());
    report.status = Some(Status::Uncertain);
    report.check(&request).unwrap();
    report.effects = WriteState::Completed;
    assert!(report.check(&request).is_err());
}

#[test]
fn preparation_cannot_claim_effects_and_bounded_reports_are_atomic() {
    let mut request = request();
    let Action::Execute(plan) = request.action.clone() else {
        unreachable!()
    };
    request.action = Action::Prepare;
    let mut report = Report::initial(&request);
    report.status = Some(Status::Succeeded);
    report.prepared = Some(plan);
    report.check(&request).unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("report.json");
    write_atomic(&path, &report).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    report.effects = WriteState::MayHaveBegun;
    assert!(report.check(&request).is_err());
    report.message = Some("x".repeat(IPC_LIMIT as usize));
    assert!(write_atomic(&path, &report).is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
