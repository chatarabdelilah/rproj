//! Frozen before extracting the terminal setup model/catalog.
use super::{
    model::{Category, Selection, SelectionConfig},
    worker,
};
use crate::{
    catalog::tool_catalog::{self, ToolKind},
    config::GlobalConfig,
};

#[test]
fn original_setup_catalog_fixture() {
    let actual: Vec<_> = tool_catalog::all_setup_entries().map(|e| {
        let (kind, source, manual) = match e.kind {
            ToolKind::SystemApp { winget_id, .. } => ("SystemApp", winget_id, ""),
            ToolKind::RokitTool { rokit_source } => ("RokitTool", rokit_source, ""),
            ToolKind::VsCodeExtension { extension_id } => ("VsCodeExtension", extension_id, ""),
            ToolKind::StudioPlugin { github_repo, .. } => ("StudioPlugin", github_repo, ""),
            ToolKind::StudioPluginManual { github_repo, install_url } => ("StudioPluginManual", github_repo, install_url),
            ToolKind::StudioPluginViaCli { github_repo } => ("StudioPluginViaCli", github_repo, ""),
            ToolKind::BlenderAddon { github_repo } => ("BlenderAddon", github_repo, ""),
        };
        let category = Category::ALL.into_iter().find(|c| c.contains(e)).unwrap();
        serde_json::json!({"key":e.key,"category":format!("{category:?}"),"description":e.description,"docs":e.docs_url,"source":source,"manual":manual,"kind":kind})
    }).collect();
    let expected: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/catalog.json")).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn original_full_plan_order_fixture() {
    let mut selected = Selection::load(&GlobalConfig {
        roblox_projects_root: Some("fixture-projects".into()),
        ..Default::default()
    })
    .unwrap();
    for category in Category::ALL {
        selected.replace_category(
            category,
            &category
                .entries()
                .iter()
                .map(|e| e.key.to_string())
                .collect(),
        );
    }
    let actual: Vec<_> = worker::plan(&selected)
        .iter()
        .map(|i| i.label().to_string())
        .collect();
    assert_eq!(
        actual,
        [
            "Rokit bootstrap",
            "git",
            "vscode",
            "studio",
            "roblox",
            "blender",
            "figma",
            "Projects folder",
            "rojo",
            "wally",
            "wally-package-types",
            "selene",
            "stylua",
            "lute",
            "luau-lsp-cli",
            "jest-roblox",
            "asphalt",
            "tungsten",
            "scribe-studio",
            "jest-roblox-plugin",
            "rojo-plugin",
            "hoarcekat",
            "luau-lsp-plugin",
            "ui-labs",
            "resurface",
            "blender-plugin",
            "luau-lsp",
            "vscode-rojo",
            "selene-vscode",
            "stylua-vscode",
            "roblox-ui",
            "testez-companion",
            "github-actions",
            "theme-one-dark",
            "theme-monospace",
            "theme-horizon",
            "theme-catppuccin",
            "theme-catppuccin-icons"
        ]
    );
}

#[test]
fn original_selection_config_roundtrip_preserves_unknown_and_inactive_choices() {
    let config: GlobalConfig = toml::from_str("roblox_projects_root='fixture-projects'\nselected_system_apps=['git']\nselected_rokit_tools=['future-cli']\nselected_studio_plugins=['blender-plugin','future-plugin']\nselected_vscode_extensions=['theme-catppuccin']\nlast_checked='old'\n").unwrap();
    let selection = Selection::load(&config).unwrap();
    let saved = selection.to_config("reviewed".into());
    let text = toml::to_string(&saved).unwrap();
    let decoded: GlobalConfig = toml::from_str(&text).unwrap();
    assert_eq!(Selection::load(&decoded).unwrap(), selection);
    assert_eq!(decoded.last_checked.as_deref(), Some("reviewed"));
    assert_eq!(
        worker::plan(&selection)
            .iter()
            .map(|i| i.label())
            .collect::<Vec<_>>(),
        ["Rokit bootstrap", "git", "Projects folder"]
    );
}
