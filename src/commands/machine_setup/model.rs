use crate::config::GlobalConfig;
pub use rproj_core::machine_setup::{Category, Selection};

pub trait SelectionConfig {
    fn load(config: &GlobalConfig) -> anyhow::Result<Selection>;
    fn to_config(&self, checked_at: String) -> GlobalConfig;
}
impl SelectionConfig for Selection {
    fn load(config: &GlobalConfig) -> anyhow::Result<Self> {
        Ok(Self {
            root: config.projects_root()?,
            apps: config.selected_system_apps.iter().cloned().collect(),
            tools: config.selected_rokit_tools.iter().cloned().collect(),
            plugins: config.selected_studio_plugins.iter().cloned().collect(),
            extensions: config.selected_vscode_extensions.iter().cloned().collect(),
        })
    }

    fn to_config(&self, checked_at: String) -> GlobalConfig {
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
    use crate::catalog::tool_catalog;
    use std::{collections::BTreeSet, path::PathBuf};

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
