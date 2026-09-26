use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::catalog::tool_catalog::{self, ToolEntry, ToolKind};
use crate::config::GlobalConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub root: PathBuf,
    pub apps: BTreeSet<String>,
    pub tools: BTreeSet<String>,
    pub plugins: BTreeSet<String>,
    pub extensions: BTreeSet<String>,
}

impl Selection {
    pub fn load(config: &GlobalConfig) -> anyhow::Result<Self> {
        Ok(Self {
            root: config.projects_root()?,
            apps: config.selected_system_apps.iter().cloned().collect(),
            tools: config.selected_rokit_tools.iter().cloned().collect(),
            plugins: config.selected_studio_plugins.iter().cloned().collect(),
            extensions: config.selected_vscode_extensions.iter().cloned().collect(),
        })
    }

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

    pub fn to_config(&self, checked_at: String) -> GlobalConfig {
        GlobalConfig {
            roblox_projects_root: Some(self.root.clone()),
            selected_system_apps: self.apps.iter().cloned().collect(),
            selected_rokit_tools: self.tools.iter().cloned().collect(),
            selected_studio_plugins: self.plugins.iter().cloned().collect(),
            selected_vscode_extensions: self.extensions.iter().cloned().collect(),
            last_checked: Some(checked_at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> GlobalConfig {
        GlobalConfig {
            roblox_projects_root: Some(PathBuf::from("fixture-projects")),
            ..GlobalConfig::default()
        }
    }

    #[test]
    fn fresh_setup_starts_empty_and_saved_choices_are_preserved() {
        let mut config = config();
        let first = Selection::load(&config).unwrap();
        assert!(first.apps.is_empty());
        assert!(first.tools.is_empty());
        assert!(first.plugins.is_empty());
        assert!(first.extensions.is_empty());
        config.selected_system_apps = vec!["studio".into()];
        config.selected_rokit_tools = vec!["rojo".into()];
        config.selected_studio_plugins = vec!["rojo-plugin".into()];
        config.selected_vscode_extensions = vec!["luau-lsp".into()];
        let saved = Selection::load(&config).unwrap();
        assert_eq!(saved.apps, BTreeSet::from(["studio".into()]));
        assert_eq!(saved.tools, BTreeSet::from(["rojo".into()]));
        assert_eq!(saved.plugins, BTreeSet::from(["rojo-plugin".into()]));
        assert_eq!(saved.extensions, BTreeSet::from(["luau-lsp".into()]));
        config.last_checked = Some("1".into());
        assert_eq!(Selection::load(&config).unwrap(), saved);
    }

    #[test]
    fn edits_preserve_unknown_and_other_category_choices() {
        let mut config = config();
        config.last_checked = Some("1".into());
        config.selected_studio_plugins = vec![
            "future-plugin".into(),
            "blender-plugin".into(),
            "rojo-plugin".into(),
        ];
        let mut selection = Selection::load(&config).unwrap();
        selection.replace_category(Category::Plugins, &BTreeSet::new());
        assert_eq!(
            selection.plugins,
            BTreeSet::from(["future-plugin".into(), "blender-plugin".into()])
        );
        assert!(!selection.active(Category::Blender));
        selection.apps.insert("blender".into());
        assert!(selection.active(Category::Blender));
    }

    #[test]
    fn catalog_categories_partition_every_entry() {
        for entry in tool_catalog::all_setup_entries() {
            assert_eq!(
                Category::ALL
                    .iter()
                    .filter(|category| category.contains(entry))
                    .count(),
                1,
                "{}",
                entry.key
            );
        }
    }
}
