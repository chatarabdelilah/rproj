//! Versioned private IPC for the matching bundled CLI, not public commands.
use super::{Outcome, Plan, Step, StepState};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::creation::{
    composition_protocol::WriteState,
    protocol::{IPC_LIMIT, read_json, write_atomic},
};
pub const PROJECT_TOOL_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Prepare,
    Execute(Plan),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub tool: String,
    pub action: Action,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == PROJECT_TOOL_PROTOCOL
                && !self.id.is_empty()
                && self.id.len() <= 256
                && self.project.is_absolute(),
            "Unsupported project tool setup request"
        );
        super::rokit_target(&self.tool)?;
        if let Action::Execute(plan) = &self.action {
            ensure!(
                plan.project == self.project && plan.tool == self.tool,
                "Tool setup review belongs to another project/tool"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Succeeded,
    Failed,
    Cancelled,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub tool: String,
    pub effects: WriteState,
    pub status: Option<Status>,
    pub prepared: Option<Plan>,
    pub outcome: Option<Outcome>,
    pub message: Option<String>,
}
impl Report {
    pub fn initial(request: &Request) -> Self {
        Self {
            protocol: PROJECT_TOOL_PROTOCOL,
            id: request.id.clone(),
            project: request.project.clone(),
            tool: request.tool.clone(),
            effects: WriteState::NotStarted,
            status: None,
            prepared: None,
            outcome: None,
            message: None,
        }
    }
    pub fn check(&self, request: &Request) -> Result<()> {
        request.validate()?;
        ensure!(
            self.protocol == PROJECT_TOOL_PROTOCOL
                && self.id == request.id
                && self.project == request.project
                && self.tool == request.tool,
            "Tool setup report does not match this operation/project/tool"
        );
        let success = self.status == Some(Status::Succeeded);
        ensure!(
            !success || self.message.is_none(),
            "Successful tool setup contains an error"
        );
        ensure!(
            self.effects != WriteState::Completed || success,
            "Completed tool setup effects disagree with outcome"
        );
        match &request.action {
            Action::Prepare => {
                ensure!(
                    self.effects == WriteState::NotStarted && self.outcome.is_none(),
                    "Read-only tool preparation reported effects"
                );
                if success {
                    let plan = self
                        .prepared
                        .as_ref()
                        .context("Tool setup preparation returned no review")?;
                    ensure!(
                        plan.project == request.project && plan.tool == request.tool,
                        "Prepared tool setup belongs to another project/tool"
                    );
                } else {
                    ensure!(
                        self.prepared.is_none(),
                        "Unsuccessful preparation returned a review"
                    );
                }
            }
            Action::Execute(plan) => {
                ensure!(
                    self.prepared.is_none() && self.status != Some(Status::Cancelled),
                    "Tool setup execution has no preparation or cancellation outcome"
                );
                if let Some(outcome) = &self.outcome {
                    ensure!(
                        matches!(self.status, Some(Status::Succeeded | Status::Failed)),
                        "Unconfirmed tool setup returned confirmed step results"
                    );
                    ensure!(
                        outcome.project == self.project
                            && outcome.tool == self.tool
                            && outcome.steps.len() == plan.steps.len(),
                        "Tool setup step results do not match the reviewed operation"
                    );
                    for (actual, planned) in outcome.steps.iter().zip(&plan.steps) {
                        ensure!(
                            actual.label == planned.label(),
                            "Tool setup step identity changed"
                        );
                        let expected = match planned {
                            Step::PreserveFile { .. } | Step::PreservePin { .. } => {
                                StepState::AlreadyPresent
                            }
                            Step::Guidance { .. } => StepState::ManualAction,
                            _ => StepState::Completed,
                        };
                        ensure!(
                            actual.state == expected
                                || matches!(actual.state, StepState::Failed | StepState::Skipped),
                            "Tool setup step status contradicts the reviewed effect"
                        );
                    }
                    ensure!(
                        success == outcome.succeeded(),
                        "Tool setup success disagrees with required step results"
                    );
                    ensure!(
                        self.effects
                            == if outcome.effects_may_have_begun {
                                if success {
                                    WriteState::Completed
                                } else {
                                    WriteState::MayHaveBegun
                                }
                            } else {
                                WriteState::NotStarted
                            },
                        "Tool setup effects disagree with step results"
                    );
                    ensure!(
                        !success
                            || outcome.effects_may_have_begun
                                == plan.steps.iter().any(Step::mutates),
                        "Successful setup did not account for reviewed effects"
                    );
                } else {
                    ensure!(!success, "Successful tool setup returned no outcomes");
                    ensure!(
                        self.status != Some(Status::Failed)
                            || self.effects == WriteState::NotStarted,
                        "Failed setup with possible effects must retain step results or be uncertain"
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Completion {
    pub project: PathBuf,
    pub tool: String,
    pub status: Status,
    pub effects: WriteState,
    pub outcome: Option<Outcome>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests;
