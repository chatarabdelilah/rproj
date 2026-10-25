//! Shared machine selections and deterministic execution order.
mod model;
#[cfg(test)]
mod tests;
use crate::catalog::tool_catalog;
pub use model::{Category, Selection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathIdentity {
    pub path: PathBuf,
    pub directory: bool,
    pub identity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupContext {
    pub config_path: PathBuf,
    pub original: Option<String>,
    pub saved: Selection,
    pub identities: Vec<PathIdentity>,
    pub detected: std::collections::BTreeMap<String, Detection>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupReview {
    pub context: SetupContext,
    pub selection: Selection,
    pub items: Vec<Item>,
}

impl SetupReview {
    pub fn new(context: SetupContext, selection: Selection) -> anyhow::Result<Self> {
        anyhow::ensure!(
            selection.root == context.saved.root,
            "The projects folder is read-only; reload Machine Setup"
        );
        for category in Category::ALL {
            let unknown = |s: &Selection| {
                s.keys(category)
                    .iter()
                    .filter(|key| {
                        !Category::ALL
                            .into_iter()
                            .filter(|other| category.shares_storage(*other))
                            .flat_map(Category::entries)
                            .any(|entry| entry.key == key.as_str())
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            };
            anyhow::ensure!(
                unknown(&selection) == unknown(&context.saved),
                "Unfamiliar saved choices must be retained without executing them"
            );
        }
        let items = plan(&selection);
        Ok(Self {
            context,
            selection,
            items,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Detection {
    Present,
    NotVerified(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Persistence {
    NotSaved,
    Saved { contents: String },
    Failed { message: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupResult {
    pub outcome: Outcome,
    pub statuses: Vec<Status>,
    pub changes_may_have_begun: bool,
    pub persistence: Persistence,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Item {
    Rokit,
    App(String),
    ProjectsFolder,
    Tool(String),
    Plugin(String),
    Extension(String),
}

impl Item {
    pub fn label(&self) -> &str {
        match self {
            Self::Rokit => "Rokit bootstrap",
            Self::ProjectsFolder => "Projects folder",
            Self::App(key) | Self::Tool(key) | Self::Plugin(key) | Self::Extension(key) => key,
        }
    }
    pub fn fatal(&self) -> bool {
        matches!(self, Self::Rokit | Self::ProjectsFolder)
    }
}

pub fn plan(selection: &Selection) -> Vec<Item> {
    let mut items = vec![Item::Rokit];
    for entry in tool_catalog::SYSTEM_APPS
        .iter()
        .filter(|entry| selection.apps.contains(entry.key))
    {
        items.push(Item::App(entry.key.into()));
    }
    items.push(Item::ProjectsFolder);
    for entry in tool_catalog::ROKIT_TOOLS
        .iter()
        .filter(|entry| selection.tools.contains(entry.key))
    {
        items.push(Item::Tool(entry.key.into()));
    }
    for category in [Category::Plugins, Category::Blender] {
        if selection.active(category) {
            for entry in tool_catalog::PLUGINS
                .iter()
                .filter(|entry| category.contains(entry) && selection.plugins.contains(entry.key))
            {
                items.push(Item::Plugin(entry.key.into()));
            }
        }
    }
    if selection.active(Category::Extensions) {
        for entry in tool_catalog::VSCODE_EXTENSIONS
            .iter()
            .filter(|entry| selection.extensions.contains(entry.key))
        {
            items.push(Item::Extension(entry.key.into()));
        }
    }
    items
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Status {
    Pending,
    Running,
    Already,
    Installed,
    Manual,
    Failed,
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Outcome {
    Completed,
    Warnings,
    Cancelled,
    Failed,
}
