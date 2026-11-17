//! Shared single-project tool review and results; not a project file format.
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::catalog::tool_catalog::{self, ToolKind};

pub mod protocol;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    pub directory: bool,
    pub identity: Option<String>,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub project: PathBuf,
    pub tool: String,
    pub inputs: BTreeMap<String, Input>,
    pub steps: Vec<Step>,
    pub prerequisites: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    InitializeRokit,
    AddTool { source: String, pin: String },
    PreservePin { pin: String, value: String },
    CreateDirectory { relative: String },
    CreateFile { relative: String, contents: String },
    PreserveFile { relative: String },
    CreateScene,
    Guidance { message: String },
}

impl Step {
    pub fn label(&self) -> String {
        match self {
            Self::InitializeRokit => "Initialize rokit.toml".into(),
            Self::AddTool { source, .. } => {
                format!("Add {source}; trust source and download into shared tool cache")
            }
            Self::PreservePin { pin, value } => {
                format!("Preserve {pin} = {value} (pin presence is not installation verification)")
            }
            Self::CreateDirectory { relative } => format!("Create {relative}/"),
            Self::CreateFile { relative, .. } => format!("Create {relative}"),
            Self::PreserveFile { relative } => format!("Preserve {relative}"),
            Self::CreateScene => {
                "Create blender/scene.blend with Blender (binary starter scene)".into()
            }
            Self::Guidance { message } => message.clone(),
        }
    }
    pub fn mutates(&self) -> bool {
        matches!(
            self,
            Self::InitializeRokit
                | Self::AddTool { .. }
                | Self::CreateDirectory { .. }
                | Self::CreateFile { .. }
                | Self::CreateScene
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepState {
    Completed,
    AlreadyPresent,
    ManualAction,
    Failed,
    Skipped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepResult {
    pub label: String,
    pub state: StepState,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub project: PathBuf,
    pub tool: String,
    pub effects_may_have_begun: bool,
    pub steps: Vec<StepResult>,
}

impl Outcome {
    pub fn succeeded(&self) -> bool {
        self.steps
            .iter()
            .all(|step| !matches!(step.state, StepState::Failed | StepState::Skipped))
    }
}

/// Catalog keys distinguish editor integrations from executable pin names.
pub fn rokit_target(key: &str) -> Result<Option<(&'static str, &'static str)>> {
    let entry = tool_catalog::find(key).context("Unknown project tool; see rproj info")?;
    Ok(match entry.kind {
        ToolKind::RokitTool { rokit_source } => Some((
            rokit_source,
            match key {
                "luau-lsp-cli" => "luau-lsp",
                "jest-roblox" => "jest-roblox-cli",
                _ => entry.key,
            },
        )),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executable_identities_do_not_match_editor_extensions() {
        assert_eq!(rokit_target("luau-lsp-cli").unwrap().unwrap().1, "luau-lsp");
        assert_eq!(
            rokit_target("jest-roblox").unwrap().unwrap().1,
            "jest-roblox-cli"
        );
        assert!(rokit_target("luau-lsp").unwrap().is_none());
        for entry in tool_catalog::ROKIT_TOOLS {
            assert!(rokit_target(entry.key).unwrap().is_some());
        }
    }
    #[test]
    fn successful_file_creation_does_not_hide_tool_failure() {
        let outcome = Outcome {
            project: PathBuf::from("fixture"),
            tool: "asphalt".into(),
            effects_may_have_begun: true,
            steps: vec![
                StepResult {
                    label: "tool".into(),
                    state: StepState::Failed,
                    detail: String::new(),
                },
                StepResult {
                    label: "file".into(),
                    state: StepState::Completed,
                    detail: String::new(),
                },
            ],
        };
        assert!(!outcome.succeeded());
        assert_eq!(
            serde_json::from_str::<Outcome>(&serde_json::to_string(&outcome).unwrap()).unwrap(),
            outcome
        );
    }
}
