//! Private IPC for matching desktop/CLI distributions.
use super::{review::*, values::parse_explorer_json};
pub use crate::creation::protocol::{IPC_LIMIT, read_json, write_atomic};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const TEMPLATE_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedTemplate {
    pub source: TemplateSource,
    pub problem: Option<String>,
}
impl LoadedTemplate {
    pub fn new(source: TemplateSource) -> Self {
        let problem = source.original.as_deref().and_then(|text| {
            parse_explorer_json(text)
                .err()
                .map(|error| format!("{error:#}"))
        });
        Self { source, problem }
    }
    pub fn validate(&self) -> Result<()> {
        self.source.validate()?;
        ensure!(
            *self == Self::new(self.source.clone()),
            "Template diagnosis differs from captured bytes"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalRojo {
    pub executable: PathBuf,
    /// OS identity and content digest, derived by the CLI.
    pub identity: String,
    pub manifest: TemplateSource,
}
impl GlobalRojo {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.executable.is_absolute() && !self.identity.is_empty(),
            "Missing captured global Rojo identity"
        );
        self.manifest.validate()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedTemplate {
    pub review: TemplateReview,
    pub rojo: Option<GlobalRojo>,
}
impl PreparedTemplate {
    pub fn needs_rojo(&self) -> bool {
        self.review.changes() && matches!(self.review.action, TemplateAction::Save(_))
    }
    pub fn validate(&self) -> Result<()> {
        self.review.validate()?;
        ensure!(
            self.rojo.is_some() == self.needs_rojo(),
            "Template review has inconsistent Rojo validation"
        );
        if let Some(rojo) = &self.rojo {
            rojo.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateJob {
    Context,
    Prepare {
        source: TemplateSource,
        action: TemplateAction,
    },
    Execute(Box<PreparedTemplate>),
}
impl TemplateJob {
    pub fn source(&self) -> Option<&TemplateSource> {
        match self {
            Self::Context => None,
            Self::Prepare { source, .. } => Some(source),
            Self::Execute(prepared) => Some(&prepared.review.source),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateRequest {
    pub protocol: u32,
    pub id: String,
    pub action: TemplateJob,
}
impl TemplateRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == TEMPLATE_PROTOCOL && !self.id.is_empty() && self.id.len() <= 256,
            "Unsupported Project Template request"
        );
        match &self.action {
            TemplateJob::Context => {}
            TemplateJob::Prepare { source, action } => {
                TemplateReview::prepare(source.clone(), action.clone())?;
            }
            TemplateJob::Execute(prepared) => prepared.validate()?,
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateStatus {
    Succeeded,
    Failed,
    Uncertain,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TemplateMutation {
    NotStarted,
    MayHaveBegun,
    Completed,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateReport {
    pub protocol: u32,
    pub id: String,
    pub path: Option<PathBuf>,
    pub status: Option<TemplateStatus>,
    pub mutation: TemplateMutation,
    pub loaded: Option<LoadedTemplate>,
    pub prepared: Option<PreparedTemplate>,
    pub saved: Option<TemplateSource>,
    pub message: Option<String>,
}
impl TemplateReport {
    pub fn initial(request: &TemplateRequest) -> Self {
        Self {
            protocol: TEMPLATE_PROTOCOL,
            id: request.id.clone(),
            path: request.action.source().map(|source| source.path.clone()),
            status: None,
            mutation: TemplateMutation::NotStarted,
            loaded: None,
            prepared: None,
            saved: None,
            message: None,
        }
    }
    pub fn check(&self, request: &TemplateRequest) -> Result<()> {
        request.validate()?;
        ensure!(
            self.protocol == TEMPLATE_PROTOCOL && self.id == request.id,
            "Template report belongs to another operation"
        );
        if let Some(source) = request.action.source() {
            ensure!(
                self.path.as_ref() == Some(&source.path),
                "Template report belongs to another source"
            );
        }
        let success = self.status == Some(TemplateStatus::Succeeded);
        let payloads = usize::from(self.loaded.is_some())
            + usize::from(self.prepared.is_some())
            + usize::from(self.saved.is_some());
        ensure!(
            payloads == usize::from(success),
            "Template report has inconsistent payloads"
        );
        ensure!(
            !success || self.message.is_none(),
            "Successful template report contains an error"
        );
        ensure!(
            self.mutation != TemplateMutation::Unknown
                || self.status == Some(TemplateStatus::Uncertain),
            "Unknown effects require uncertainty"
        );
        ensure!(
            self.mutation != TemplateMutation::Completed || success,
            "Completed template writes lack success"
        );
        ensure!(
            self.status != Some(TemplateStatus::Failed)
                || self.mutation == TemplateMutation::NotStarted,
            "Failure cannot hide possible template writes"
        );
        match &request.action {
            TemplateJob::Context => {
                ensure!(
                    self.mutation == TemplateMutation::NotStarted,
                    "Context cannot write a template"
                );
                if success {
                    let loaded = self.loaded.as_ref().context("Missing template context")?;
                    loaded.validate()?;
                    ensure!(
                        self.path.as_ref() == Some(&loaded.source.path),
                        "Context path mismatch"
                    );
                }
            }
            TemplateJob::Prepare { source, action } => {
                ensure!(
                    self.mutation == TemplateMutation::NotStarted,
                    "Preparation cannot write a template"
                );
                if success {
                    let prepared = self.prepared.as_ref().context("Missing template review")?;
                    prepared.validate()?;
                    ensure!(
                        prepared.review == TemplateReview::prepare(source.clone(), action.clone())?,
                        "Prepared template differs from reviewed edits"
                    );
                }
            }
            TemplateJob::Execute(prepared) if success => {
                let saved = self
                    .saved
                    .as_ref()
                    .context("Missing saved template baseline")?;
                saved.validate()?;
                let review = &prepared.review;
                ensure!(
                    saved.path == review.source.path,
                    "Saved template has another path"
                );
                if review.changes() {
                    ensure!(
                        self.mutation == TemplateMutation::Completed
                            && saved.original == review.replacement,
                        "Saved template bytes or effects differ from review"
                    );
                    ensure!(
                        saved.parents.values().all(Option::is_some)
                            && saved.parents.keys().eq(review.source.parents.keys())
                            && review
                                .source
                                .parents
                                .iter()
                                .all(|(path, id)| id.is_none()
                                    || saved.parents.get(path) == Some(id)),
                        "Captured template parent changed"
                    );
                } else {
                    ensure!(
                        self.mutation == TemplateMutation::NotStarted && saved == &review.source,
                        "No-op template changed its baseline"
                    );
                }
            }
            TemplateJob::Execute(_) => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplateOutcome {
    pub path: PathBuf,
    pub status: TemplateStatus,
    pub mutation: TemplateMutation,
    pub saved: Option<TemplateSource>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> TemplateSource {
        let path = std::env::temp_dir()
            .join("template-protocol-fixture")
            .join("default.project.json");
        TemplateSource {
            parents: path
                .ancestors()
                .skip(1)
                .map(|p| (p.to_owned(), Some("directory".into())))
                .collect(),
            path,
            original: Some("{broken".into()),
            identity: Some("old-file".into()),
        }
    }
    fn request(action: TemplateJob) -> TemplateRequest {
        TemplateRequest {
            protocol: TEMPLATE_PROTOCOL,
            id: "owned".into(),
            action,
        }
    }

    #[test]
    fn context_correlates_diagnosis_path_and_operation() {
        let source = source();
        let req = request(TemplateJob::Context);
        let mut report = TemplateReport::initial(&req);
        report.path = Some(source.path.clone());
        report.loaded = Some(LoadedTemplate::new(source));
        report.status = Some(TemplateStatus::Succeeded);
        report.check(&req).unwrap();
        report.id = "other".into();
        assert!(report.check(&req).is_err());
        report.id = req.id.clone();
        report.loaded.as_mut().unwrap().problem = None;
        assert!(report.check(&req).is_err());
    }

    #[test]
    fn reset_success_requires_absent_override_and_matching_parent_identities() {
        let source = source();
        let review = TemplateReview::prepare(source.clone(), TemplateAction::Reset).unwrap();
        let req = request(TemplateJob::Execute(Box::new(PreparedTemplate {
            review,
            rojo: None,
        })));
        let mut report = TemplateReport::initial(&req);
        report.status = Some(TemplateStatus::Succeeded);
        report.mutation = TemplateMutation::Completed;
        report.saved = Some(source);
        assert!(report.check(&req).is_err());
        let saved = report.saved.as_mut().unwrap();
        saved.original = None;
        saved.identity = None;
        report.check(&req).unwrap();
        *report
            .saved
            .as_mut()
            .unwrap()
            .parents
            .values_mut()
            .next()
            .unwrap() = None;
        assert!(report.check(&req).is_err());
        report.saved = None;
        report.status = Some(TemplateStatus::Failed);
        assert!(report.check(&req).is_err());
    }

    #[test]
    fn no_op_success_cannot_claim_writes_or_a_different_baseline() {
        let mut source = source();
        source.original = None;
        source.identity = None;
        let review = TemplateReview::prepare(source.clone(), TemplateAction::Reset).unwrap();
        let req = request(TemplateJob::Execute(Box::new(PreparedTemplate {
            review,
            rojo: None,
        })));
        let mut report = TemplateReport::initial(&req);
        report.saved = Some(source);
        report.status = Some(TemplateStatus::Succeeded);
        report.check(&req).unwrap();
        report.mutation = TemplateMutation::Completed;
        assert!(report.check(&req).is_err());
        report.mutation = TemplateMutation::NotStarted;
        report
            .saved
            .as_mut()
            .unwrap()
            .path
            .set_file_name("elsewhere");
        assert!(report.check(&req).is_err());
    }
}
