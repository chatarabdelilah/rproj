//! Versioned private settings IPC, not a project or preference format.
use super::{SettingsContext, SettingsDraft, SettingsReview};
pub use crate::creation::composition_protocol::WriteState;
pub use crate::creation::protocol::{IPC_LIMIT, read_json, write_atomic};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SETTINGS_PROTOCOL: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsLoaded {
    pub context: SettingsContext,
    pub draft: SettingsDraft,
    pub blocked: std::collections::BTreeMap<usize, String>,
}
impl SettingsLoaded {
    pub fn validate(&self) -> Result<()> {
        self.context.validate()?;
        self.draft.validate(self.context.target()?)?;
        ensure!(!self.draft.dirty(), "Loaded settings contain pending edits");
        ensure!(
            self.blocked.keys().all(|i| *i < self.draft.current.len()),
            "Blocked field is outside the settings catalog"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum SettingsAction {
    Context,
    Prepare {
        context: SettingsContext,
        draft: SettingsDraft,
    },
    Save(SettingsReview),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsRequest {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub tool: String,
    pub action: SettingsAction,
}
impl SettingsRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == SETTINGS_PROTOCOL
                && !self.id.is_empty()
                && self.id.len() <= 256
                && self.project.is_absolute(),
            "Unsupported settings request"
        );
        ensure!(
            crate::catalog::tool_settings::find(&self.tool).is_some(),
            "Unknown settings target"
        );
        let context = match &self.action {
            SettingsAction::Context => return Ok(()),
            SettingsAction::Prepare { context, draft } => {
                context.validate()?;
                draft.validate(context.target()?)?;
                context
            }
            SettingsAction::Save(review) => {
                review.validate()?;
                &review.context
            }
        };
        ensure!(
            context.project == self.project && context.tool == self.tool,
            "Settings request belongs to another project/target"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettingsStatus {
    Succeeded,
    Failed,
    Cancelled,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsReport {
    pub protocol: u32,
    pub id: String,
    pub project: PathBuf,
    pub tool: String,
    pub writes: WriteState,
    pub status: Option<SettingsStatus>,
    pub loaded: Option<SettingsLoaded>,
    pub prepared: Option<SettingsReview>,
    pub message: Option<String>,
}
impl SettingsReport {
    pub fn initial(request: &SettingsRequest) -> Self {
        Self {
            protocol: SETTINGS_PROTOCOL,
            id: request.id.clone(),
            project: request.project.clone(),
            tool: request.tool.clone(),
            writes: WriteState::NotStarted,
            status: None,
            loaded: None,
            prepared: None,
            message: None,
        }
    }
    pub fn check(&self, request: &SettingsRequest) -> Result<()> {
        request.validate()?;
        ensure!(
            self.protocol == SETTINGS_PROTOCOL
                && self.id == request.id
                && self.project == request.project
                && self.tool == request.tool,
            "Settings report does not match this operation/project/target"
        );
        let success = self.status == Some(SettingsStatus::Succeeded);
        ensure!(
            !success || self.message.is_none(),
            "Successful settings report contains an error"
        );
        ensure!(
            self.writes != WriteState::Completed || success,
            "Completed settings write has an inconsistent outcome"
        );
        if !matches!(request.action, SettingsAction::Save(_)) {
            ensure!(
                self.writes == WriteState::NotStarted,
                "Read-only settings operation reported writes"
            );
        } else {
            ensure!(
                self.status != Some(SettingsStatus::Cancelled),
                "Save has no cancellation outcome"
            );
        }
        if !success {
            ensure!(
                self.loaded.is_none() && self.prepared.is_none(),
                "Unsuccessful settings operation returned a baseline/review"
            );
            return Ok(());
        }
        match &request.action {
            SettingsAction::Prepare { context, draft } => {
                ensure!(
                    self.loaded.is_none(),
                    "Preparation returned an unexpected baseline"
                );
                let review = self
                    .prepared
                    .as_ref()
                    .context("Preparation returned no review")?;
                review.validate()?;
                ensure!(
                    &review.context == context && &review.draft == draft,
                    "Prepared settings do not match the requested edits"
                );
            }
            action => {
                ensure!(
                    self.prepared.is_none(),
                    "Settings operation returned an unexpected review"
                );
                let loaded = self
                    .loaded
                    .as_ref()
                    .context("Settings operation returned no baseline")?;
                loaded.validate()?;
                ensure!(
                    loaded.context.project == request.project
                        && loaded.context.tool == request.tool,
                    "Returned settings belong to another project/target"
                );
                if let SettingsAction::Save(review) = action {
                    let expected = review
                        .contents
                        .as_ref()
                        .or(review.context.original.as_ref());
                    ensure!(
                        loaded.context.original.as_ref() == expected,
                        "Save returned different bytes than reviewed"
                    );
                    ensure!(
                        self.writes
                            == if review.contents.is_some() {
                                WriteState::Completed
                            } else {
                                WriteState::NotStarted
                            },
                        "Save outcome does not confirm its write state"
                    );
                    ensure!(
                        loaded.context.identities.contains_key("")
                            && loaded.context.identities.get("")
                                == review.context.identities.get(""),
                        "Save returned another project identity"
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SettingsOutcome {
    pub project: PathBuf,
    pub tool: String,
    pub status: SettingsStatus,
    pub writes: WriteState,
    pub saved: Option<SettingsLoaded>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn prepared() -> SettingsRequest {
        let project = std::env::current_dir().unwrap();
        let context = SettingsContext {
            project: project.clone(),
            tool: "stylua-vscode".into(),
            identities: BTreeMap::from([
                ("".into(), Some("root".into())),
                (".vscode".into(), None),
                (".vscode/settings.json".into(), None),
            ]),
            original: None,
        };
        let mut draft = SettingsDraft::new(vec![None, None]);
        draft.set(0, json!(true));
        SettingsRequest {
            protocol: SETTINGS_PROTOCOL,
            id: "settings-operation".into(),
            project,
            tool: context.tool.clone(),
            action: SettingsAction::Prepare { context, draft },
        }
    }

    #[test]
    fn preparation_reports_must_keep_the_exact_captured_edits_and_identity() {
        let request = prepared();
        let SettingsAction::Prepare { context, draft } = &request.action else {
            unreachable!()
        };
        let mut report = SettingsReport::initial(&request);
        report.status = Some(SettingsStatus::Succeeded);
        report.prepared = Some(SettingsReview {
            context: context.clone(),
            draft: draft.clone(),
            contents: Some("{}".into()),
        });
        report.check(&request).unwrap();
        let original = report.clone();
        report.prepared.as_mut().unwrap().draft.set(0, json!(false));
        assert!(report.check(&request).is_err());
        report = original.clone();
        report.id.push_str("-late");
        assert!(report.check(&request).is_err());
        report = original.clone();
        report.tool = "luau-lsp".into();
        assert!(report.check(&request).is_err());
        report = original;
        report.writes = WriteState::MayHaveBegun;
        assert!(report.check(&request).is_err());
    }

    #[test]
    fn save_reports_require_matching_saved_bytes_and_confirmed_write_state() {
        let mut request = prepared();
        let SettingsAction::Prepare { context, draft } = request.action else {
            unreachable!()
        };
        let review = SettingsReview {
            context,
            draft,
            contents: Some("{}".into()),
        };
        let mut context = review.context.clone();
        context.original = review.contents.clone();
        context
            .identities
            .insert(".vscode".into(), Some("parent".into()));
        context
            .identities
            .insert(".vscode/settings.json".into(), Some("target".into()));
        request.action = SettingsAction::Save(review);
        let mut report = SettingsReport::initial(&request);
        report.status = Some(SettingsStatus::Succeeded);
        report.writes = WriteState::Completed;
        report.loaded = Some(SettingsLoaded {
            context,
            draft: SettingsDraft::new(vec![None, None]),
            blocked: BTreeMap::new(),
        });
        report.check(&request).unwrap();
        let original = report.clone();
        report
            .loaded
            .as_mut()
            .unwrap()
            .context
            .identities
            .remove("");
        assert!(report.check(&request).is_err());
        report = original.clone();
        report.loaded.as_mut().unwrap().context.original = Some("{\"unexpected\":1}".into());
        assert!(report.check(&request).is_err());
        report = original.clone();
        report.writes = WriteState::NotStarted;
        assert!(report.check(&request).is_err());
        report = original;
        report.loaded = None;
        report.writes = WriteState::MayHaveBegun;
        report.status = Some(SettingsStatus::Cancelled);
        assert!(report.check(&request).is_err());
        report.status = Some(SettingsStatus::Uncertain);
        report.check(&request).unwrap();
    }
}
