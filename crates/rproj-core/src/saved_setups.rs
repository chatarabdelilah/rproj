//! Pure saved-document interpretation and review; filesystem execution stays in the CLI.
use crate::{
    catalog::{artifacts, capabilities, wally_packages},
    creation::{CreationDraft, CreationEnvironment, PackageWorkflow, ProjectGraph},
    upgrade::diff,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub mod protocol;

pub struct DecodedDocument {
    pub parsed: Option<toml::Value>,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
}

pub fn decode(bytes: &[u8]) -> DecodedDocument {
    let parsed = std::str::from_utf8(bytes)
        .context("Setup is not UTF-8")
        .and_then(|text| {
            toml::from_str::<toml::Value>(text).map_err(|error| {
                anyhow::anyhow!(
                    "Malformed saved setup TOML near byte {}",
                    error.span().map_or(0, |span| span.start)
                )
            })
        });
    match parsed {
        Ok(value) => match value.clone().try_into::<ProjectGraph>() {
            Ok(graph) => DecodedDocument {
                parsed: Some(value),
                graph: Some(graph),
                problem: None,
            },
            Err(_) => DecodedDocument {
                parsed: Some(value),
                graph: None,
                problem: Some("Unsupported saved setup structure".into()),
            },
        },
        Err(error) => DecodedDocument {
            parsed: None,
            graph: None,
            problem: Some(format!("{error:#}")),
        },
    }
}

pub fn unsupported(graph: &ProjectGraph) -> bool {
    graph
        .packages
        .iter()
        .any(|key| wally_packages::find(key).is_none())
        || graph.capabilities.iter().any(|(key, value)| {
            capabilities::find(key)
                .is_none_or(|capability| capability.implementation(value).is_none())
        })
}

/// Metadata is deliberately not copied into the project graph. The source bytes
/// remain the replay identity; unknown composition choices still fail validation.
pub fn reusable_graph(bytes: &[u8]) -> Result<ProjectGraph> {
    let decoded = decode(bytes);
    let graph = decoded.graph.context(
        decoded
            .problem
            .unwrap_or_else(|| "Saved composition cannot be read".into()),
    )?;
    CreationDraft {
        name: "Validation".into(),
        graph: graph.clone(),
        save_setup: None,
    }
    .validate()?;
    Ok(graph)
}

pub fn validate_changes(original: &ProjectGraph, graph: &ProjectGraph) -> Result<()> {
    ensure!(
        original.mode == graph.mode,
        "Setup provenance must not change"
    );
    if unsupported(original) {
        ensure!(
            original == graph,
            "Unsupported choices are read-only; their editing consequences cannot be resolved safely."
        );
    } else {
        ensure!(
            !unsupported(graph),
            "Unsupported package or capability selection"
        );
    }
    ensure!(
        graph.testing_is_compatible(),
        "Jest Roblox requires Wally; select TestEZ or disable Testing."
    );
    for (key, implementation) in &graph.capabilities {
        if let Some(capability) = capabilities::find(key)
            && capability.implementation(implementation).is_some()
        {
            ensure!(
                capability
                    .implementations_for(graph.package_workflow)
                    .iter()
                    .any(|item| item.key == implementation),
                "{key}/{implementation} is incompatible with the selected dependencies"
            );
            ensure!(
                capability
                    .requires
                    .iter()
                    .all(|required| graph.capabilities.contains_key(*required)),
                "{key} requires {}",
                capability.requires.join(", ")
            );
        }
    }
    if graph.package_workflow == PackageWorkflow::None && !graph.packages.is_empty() {
        bail!("Dependency-free setups cannot contain packages");
    }
    for key in &graph.dropped {
        if !original.dropped.contains(key) {
            ensure!(
                artifacts::find(key).is_some_and(artifacts::Artifact::droppable),
                "Unknown or required file cannot be newly excluded: {key}"
            );
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentReview {
    pub original: Vec<u8>,
    pub graph: ProjectGraph,
    /// None means no write, including no formatting-only rewrite.
    pub replacement: Option<String>,
    pub changes: Vec<String>,
}

impl DocumentReview {
    pub fn prepare(bytes: &[u8], graph: &ProjectGraph) -> Result<Self> {
        let decoded = decode(bytes);
        let original = decoded
            .graph
            .context("Guided editing is unavailable for this document")?;
        validate_changes(&original, graph)?;
        let old = toml::Value::try_from(&original)?;
        let new = toml::Value::try_from(graph)?;
        let mut merged = decoded.parsed.context("Malformed setup TOML")?;
        let mut changes = Vec::new();
        for key in ["package_workflow", "packages", "dropped", "capabilities"] {
            if old.get(key) != new.get(key) {
                changes.push(format!("{key}: {} → {}", old[key], new[key]));
                merged
                    .as_table_mut()
                    .context("Expected a setup table")?
                    .insert(key.into(), new[key].clone());
            }
        }
        let replacement = if old == new {
            None
        } else {
            Some(toml::to_string_pretty(&merged)?)
        };
        Ok(Self {
            original: bytes.to_vec(),
            graph: graph.clone(),
            replacement,
            changes,
        })
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            *self == Self::prepare(&self.original, &self.graph)?,
            "Saved setup review does not match its captured document"
        );
        Ok(())
    }

    pub fn diff(&self, name: &str) -> Result<Vec<diff::Row>> {
        self.validate()?;
        Ok(match &self.replacement {
            Some(next) => diff::rows(Some(std::str::from_utf8(&self.original)?), next, name),
            None => vec![],
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSetupDraft {
    pub original: ProjectGraph,
    pub graph: ProjectGraph,
}

impl SavedSetupDraft {
    pub fn new(graph: ProjectGraph) -> Result<Self> {
        ensure!(!unsupported(&graph), "Unsupported choices are read-only");
        Ok(Self {
            original: graph.clone(),
            graph,
        })
    }
    pub fn dirty(&self) -> bool {
        self.graph != self.original
    }
    pub fn revert(&mut self) {
        self.graph = self.original.clone();
    }

    fn revise(&mut self, edit: impl FnOnce(&mut CreationDraft) -> Result<()>) -> Result<()> {
        let mut candidate = CreationDraft {
            name: "Validation".into(),
            graph: self.graph.clone(),
            save_setup: None,
        };
        edit(&mut candidate)?;
        // Creation invalidates file decisions; editing a reusable document must
        // retain even unfamiliar or presently inactive exclusions and provenance.
        candidate.graph.dropped = self.graph.dropped.clone();
        candidate.graph.mode = self.original.mode.clone();
        // Saved compositions historically record explicit packages, while
        // creation materializes capability packages later. Unrelated edits must
        // not insert TestEZ/Jest into an existing document as a side effect.
        candidate
            .graph
            .packages
            .retain(|key| !crate::creation::capability_owned(key));
        if candidate.graph.package_workflow == self.graph.package_workflow {
            candidate.graph.packages.extend(
                self.graph
                    .packages
                    .iter()
                    .filter(|key| crate::creation::capability_owned(key))
                    .cloned(),
            );
        }
        self.graph = candidate.graph;
        self.restore_original_order();
        Ok(())
    }

    fn restore_original_order(&mut self) {
        if self.graph.package_set() == self.original.package_set() {
            self.graph.packages = self.original.packages.clone();
        }
        let set = |items: &[String]| {
            items
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
        };
        if set(&self.graph.dropped) == set(&self.original.dropped) {
            self.graph.dropped = self.original.dropped.clone();
        }
    }

    /// Call only after frontend confirmation. Incompatible testing remains
    /// selected until an explicit implementation change repairs it.
    pub fn set_workflow(&mut self, workflow: PackageWorkflow) -> Result<()> {
        self.revise(|draft| {
            draft.set_workflow(workflow);
            Ok(())
        })
    }
    pub fn toggle_package(&mut self, key: &str, guided: bool) -> Result<()> {
        self.revise(|draft| draft.toggle_package(key, guided))
    }
    pub fn set_capability(&mut self, key: &str, implementation: Option<&str>) -> Result<()> {
        self.revise(|draft| draft.set_capability(key, implementation))
    }
    pub fn toggle_file(&mut self, key: &str, environment: &CreationEnvironment) -> Result<()> {
        let mut candidate = CreationDraft {
            name: "Validation".into(),
            graph: self.graph.clone(),
            save_setup: None,
        };
        candidate.toggle_file(key, environment)?;
        self.graph = candidate.graph;
        self.restore_original_order();
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        validate_changes(&self.original, &self.graph)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedSetup {
    pub root: PathBuf,
    pub name: String,
    pub storage_identity: String,
    pub source_identity: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetupAction {
    Save(ProjectGraph),
    Rename(String),
    Duplicate(String),
    Delete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementReview {
    pub source: CapturedSetup,
    pub action: SetupAction,
    pub document: Option<DocumentReview>,
    pub destination: Option<PathBuf>,
}

impl ManagementReview {
    pub fn validate(&self) -> Result<()> {
        crate::creation::validate_name(&self.source.name).map_err(anyhow::Error::msg)?;
        ensure!(
            self.source.root.is_absolute()
                && !self.source.storage_identity.is_empty()
                && !self.source.source_identity.is_empty(),
            "Invalid captured setup identity"
        );
        let (document, destination) = match &self.action {
            SetupAction::Save(graph) => (
                Some(DocumentReview::prepare(&self.source.bytes, graph)?),
                None,
            ),
            SetupAction::Rename(name) | SetupAction::Duplicate(name) => {
                crate::creation::validate_name(name).map_err(anyhow::Error::msg)?;
                if matches!(self.action, SetupAction::Rename(_)) {
                    ensure!(
                        name == &self.source.name
                            || name.to_lowercase() != self.source.name.to_lowercase(),
                        "Case-only rename is not supported; use an intermediate distinct name"
                    );
                }
                (None, Some(self.source.root.join(format!("{name}.toml"))))
            }
            SetupAction::Delete => (None, None),
        };
        ensure!(
            self.document == document && self.destination == destination,
            "Setup action does not match its authoritative review"
        );
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationState {
    Unknown,
    NotStarted,
    MayHaveBegun,
    Copied,
    Completed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManagementStatus {
    Succeeded,
    Failed,
    Partial,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagementOutcome {
    pub status: ManagementStatus,
    pub mutation: MutationState,
    pub source: PathBuf,
    pub destination: Option<PathBuf>,
    pub saved: Option<CapturedSetup>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests;
