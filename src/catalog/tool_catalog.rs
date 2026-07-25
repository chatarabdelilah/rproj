use super::Maintenance;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ToolKind {
    /// Installed/detected via `winget`.
    SystemApp { winget_id: &'static str },
    /// Installed/detected via `rokit add <source>`, pinned in the project's rokit.toml.
    RokitTool { rokit_source: &'static str },
    /// Installed/detected via `code --install-extension <id>`.
    VsCodeExtension { extension_id: &'static str },
    /// A Roblox Studio plugin. Rojo's own plugin installs via `rojo plugin install`;
    /// anything else is installed by downloading a GitHub release asset into the
    /// Studio plugins folder (see steps::studio_plugin).
    StudioPlugin { github_repo: &'static str, asset_suffix: &'static str },
}

pub struct ToolEntry {
    pub key: &'static str,
    pub description: &'static str,
    pub maintenance: Maintenance,
    pub kind: ToolKind,
    /// Pre-checked in the `rproj setup` picker.
    pub default_selected: bool,
    pub docs_url: &'static str,
}

pub const SYSTEM_APPS: &[ToolEntry] = &[
    ToolEntry {
        key: "git",
        description: "Version control - required for any real team workflow",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Git.Git" },
        default_selected: true,
        docs_url: "https://git-scm.com/",
    },
    ToolEntry {
        key: "vscode",
        description: "Code editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Microsoft.VisualStudioCode" },
        default_selected: true,
        docs_url: "https://code.visualstudio.com/",
    },
    ToolEntry {
        key: "studio",
        description: "Roblox Studio, the game editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Roblox.RobloxStudio" },
        default_selected: true,
        docs_url: "https://create.roblox.com/",
    },
    ToolEntry {
        key: "roblox",
        description: "Roblox client - needed to play/test published games",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Roblox.Roblox" },
        default_selected: true,
        docs_url: "https://www.roblox.com/",
    },
    ToolEntry {
        key: "blender",
        description: "3D modeling tool, for building custom meshes/animations (optional, heavier install)",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "BlenderFoundation.Blender" },
        default_selected: false,
        docs_url: "https://www.blender.org/",
    },
];

pub const ROKIT_TOOLS: &[ToolEntry] = &[
    ToolEntry {
        key: "rojo",
        description: "Syncs your filesystem code into Roblox Studio",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "rojo" },
        default_selected: true,
        docs_url: "https://rojo.space/",
    },
    ToolEntry {
        key: "wally",
        description: "Package manager for Roblox/Luau, similar to npm or cargo",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "wally" },
        default_selected: true,
        docs_url: "https://wally.run/",
    },
    ToolEntry {
        key: "wally-package-types",
        description: "Generates Luau type definitions for your installed Wally packages",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "wally-package-types" },
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/wally-package-types",
    },
    ToolEntry {
        key: "selene",
        description: "Static analysis linter for Luau",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "selene" },
        default_selected: true,
        docs_url: "https://kampfkarren.github.io/selene/",
    },
    ToolEntry {
        key: "stylua",
        description: "Deterministic code formatter for Luau",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "JohnnyMorganz/StyLua" },
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/StyLua",
    },
    ToolEntry {
        key: "lute",
        description: "Standalone Luau runtime, used for scripting/tooling tasks",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "luau-lang/lute" },
        default_selected: true,
        docs_url: "https://github.com/luau-lang/lute",
    },
];

pub const STUDIO_PLUGINS: &[ToolEntry] = &[
    ToolEntry {
        key: "rojo-plugin",
        description: "Studio-side companion for Rojo's file sync (installed via `rojo plugin install`)",
        maintenance: Maintenance::Active,
        kind: ToolKind::StudioPlugin { github_repo: "rojo-rbx/rojo", asset_suffix: "" },
        default_selected: true,
        docs_url: "https://rojo.space/",
    },
    ToolEntry {
        key: "hoarcekat",
        description: "Storybook-style previewer for isolated UI components, especially useful with React/Roact",
        maintenance: Maintenance::Active,
        kind: ToolKind::StudioPlugin { github_repo: "Kampfkarren/hoarcekat", asset_suffix: ".rbxmx" },
        default_selected: false,
        docs_url: "https://github.com/Kampfkarren/hoarcekat",
    },
];

pub const VSCODE_EXTENSIONS: &[ToolEntry] = &[
    ToolEntry {
        key: "luau-lsp",
        description: "Luau language server - autocomplete, type checking, go-to-definition",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "JohnnyMorganz.luau-language-server" },
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/luau-lsp",
    },
    ToolEntry {
        key: "vscode-rojo",
        description: "Rojo commands and status from inside the editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "evaera.vscode-rojo" },
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=evaera.vscode-rojo",
    },
    ToolEntry {
        key: "selene-vscode",
        description: "Inline Selene lint diagnostics",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "Kampfkarren.selene-vscode" },
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=Kampfkarren.selene-vscode",
    },
    ToolEntry {
        key: "stylua-vscode",
        description: "Format Luau files with StyLua on save",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "JohnnyMorganz.stylua" },
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=JohnnyMorganz.stylua",
    },
];

pub fn all_setup_entries() -> impl Iterator<Item = &'static ToolEntry> {
    SYSTEM_APPS
        .iter()
        .chain(ROKIT_TOOLS.iter())
        .chain(STUDIO_PLUGINS.iter())
        .chain(VSCODE_EXTENSIONS.iter())
}

pub fn find(key: &str) -> Option<&'static ToolEntry> {
    all_setup_entries().find(|t| t.key == key)
}
