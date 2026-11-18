//! Private, bounded IPC for custom test execution by the matching bundled CLI.
use super::{Invocation, TestContext};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use crate::creation::{
    composition_protocol::WriteState,
    protocol::{IPC_LIMIT, read_json, write_atomic},
};
pub const TEST_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Context,
    Run {
        context: TestContext,
        arguments: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub action: Action,
}
impl Request {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == TEST_PROTOCOL
                && !self.id.is_empty()
                && self.id.len() <= 256
                && self.project.is_absolute(),
            "Unsupported test request"
        );
        if let Action::Run { context, arguments } = &self.action {
            ensure!(
                context.project == self.project,
                "Test context belongs to another project"
            );
            context.invocation(arguments)?;
        }
        ensure!(
            serde_json::to_vec(self)?.len() as u64 <= IPC_LIMIT,
            "Test request exceeds the 2 MiB worker limit"
        );
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
    pub invocation: Option<Invocation>,
    pub accepted: bool,
    pub effects: WriteState,
    pub status: Option<Status>,
    pub context: Option<TestContext>,
    pub runner_exit: Option<i32>,
    pub message: Option<String>,
}
impl Report {
    pub fn initial(request: &Request) -> Result<Self> {
        request.validate()?;
        Ok(Self {
            protocol: TEST_PROTOCOL,
            id: request.id.clone(),
            project: request.project.clone(),
            invocation: match &request.action {
                Action::Context => None,
                Action::Run { context, arguments } => Some(context.invocation(arguments)?),
            },
            accepted: false,
            effects: WriteState::NotStarted,
            status: None,
            context: None,
            runner_exit: None,
            message: None,
        })
    }
    pub fn check(&self, request: &Request) -> Result<()> {
        let expected = Self::initial(request)?;
        ensure!(
            self.protocol == expected.protocol
                && self.id == expected.id
                && self.project == expected.project
                && self.invocation == expected.invocation,
            "Test report does not match this operation/project/invocation"
        );
        let success = self.status == Some(Status::Succeeded);
        ensure!(
            !success || self.message.is_none(),
            "Successful test report contains an error"
        );
        ensure!(
            self.effects != WriteState::Completed || success,
            "Test effects contradict completion"
        );
        ensure!(
            self.accepted || self.effects == WriteState::NotStarted,
            "Unaccepted test reported effects"
        );
        match &request.action {
            Action::Context => {
                ensure!(
                    !self.accepted
                        && self.effects == WriteState::NotStarted
                        && self.runner_exit.is_none(),
                    "Read-only test context reported execution"
                );
                if success {
                    ensure!(
                        self.context
                            .as_ref()
                            .context("Missing test context")?
                            .project
                            == request.project,
                        "Test context project mismatch"
                    );
                } else {
                    ensure!(
                        self.context.is_none(),
                        "Unsuccessful context returned a result"
                    );
                }
            }
            Action::Run { .. } => {
                ensure!(
                    self.context.is_none(),
                    "Test execution returned an unexpected context"
                );
                ensure!(
                    !success
                        || self.accepted
                            && self.effects == WriteState::Completed
                            && self.runner_exit == Some(0),
                    "Successful test was not accepted/completed"
                );
                if let Some(code) = self.runner_exit {
                    ensure!(
                        self.accepted
                            && (code == 0 && success
                                || code != 0 && self.status == Some(Status::Failed)),
                        "Runner exit contradicts the test report"
                    );
                }
            }
        }
        ensure!(
            serde_json::to_vec(self)?.len() as u64 <= IPC_LIMIT,
            "Test report exceeds the 2 MiB worker limit"
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Completion {
    pub project: PathBuf,
    /// None means no trustworthy acknowledgement survived; never imply no execution.
    pub accepted: Option<bool>,
    pub status: Status,
    pub effects: WriteState,
    pub runner_exit: Option<i32>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creation::{
        PackageWorkflow,
        graph::{JestBackend, TestRunner},
    };
    fn request() -> Request {
        let project = std::env::current_dir().unwrap();
        Request {
            protocol: TEST_PROTOCOL,
            id: "test".into(),
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
                arguments: vec!["exact\r\n".into()],
            },
        }
    }
    #[test]
    fn report_identity_acceptance_and_outcome_are_checked() {
        let request = request();
        let report = Report::initial(&request).unwrap();
        report.check(&request).unwrap();
        let mut wrong = report.clone();
        wrong.id = "other".into();
        assert!(wrong.check(&request).is_err());
        let mut wrong = report.clone();
        wrong
            .invocation
            .as_mut()
            .unwrap()
            .arguments
            .push("extra".into());
        assert!(wrong.check(&request).is_err());
        let mut wrong = report.clone();
        wrong.status = Some(Status::Succeeded);
        assert!(wrong.check(&request).is_err());
        wrong.accepted = true;
        wrong.effects = WriteState::Completed;
        wrong.runner_exit = Some(0);
        wrong.check(&request).unwrap();
        wrong.runner_exit = Some(7);
        assert!(wrong.check(&request).is_err());
        wrong.status = Some(Status::Failed);
        wrong.effects = WriteState::MayHaveBegun;
        wrong.check(&request).unwrap();
    }
    #[test]
    fn mismatched_or_oversized_requests_are_refused() {
        let mut request = request();
        request.protocol += 1;
        assert!(request.validate().is_err());
        let mut request = self::request();
        request.project.push("other");
        assert!(request.validate().is_err());
        let mut request = self::request();
        if let Action::Run { context, .. } = &mut request.action {
            context.name = "x".repeat(2 * 1024 * 1024);
        }
        assert!(request.validate().is_err());
    }
}
