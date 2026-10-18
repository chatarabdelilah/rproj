//! Private, versioned Upgrade handoff; never stored in project/preferences.
use super::UpgradeReview;
pub use crate::creation::composition_protocol::{CompositionStatus as UpgradeStatus, WriteState};
pub use crate::creation::protocol::{IPC_LIMIT, read_json, write_atomic};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const UPGRADE_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeContext {
    pub protocol: u32,
    pub project: PathBuf,
    pub name: String,
    pub identity: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum UpgradeAction {
    Prepare(UpgradeContext),
    Apply(UpgradeReview),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeRequest {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub action: UpgradeAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeReport {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub writes: WriteState,
    pub status: Option<UpgradeStatus>,
    pub prepared: Option<UpgradeReview>,
    pub message: Option<String>,
}
impl UpgradeReport {
    pub fn initial(request: &UpgradeRequest) -> Self {
        Self {
            protocol: UPGRADE_PROTOCOL,
            id: request.id.clone(),
            project: request.project.clone(),
            writes: WriteState::NotStarted,
            status: None,
            prepared: None,
            message: None,
        }
    }
    pub fn check(&self, request: &UpgradeRequest) -> Result<()> {
        ensure!(
            self.protocol == UPGRADE_PROTOCOL
                && self.id == request.id
                && self.project == request.project,
            "Upgrade report does not match this operation/project"
        );
        match &request.action {
            UpgradeAction::Prepare(context) => {
                ensure!(
                    self.writes == WriteState::NotStarted,
                    "Preparation returned a mutation state"
                );
                if self.status == Some(UpgradeStatus::Succeeded) {
                    let review = self
                        .prepared
                        .as_ref()
                        .ok_or_else(|| anyhow::anyhow!("Upgrade preparation returned no review"))?;
                    ensure!(
                        review.project == request.project && review.identity == context.identity,
                        "Upgrade preparation returned another project"
                    );
                    review.validate()?;
                } else {
                    ensure!(
                        self.prepared.is_none(),
                        "Failed preparation returned a review"
                    );
                }
            }
            UpgradeAction::Apply(review) => {
                ensure!(
                    self.prepared.is_none(),
                    "Apply returned a preparation payload"
                );
                if self.status == Some(UpgradeStatus::Succeeded) {
                    ensure!(
                        self.writes == WriteState::Completed
                            || (self.writes == WriteState::NotStarted && review.files.is_empty()),
                        "Upgrade success has no completed writes"
                    );
                }
            }
        }
        ensure!(
            self.writes != WriteState::Completed || self.status == Some(UpgradeStatus::Succeeded),
            "Completed writes have an inconsistent outcome"
        );
        ensure!(
            self.status != Some(UpgradeStatus::Succeeded) || self.message.is_none(),
            "Upgrade success contains an error"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpgradePreparation {
    Prepared(Box<UpgradeReview>),
    Failed(String),
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpgradeOutcome {
    pub project: PathBuf,
    pub status: UpgradeStatus,
    pub writes: WriteState,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> UpgradeRequest {
        UpgradeRequest {
            protocol: UPGRADE_PROTOCOL,
            id: "current".into(),
            project: std::env::temp_dir(),
            action: UpgradeAction::Prepare(UpgradeContext {
                protocol: UPGRADE_PROTOCOL,
                project: std::env::temp_dir(),
                name: "sample".into(),
                identity: "root".into(),
            }),
        }
    }
    #[test]
    fn reports_require_correlated_operation_project_and_read_only_preparation() {
        let request = request();
        let mut report = UpgradeReport::initial(&request);
        report.check(&request).unwrap();
        report.id = "stale".into();
        assert!(report.check(&request).is_err());
        report.id = request.id.clone();
        report.project.push("other");
        assert!(report.check(&request).is_err());
        report.project = request.project.clone();
        report.writes = WriteState::MayHaveBegun;
        assert!(report.check(&request).is_err());
        report.writes = WriteState::NotStarted;
        report.status = Some(UpgradeStatus::Succeeded);
        assert!(report.check(&request).is_err());
    }
    #[test]
    fn success_requires_completed_writes_and_no_error_or_preparation_payload() {
        let mut request = request();
        request.action = UpgradeAction::Apply(UpgradeReview {
            project: request.project.clone(),
            identity: "root".into(),
            originals: [("config".into(), None)].into(),
            files: vec![super::super::UpgradeFile {
                relative: "config".into(),
                contents: "new".into(),
                reason: "managed".into(),
                creating: true,
            }],
            skipped: vec![],
        });
        let mut report = UpgradeReport::initial(&request);
        report.status = Some(UpgradeStatus::Succeeded);
        assert!(report.check(&request).is_err());
        report.writes = WriteState::Completed;
        report.check(&request).unwrap();
        report.message = Some("error".into());
        assert!(report.check(&request).is_err());
        report.message = None;
        report.status = Some(UpgradeStatus::Failed);
        assert!(report.check(&request).is_err());
    }
}
