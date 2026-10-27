//! Private, versioned IPC for the matching desktop/CLI distribution.
use super::*;
use crate::creation::CreationEnvironment;
pub use crate::creation::protocol::{IPC_LIMIT, read_json, write_atomic};

pub const SAVED_SETUPS_PROTOCOL: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupEntry {
    pub name: String,
    pub path: PathBuf,
    pub summary: String,
    pub warning: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupListing {
    pub root: PathBuf,
    pub entries: Vec<SetupEntry>,
    pub environment: Option<CreationEnvironment>,
    pub environment_problem: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadedSetup {
    pub source: CapturedSetup,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
    pub edit_problem: Option<String>,
    pub reuse_problem: Option<String>,
}
impl LoadedSetup {
    pub fn new(source: CapturedSetup) -> Self {
        let decoded = decode(&source.bytes);
        let edit_problem = decoded.graph.clone().map_or_else(
            || Some("Guided editing is unavailable for this document".into()),
            |graph| SavedSetupDraft::new(graph).err().map(|e| e.to_string()),
        );
        let reuse_problem = reusable_graph(&source.bytes).err().map(|e| e.to_string());
        Self {
            source,
            graph: decoded.graph,
            problem: decoded.problem,
            edit_problem,
            reuse_problem,
        }
    }
    pub fn validate(&self) -> Result<()> {
        validate_source(&self.source)?;
        ensure!(
            *self == Self::new(self.source.clone()),
            "Saved setup baseline does not match its bytes"
        );
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedSetup {
    pub review: ManagementReview,
}
impl PreparedSetup {
    pub fn new(review: ManagementReview) -> Result<Self> {
        review.validate()?;
        Ok(Self { review })
    }
    pub fn validate(&self) -> Result<()> {
        self.review.validate()
    }
    /// Compute off the UI thread, as with Upgrade and Configure Tools.
    pub fn rows(&self) -> Result<Vec<diff::Row>> {
        self.review
            .document
            .as_ref()
            .map(|d| d.diff(&format!("{}.toml", self.review.source.name)))
            .transpose()
            .map(Option::unwrap_or_default)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReusedSetup {
    pub source: CapturedSetup,
    pub graph: ProjectGraph,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedAction {
    Browse,
    Load(String),
    Prepare {
        source: CapturedSetup,
        action: SetupAction,
    },
    Execute(Box<ManagementReview>),
    Reuse(CapturedSetup),
}
impl SavedAction {
    pub fn source(&self) -> Option<&CapturedSetup> {
        match self {
            Self::Prepare { source, .. } | Self::Reuse(source) => Some(source),
            Self::Execute(review) => Some(&review.source),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRequest {
    pub protocol: u32,
    pub id: String,
    pub action: SavedAction,
}
fn validate_source(source: &CapturedSetup) -> Result<()> {
    crate::creation::validate_name(&source.name).map_err(anyhow::Error::msg)?;
    ensure!(
        source.root.is_absolute()
            && !source.storage_identity.is_empty()
            && !source.source_identity.is_empty(),
        "Invalid saved setup identity"
    );
    Ok(())
}
impl SavedRequest {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.protocol == SAVED_SETUPS_PROTOCOL && !self.id.is_empty() && self.id.len() <= 256,
            "Unsupported Saved Setups request"
        );
        if let Some(source) = self.action.source() {
            validate_source(source)?;
        }
        match &self.action {
            SavedAction::Load(name) => {
                crate::creation::validate_name(name).map_err(anyhow::Error::msg)?
            }
            SavedAction::Prepare { source, action } => {
                let document = if let SetupAction::Save(graph) = action {
                    Some(DocumentReview::prepare(&source.bytes, graph)?)
                } else {
                    None
                };
                let destination = match action {
                    SetupAction::Rename(n) | SetupAction::Duplicate(n) => {
                        Some(source.root.join(format!("{n}.toml")))
                    }
                    _ => None,
                };
                ManagementReview {
                    source: source.clone(),
                    action: action.clone(),
                    document,
                    destination,
                }
                .validate()?;
            }
            SavedAction::Execute(review) => review.validate()?,
            _ => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedReport {
    pub protocol: u32,
    pub id: String,
    pub status: Option<ManagementStatus>,
    pub mutation: MutationState,
    pub listing: Option<SetupListing>,
    pub loaded: Option<LoadedSetup>,
    pub prepared: Option<PreparedSetup>,
    pub reused: Option<ReusedSetup>,
    pub outcome: Option<ManagementOutcome>,
    pub message: Option<String>,
}
impl SavedReport {
    pub fn initial(request: &SavedRequest) -> Self {
        Self {
            protocol: SAVED_SETUPS_PROTOCOL,
            id: request.id.clone(),
            status: None,
            mutation: MutationState::NotStarted,
            listing: None,
            loaded: None,
            prepared: None,
            reused: None,
            outcome: None,
            message: None,
        }
    }
    pub fn check(&self, request: &SavedRequest) -> Result<()> {
        request.validate()?;
        ensure!(
            self.protocol == SAVED_SETUPS_PROTOCOL && self.id == request.id,
            "Saved Setups report belongs to another operation"
        );
        let success = self.status == Some(ManagementStatus::Succeeded);
        let count = [
            self.listing.is_some(),
            self.loaded.is_some(),
            self.prepared.is_some(),
            self.reused.is_some(),
            self.outcome.is_some(),
        ]
        .into_iter()
        .filter(|v| *v)
        .count();
        if let SavedAction::Execute(review) = &request.action {
            ensure!(
                count == usize::from(self.outcome.is_some()),
                "Mutation returned unrelated data"
            );
            if let Some(outcome) = &self.outcome {
                ensure!(
                    self.status == Some(outcome.status)
                        && self.mutation == outcome.mutation
                        && self.message == outcome.message,
                    "Contradictory mutation outcome"
                );
                ensure!(
                    outcome.source
                        == review
                            .source
                            .root
                            .join(format!("{}.toml", review.source.name))
                        && outcome.destination == review.destination,
                    "Outcome belongs to another setup"
                );
                if success {
                    let noop = matches!(&review.action, SetupAction::Rename(n) if n == &review.source.name)
                        || review
                            .document
                            .as_ref()
                            .is_some_and(|d| d.replacement.is_none());
                    ensure!(
                        self.mutation
                            == if noop {
                                MutationState::NotStarted
                            } else {
                                MutationState::Completed
                            },
                        "Success did not confirm mutation state"
                    );
                    if matches!(review.action, SetupAction::Delete) {
                        ensure!(outcome.saved.is_none(), "Deletion returned a baseline");
                    } else {
                        let saved = outcome
                            .saved
                            .as_ref()
                            .context("Success returned no saved baseline")?;
                        validate_source(saved)?;
                        let name = match &review.action {
                            SetupAction::Rename(n) | SetupAction::Duplicate(n) => n,
                            _ => &review.source.name,
                        };
                        let bytes = review
                            .document
                            .as_ref()
                            .and_then(|d| d.replacement.as_ref())
                            .map_or(review.source.bytes.as_slice(), |s| s.as_bytes());
                        ensure!(
                            saved.root == review.source.root
                                && saved.storage_identity == review.source.storage_identity
                                && &saved.name == name
                                && saved.bytes == bytes,
                            "Saved baseline differs from reviewed document"
                        );
                        if noop {
                            ensure!(saved == &review.source, "No-op changed the baseline");
                        }
                    }
                } else {
                    ensure!(
                        outcome.saved.is_none(),
                        "Unsuccessful mutation returned a baseline"
                    );
                }
            } else {
                ensure!(!success, "Success has no outcome");
            }
            ensure!(
                self.status != Some(ManagementStatus::Failed)
                    || self.mutation == MutationState::NotStarted,
                "Failure incorrectly claims known unchanged files"
            );
            ensure!(
                self.status != Some(ManagementStatus::Partial)
                    || (matches!(review.action, SetupAction::Rename(_))
                        && self.mutation == MutationState::Copied),
                "Partial result is not a copied Rename"
            );
        } else {
            ensure!(
                self.mutation == MutationState::NotStarted,
                "Read-only operation reported mutation"
            );
            ensure!(
                count == usize::from(success),
                "Read-only result has missing or unexpected data"
            );
            if success {
                match &request.action {
                    SavedAction::Browse => {
                        let listing = self.listing.as_ref().context("Missing setup listing")?;
                        ensure!(listing.root.is_absolute(), "Relative setup storage");
                        let mut names = std::collections::BTreeSet::new();
                        for entry in &listing.entries {
                            ensure!(
                                entry.path == listing.root.join(format!("{}.toml", entry.name))
                                    && names.insert(&entry.name),
                                "Invalid listing entry"
                            );
                        }
                    }
                    SavedAction::Load(name) => {
                        let loaded = self.loaded.as_ref().context("Missing saved document")?;
                        loaded.validate()?;
                        ensure!(&loaded.source.name == name, "Loaded another setup");
                    }
                    SavedAction::Prepare { source, action } => {
                        let prepared = self.prepared.as_ref().context("Missing prepared review")?;
                        prepared.validate()?;
                        ensure!(
                            &prepared.review.source == source && &prepared.review.action == action,
                            "Review differs from requested action"
                        );
                    }
                    SavedAction::Reuse(source) => {
                        let reused = self
                            .reused
                            .as_ref()
                            .context("Missing reusable composition")?;
                        ensure!(
                            &reused.source == source
                                && reused.graph == reusable_graph(&source.bytes)?,
                            "Reuse differs from saved composition"
                        );
                    }
                    SavedAction::Execute(_) => unreachable!(),
                }
            }
        }
        ensure!(
            !success || self.message.is_none(),
            "Success contains an error"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests;
