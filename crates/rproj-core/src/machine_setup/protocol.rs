//! Private, versioned Machine Setup IPC. Logs are never completion evidence.
use super::{Outcome, Persistence, Selection, SetupContext, SetupResult, SetupReview, Status};
pub use crate::creation::composition_protocol::WriteState;
pub use crate::creation::protocol::{IPC_LIMIT, read_json};

/// Progress is sampled concurrently. Windows may briefly refuse replacement
/// while a reader releases the previous file; retry only transient IPC errors.
pub fn write_atomic<T: Serialize>(path: &std::path::Path, value: &T) -> Result<()> {
    for attempt in 0..8 {
        match crate::creation::protocol::write_atomic(path, value) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let transient = cfg!(windows)
                    && error.chain().any(|cause| {
                        cause
                            .downcast_ref::<std::io::Error>()
                            .is_some_and(|io| matches!(io.raw_os_error(), Some(5 | 32 | 33)))
                    });
                if !transient || attempt == 7 {
                    return Err(error.context("Cannot write Machine Setup IPC"));
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
        }
    }
    unreachable!()
}
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SETUP_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupAction {
    Context,
    Prepare {
        context: SetupContext,
        selection: Selection,
    },
    Apply(SetupReview),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupRequest {
    pub protocol: u32,
    pub id: String,
    pub action: SetupAction,
}
impl SetupRequest {
    pub fn machine(&self) -> Option<PathBuf> {
        match &self.action {
            SetupAction::Context => None,
            SetupAction::Prepare { context, .. } => Some(context.config_path.clone()),
            SetupAction::Apply(review) => Some(review.context.config_path.clone()),
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == SETUP_PROTOCOL && !self.id.is_empty() && self.id.len() <= 256,
            "Unsupported Machine Setup request"
        );
        let context = match &self.action {
            SetupAction::Context => return Ok(()),
            SetupAction::Prepare { context, selection } => {
                SetupReview::new(context.clone(), selection.clone())?;
                context
            }
            SetupAction::Apply(review) => {
                ensure!(
                    SetupReview::new(review.context.clone(), review.selection.clone())? == *review,
                    "Setup queue differs from catalog selections"
                );
                &review.context
            }
        };
        validate_context(context)
    }
}

fn validate_context(context: &SetupContext) -> Result<()> {
    ensure!(
        context.config_path.is_absolute()
            && context.saved.root.is_absolute()
            && !context.identities.is_empty()
            && context.identities.iter().all(|p| p.path.is_absolute()),
        "Setup context has invalid captured paths"
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupStatus {
    Succeeded,
    Failed,
    Stopped,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupReport {
    pub protocol: u32,
    pub id: String,
    pub machine: Option<PathBuf>,
    pub sequence: u64,
    pub effects: WriteState,
    pub stop_acknowledged: bool,
    pub statuses: Vec<Status>,
    pub active: Option<usize>,
    pub status: Option<SetupStatus>,
    pub loaded: Option<SetupContext>,
    pub prepared: Option<SetupReview>,
    pub result: Option<SetupResult>,
    pub message: Option<String>,
}
impl SetupReport {
    pub fn initial(request: &SetupRequest) -> Self {
        Self {
            protocol: SETUP_PROTOCOL,
            id: request.id.clone(),
            machine: request.machine(),
            sequence: 0,
            effects: WriteState::NotStarted,
            stop_acknowledged: false,
            statuses: match &request.action {
                SetupAction::Apply(review) => vec![Status::Pending; review.items.len()],
                _ => vec![],
            },
            active: None,
            status: None,
            loaded: None,
            prepared: None,
            result: None,
            message: None,
        }
    }
    pub fn check(&self, request: &SetupRequest) -> Result<()> {
        request.validate()?;
        ensure!(
            self.protocol == SETUP_PROTOCOL
                && self.id == request.id
                && self.machine == request.machine(),
            "Setup report belongs to a different operation or machine"
        );
        ensure!(
            self.active
                .is_none_or(|i| self.statuses.get(i) == Some(&Status::Running)),
            "Setup active item is inconsistent"
        );
        ensure!(
            self.status.is_none() || self.active.is_none(),
            "Terminal report still has an active installer"
        );
        match &request.action {
            SetupAction::Context => {
                ensure!(
                    self.prepared.is_none() && self.result.is_none(),
                    "Context returned unexpected setup data"
                );
                if self.status == Some(SetupStatus::Succeeded) {
                    validate_context(self.loaded.as_ref().context("Missing setup context")?)?;
                }
            }
            SetupAction::Prepare { context, selection } => {
                ensure!(
                    self.loaded.is_none() && self.result.is_none(),
                    "Preparation returned unexpected setup data"
                );
                if self.status == Some(SetupStatus::Succeeded) {
                    ensure!(
                        self.prepared.as_ref()
                            == Some(&SetupReview::new(context.clone(), selection.clone())?),
                        "Prepared setup differs from reviewed selections"
                    );
                }
            }
            SetupAction::Apply(review) => {
                ensure!(
                    self.loaded.is_none()
                        && self.prepared.is_none()
                        && self.statuses.len() == review.items.len(),
                    "Apply report has unexpected payload or items"
                );
                if let Some(result) = &self.result {
                    ensure!(
                        self.status.is_some()
                            && result.statuses == self.statuses
                            && self
                                .statuses
                                .iter()
                                .all(|s| !matches!(s, Status::Pending | Status::Running)),
                        "Setup result contains unfinished items"
                    );
                    ensure!(
                        result.changes_may_have_begun == (self.effects != WriteState::NotStarted),
                        "Setup effect flags disagree"
                    );
                    let expected = match (&result.outcome, &result.persistence) {
                        (Outcome::Completed | Outcome::Warnings, Persistence::Saved { .. }) => {
                            SetupStatus::Succeeded
                        }
                        (Outcome::Completed | Outcome::Warnings, Persistence::Failed { .. })
                        | (Outcome::Failed, Persistence::NotSaved) => SetupStatus::Failed,
                        (Outcome::Cancelled, Persistence::NotSaved) => SetupStatus::Stopped,
                        _ => anyhow::bail!("Setup outcome and persistence disagree"),
                    };
                    ensure!(
                        self.status == Some(expected),
                        "Setup terminal status disagrees with its result"
                    );
                    validate_result(review, result)?;
                    ensure!(
                        self.effects != WriteState::Unknown,
                        "Known setup result has unknown effects"
                    );
                    ensure!(
                        self.status != Some(SetupStatus::Succeeded)
                            || self.effects == WriteState::Completed,
                        "Successful setup has not confirmed persistence"
                    );
                } else {
                    ensure!(
                        !matches!(
                            self.status,
                            Some(SetupStatus::Succeeded | SetupStatus::Stopped)
                        ),
                        "Setup completion has no typed result"
                    );
                    ensure!(
                        self.status != Some(SetupStatus::Failed)
                            || self.effects == WriteState::NotStarted,
                        "A failed report cannot hide possible machine changes"
                    );
                }
            }
        }
        if !matches!(request.action, SetupAction::Apply(_)) {
            ensure!(
                self.effects == WriteState::NotStarted
                    && self.active.is_none()
                    && self.statuses.is_empty()
                    && !self.stop_acknowledged,
                "Read-only preparation reported installation effects"
            );
            ensure!(
                self.status != Some(SetupStatus::Stopped),
                "Read-only preparation cannot report installer Stop"
            );
        }
        if self.status != Some(SetupStatus::Succeeded) {
            ensure!(
                self.loaded.is_none() && self.prepared.is_none(),
                "Unsuccessful setup returned a context or review"
            );
        }
        ensure!(
            self.status != Some(SetupStatus::Succeeded) || self.message.is_none(),
            "Successful setup report contains an error"
        );
        ensure!(
            self.effects != WriteState::Completed || self.status == Some(SetupStatus::Succeeded),
            "Completed effects lack a successful result"
        );
        Ok(())
    }
}

fn validate_result(review: &SetupReview, result: &SetupResult) -> Result<()> {
    let fatal = review
        .items
        .iter()
        .zip(&result.statuses)
        .any(|(item, status)| item.fatal() && *status == Status::Failed);
    let warning = result
        .statuses
        .iter()
        .any(|s| matches!(s, Status::Manual | Status::Failed));
    let skipped = result.statuses.iter().position(|s| *s == Status::Skipped);
    ensure!(
        skipped.is_none_or(|i| result.statuses[i..].iter().all(|s| *s == Status::Skipped)),
        "Setup ran an item after skipping the queue"
    );
    ensure!(
        match result.outcome {
            Outcome::Completed => !fatal && !warning && skipped.is_none(),
            Outcome::Warnings => !fatal && warning && skipped.is_none(),
            Outcome::Cancelled => !fatal,
            Outcome::Failed => fatal,
        },
        "Setup outcome contradicts item results"
    );
    ensure!(
        result.changes_may_have_begun || result.statuses.iter().all(|s| *s == Status::Skipped),
        "Setup ran items without recording possible changes"
    );
    if let Persistence::Saved { contents } = &result.persistence {
        let saved: toml::Value =
            toml::from_str(contents).context("Invalid saved machine configuration")?;
        ensure!(
            saved
                .get("roblox_projects_root")
                .and_then(toml::Value::as_str)
                .is_some_and(|root| std::path::Path::new(root) == review.selection.root),
            "Saved projects folder differs from review"
        );
        for (key, expected) in [
            ("selected_system_apps", &review.selection.apps),
            ("selected_rokit_tools", &review.selection.tools),
            ("selected_studio_plugins", &review.selection.plugins),
            ("selected_vscode_extensions", &review.selection.extensions),
        ] {
            let values = saved
                .get(key)
                .and_then(toml::Value::as_array)
                .context("Saved selections missing")?;
            let actual = values
                .iter()
                .map(|v| v.as_str().context("Invalid saved choice"))
                .collect::<Result<std::collections::BTreeSet<_>>>()?;
            ensure!(
                actual == expected.iter().map(String::as_str).collect(),
                "Saved selections differ from review"
            );
        }
        ensure!(
            saved
                .get("last_checked")
                .and_then(toml::Value::as_str)
                .is_some_and(|s| !s.is_empty()),
            "Saved setup completion marker is missing"
        );
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopAfterItem {
    pub protocol: u32,
    pub id: String,
}
impl StopAfterItem {
    pub fn check(&self, request: &SetupRequest) -> Result<()> {
        ensure!(
            self.protocol == SETUP_PROTOCOL
                && self.id == request.id
                && matches!(request.action, SetupAction::Apply(_)),
            "Stop control does not match this setup operation"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests;
