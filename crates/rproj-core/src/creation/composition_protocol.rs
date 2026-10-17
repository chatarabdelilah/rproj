//! Private existing-project worker handoff. Never persisted in project/preferences.
pub use super::protocol::{IPC_LIMIT, read_json, write_atomic};
use super::{CompositionDraft, CompositionInput, CompositionPlan, ProjectGraph};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub const COMPOSITION_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionContext {
    pub protocol: u32,
    pub project: PathBuf,
    pub name: String,
    pub source: String,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
    pub inputs: Vec<CompositionInput>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareComposition {
    pub context: CompositionContext,
    pub draft: CompositionDraft,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum CompositionAction {
    Prepare(PrepareComposition),
    Apply(CompositionPlan),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionRequest {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub action: CompositionAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteState {
    NotStarted,
    MayHaveBegun,
    Completed,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositionStatus {
    Succeeded,
    Failed,
    Cancelled,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionReport {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub writes: WriteState,
    pub status: Option<CompositionStatus>,
    pub prepared: Option<CompositionPlan>,
    pub message: Option<String>,
}
impl CompositionReport {
    pub fn initial(request: &CompositionRequest) -> Self {
        Self {
            protocol: COMPOSITION_PROTOCOL,
            id: request.id.clone(),
            project: request.project.clone(),
            writes: WriteState::NotStarted,
            status: None,
            prepared: None,
            message: None,
        }
    }
    pub fn check(&self, request: &CompositionRequest) -> Result<()> {
        ensure!(
            self.protocol == COMPOSITION_PROTOCOL
                && self.id == request.id
                && self.project == request.project,
            "Composition report does not match this operation/project"
        );
        match &request.action {
            CompositionAction::Prepare(prepare) => {
                ensure!(
                    self.writes == WriteState::NotStarted,
                    "Preparation returned a mutation state"
                );
                if self.status == Some(CompositionStatus::Succeeded) {
                    ensure!(
                        self.prepared.as_ref().is_some_and(
                            |p| p.project == request.project && p.draft == prepare.draft
                        ),
                        "Preparation returned a mismatched or missing plan"
                    );
                } else {
                    ensure!(
                        self.prepared.is_none(),
                        "Failed preparation returned a plan"
                    );
                }
            }
            CompositionAction::Apply(plan) => {
                ensure!(
                    self.prepared.is_none(),
                    "Apply returned a preparation payload"
                );
                if self.status == Some(CompositionStatus::Succeeded) {
                    ensure!(
                        self.writes == WriteState::Completed
                            || (self.writes == WriteState::NotStarted
                                && plan.draft.graph == plan.draft.original),
                        "Apply reported success without a completed record"
                    );
                }
            }
        }
        ensure!(
            self.writes != WriteState::Completed
                || self.status == Some(CompositionStatus::Succeeded),
            "Completed mutation has an inconsistent outcome"
        );
        ensure!(
            self.status != Some(CompositionStatus::Succeeded) || self.message.is_none(),
            "Success report contains an error"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionPreparation {
    Prepared(Box<CompositionPlan>),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionOutcome {
    pub project: PathBuf,
    pub status: CompositionStatus,
    pub writes: WriteState,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> CompositionRequest {
        let draft = CompositionDraft::new(ProjectGraph::default());
        let project = PathBuf::from("captured");
        CompositionRequest {
            protocol: COMPOSITION_PROTOCOL,
            id: "one".into(),
            project: project.clone(),
            action: CompositionAction::Apply(CompositionPlan {
                project,
                draft,
                inputs: vec![],
                files: vec![],
                tools_to_add: vec![],
                external_operations: vec![],
                create_blender: false,
            }),
        }
    }

    #[test]
    fn reports_require_operation_project_and_mutation_agreement() {
        let mut request = request();
        if let CompositionAction::Apply(plan) = &mut request.action {
            plan.draft.graph.choose("format", None);
        }
        let mut report = CompositionReport::initial(&request);
        report.status = Some(CompositionStatus::Succeeded);
        assert!(report.check(&request).is_err());
        report.writes = WriteState::Completed;
        report.check(&request).unwrap();
        report.id = "old".into();
        assert!(report.check(&request).is_err());
        report.id = request.id.clone();
        report.project = PathBuf::from("other");
        assert!(report.check(&request).is_err());
    }

    #[test]
    fn preparation_cannot_claim_writes_and_reports_are_bounded() {
        let mut request = request();
        let context = CompositionContext {
            protocol: COMPOSITION_PROTOCOL,
            project: request.project.clone(),
            name: "captured".into(),
            source: String::new(),
            graph: Some(ProjectGraph::default()),
            problem: None,
            inputs: vec![],
        };
        request.action = CompositionAction::Prepare(PrepareComposition {
            context,
            draft: CompositionDraft::new(ProjectGraph::default()),
        });
        let mut report = CompositionReport::initial(&request);
        report.writes = WriteState::MayHaveBegun;
        assert!(report.check(&request).is_err());
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("report.json");
        report.message = Some("x".repeat(IPC_LIMIT as usize));
        assert!(write_atomic(&path, &report).is_err());
        assert!(!path.exists());
    }
}
