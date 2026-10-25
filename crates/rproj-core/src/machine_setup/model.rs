use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::catalog::tool_catalog::{self, ToolEntry, ToolKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Apps,
    Tools,
    Plugins,
    Blender,
    Extensions,
    Themes,
}

impl Category {
    pub const ALL: [Self; 6] = [
        Self::Apps,
        Self::Tools,
        Self::Plugins,
        Self::Blender,
        Self::Extensions,
        Self::Themes,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Apps => "System Apps",
            Self::Tools => "CLI Tools",
            Self::Plugins => "Studio Plugins",
            Self::Blender => "Blender Add-ons",
            Self::Extensions => "VS Code Extensions",
            Self::Themes => "VS Code Themes & Icons",
        }
    }

    pub fn contains(self, entry: &ToolEntry) -> bool {
        match entry.kind {
            ToolKind::SystemApp { .. } => self == Self::Apps,
            ToolKind::RokitTool { .. } => self == Self::Tools,
            ToolKind::BlenderAddon { .. } => self == Self::Blender,
            ToolKind::VsCodeExtension { .. } => {
                self == if entry.key.starts_with("theme-") {
                    Self::Themes
                } else {
                    Self::Extensions
                }
            }
            _ => self == Self::Plugins,
        }
    }

    pub fn entries(self) -> Vec<&'static ToolEntry> {
        let mut entries: Vec<_> = tool_catalog::all_setup_entries()
            .filter(|e| self.contains(e))
            .collect();
        entries.sort_by_key(|e| e.key);
        entries
    }

    pub fn shares_storage(self, other: Self) -> bool {
        self == other
            || matches!(
                (self, other),
                (Self::Plugins | Self::Blender, Self::Plugins | Self::Blender)
                    | (
                        Self::Extensions | Self::Themes,
                        Self::Extensions | Self::Themes
                    )
            )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    pub root: PathBuf,
    pub apps: BTreeSet<String>,
    pub tools: BTreeSet<String>,
    pub plugins: BTreeSet<String>,
    pub extensions: BTreeSet<String>,
}

impl Selection {
    pub fn keys(&self, category: Category) -> &BTreeSet<String> {
        match category {
            Category::Apps => &self.apps,
            Category::Tools => &self.tools,
            Category::Plugins | Category::Blender => &self.plugins,
            Category::Extensions | Category::Themes => &self.extensions,
        }
    }

    pub fn replace_category(&mut self, category: Category, checked: &BTreeSet<String>) {
        let keys = match category {
            Category::Apps => &mut self.apps,
            Category::Tools => &mut self.tools,
            Category::Plugins | Category::Blender => &mut self.plugins,
            Category::Extensions | Category::Themes => &mut self.extensions,
        };
        // Only this category's known keys are editable; future keys survive.
        for entry in category.entries() {
            if checked.contains(entry.key) {
                keys.insert(entry.key.into());
            } else {
                keys.remove(entry.key);
            }
        }
    }

    pub fn active(&self, category: Category) -> bool {
        match category {
            Category::Blender => self.apps.contains("blender"),
            Category::Extensions | Category::Themes => self.apps.contains("vscode"),
            _ => true,
        }
    }
}
