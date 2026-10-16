//! Private, versioned handoff between matching desktop and CLI distributions.
use super::{CreationDraft, CreationEnvironment, CreationPlan, ProjectGraph};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const CREATION_PROTOCOL: u32 = 1;
pub const IPC_LIMIT: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSnapshot {
    pub bytes: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedComposition {
    pub name: String,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
    pub source: SourceSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationContext {
    pub protocol: u32,
    pub environment: CreationEnvironment,
    pub machine_ready: bool,
    pub template: Option<serde_json::Value>,
    pub template_problem: Option<String>,
    pub config_source: SourceSnapshot,
    pub template_source: SourceSnapshot,
    pub setups: Vec<SavedComposition>,
    pub setups_problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationSources {
    pub config: SourceSnapshot,
    pub template: SourceSnapshot,
    pub selected_setup: Option<(String, SourceSnapshot)>,
}

impl CreationContext {
    pub fn sources(&self, setup: Option<&str>) -> Result<CreationSources> {
        ensure!(
            self.protocol == CREATION_PROTOCOL,
            "Unsupported creation protocol; use a matching portable distribution"
        );
        let selected_setup = setup
            .map(|name| {
                let entry = self
                    .setups
                    .iter()
                    .find(|s| s.name == name)
                    .context("Saved setup is no longer available; refresh before retrying")?;
                ensure!(
                    entry.problem.is_none() && entry.graph.is_some(),
                    "Saved setup is unsupported; inspect it with the existing Saved Setups workflow"
                );
                Ok((name.to_owned(), entry.source.clone()))
            })
            .transpose()?;
        Ok(CreationSources {
            config: self.config_source.clone(),
            template: self.template_source.clone(),
            selected_setup,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrepareCreation {
    pub draft: CreationDraft,
    pub sources: CreationSources,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCreation {
    pub plan: CreationPlan,
    pub sources: CreationSources,
    pub template: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparationOutcome {
    Prepared(Box<PreparedCreation>),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "payload")]
pub enum CreationAction {
    Prepare(PrepareCreation),
    Execute(PreparedCreation),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationRequest {
    pub protocol: u32,
    pub id: String,
    pub action: CreationAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimState {
    Unknown,
    NotClaimed,
    Claiming,
    Claimed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreationStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreationReport {
    pub protocol: u32,
    pub id: String,
    pub claim: ClaimState,
    pub status: Option<CreationStatus>,
    pub prepared: Option<PreparedCreation>,
    pub error: Option<String>,
}

impl CreationReport {
    pub fn initial(id: String) -> Self {
        Self {
            protocol: CREATION_PROTOCOL,
            id,
            claim: ClaimState::NotClaimed,
            status: None,
            prepared: None,
            error: None,
        }
    }
    pub fn check(&self, request: &CreationRequest) -> Result<()> {
        ensure!(
            self.protocol == CREATION_PROTOCOL && self.id == request.id,
            "Creation report does not match this operation"
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreationOutcome {
    pub destination: PathBuf,
    pub status: CreationStatus,
    pub claim: ClaimState,
    pub message: Option<String>,
}

pub fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("Cannot read {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(IPC_LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= IPC_LIMIT,
        "Creation data exceeded its size limit"
    );
    Ok(bytes)
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read_bounded(path)?).context("Invalid creation data")
}

pub fn write_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec(value)?;
    ensure!(
        bytes.len() as u64 <= IPC_LIMIT,
        "Creation data exceeded its size limit"
    );
    let mut pending =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing creation IPC directory")?)?;
    pending.write_all(&bytes)?;
    pending.as_file().sync_all()?;
    pending
        .persist(path)
        .map_err(|e| e.error)
        .context("Cannot replace creation report")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_replace_atomically_and_reject_other_operations_or_oversized_input() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json");
        let request = CreationRequest {
            protocol: 1,
            id: "one".into(),
            action: CreationAction::Prepare(PrepareCreation {
                draft: CreationDraft::new("Example".into()),
                sources: CreationSources {
                    config: SourceSnapshot { bytes: None },
                    template: SourceSnapshot { bytes: None },
                    selected_setup: None,
                },
            }),
        };
        let mut report = CreationReport::initial(request.id.clone());
        write_atomic(&path, &report).unwrap();
        report.claim = ClaimState::Claiming;
        write_atomic(&path, &report).unwrap();
        let loaded: CreationReport = read_json(&path).unwrap();
        loaded.check(&request).unwrap();
        assert_eq!(loaded.claim, ClaimState::Claiming);
        report.id = "other".into();
        assert!(report.check(&request).is_err());
        fs::write(&path, vec![b' '; IPC_LIMIT as usize + 1]).unwrap();
        assert!(read_json::<CreationReport>(&path).is_err());
    }
}
