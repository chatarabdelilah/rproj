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

impl ToolKind {
    pub fn label(&self) -> &'static str {
        match self {
            ToolKind::SystemApp { .. } => "system app",
            ToolKind::RokitTool { .. } => "rokit tool",
            ToolKind::VsCodeExtension { .. } => "vscode extension",
            ToolKind::StudioPlugin { .. } => "studio plugin",
        }
    }

    /// The underlying package/extension/repo identifier, e.g. "Git.Git" or
    /// "rojo-rbx/rojo". Used for the compact `rproj info` listing.
    pub fn provider(&self) -> &'static str {
        match self {
            ToolKind::SystemApp { winget_id } => winget_id,
            ToolKind::RokitTool { rokit_source } => rokit_source,
            ToolKind::VsCodeExtension { extension_id } => extension_id,
            ToolKind::StudioPlugin { github_repo, .. } => github_repo,
        }
    }
}

pub struct ToolEntry {
    pub key: &'static str,
    pub description: &'static str,
    pub maintenance: Maintenance,
    pub kind: ToolKind,
    /// Groups a tool with its counterparts across install mechanisms, e.g.
    /// the "Rojo" family contains the rokit CLI tool, its Studio sync
    /// plugin, and its VS Code extension - shown together in `rproj info`
    /// instead of split across separate system/rokit/plugin/extension lists.
    pub family: &'static str,
    /// Pre-checked in the `rproj setup` picker.
    pub default_selected: bool,
    pub docs_url: &'static str,
}

/// Display order for `rproj info`'s Tools section.
pub const FAMILY_ORDER: &[&str] = &[
    "System apps",
    "Rojo",
    "Wally",
    "Selene",
    "StyLua",
    "Lute",
    "Luau Language Server",
    "Testing & extras",
];

pub const SYSTEM_APPS: &[ToolEntry] = &[
    ToolEntry {
        key: "git",
        description: "Version control - required for any real team workflow",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Git.Git" },
        family: "System apps",
        default_selected: true,
        docs_url: "https://git-scm.com/",
    },
    ToolEntry {
        key: "vscode",
        description: "Code editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Microsoft.VisualStudioCode" },
        family: "System apps",
        default_selected: true,
        docs_url: "https://code.visualstudio.com/",
    },
    ToolEntry {
        key: "studio",
        description: "Roblox Studio, the game editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Roblox.RobloxStudio" },
        family: "System apps",
        default_selected: true,
        docs_url: "https://create.roblox.com/",
    },
    ToolEntry {
        key: "roblox",
        description: "Roblox client - needed to play/test published games",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Roblox.Roblox" },
        family: "System apps",
        default_selected: true,
        docs_url: "https://www.roblox.com/",
    },
    ToolEntry {
        key: "blender",
        description: "3D modeling tool, for building custom meshes/animations (optional, heavier install)",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "BlenderFoundation.Blender" },
        family: "System apps",
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
        family: "Rojo",
        default_selected: true,
        docs_url: "https://rojo.space/",
    },
    ToolEntry {
        key: "wally",
        description: "Package manager for Roblox/Luau, similar to npm or cargo",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "wally" },
        family: "Wally",
        default_selected: true,
        docs_url: "https://wally.run/",
    },
    ToolEntry {
        key: "wally-package-types",
        description: "Generates Luau type definitions for your installed Wally packages",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "wally-package-types" },
        family: "Wally",
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/wally-package-types",
    },
    ToolEntry {
        key: "selene",
        description: "Static analysis linter for Luau",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "selene" },
        family: "Selene",
        default_selected: true,
        docs_url: "https://kampfkarren.github.io/selene/",
    },
    ToolEntry {
        key: "stylua",
        description: "Deterministic code formatter for Luau",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "JohnnyMorganz/StyLua" },
        family: "StyLua",
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/StyLua",
    },
    ToolEntry {
        key: "lute",
        description: "Standalone Luau runtime, used for scripting/tooling tasks",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "luau-lang/lute" },
        family: "Lute",
        default_selected: true,
        docs_url: "https://lute.luau.org/",
    },
];

pub const STUDIO_PLUGINS: &[ToolEntry] = &[
    ToolEntry {
        key: "rojo-plugin",
        description: "Studio-side companion for Rojo's file sync (installed via `rojo plugin install`)",
        maintenance: Maintenance::Active,
        kind: ToolKind::StudioPlugin { github_repo: "rojo-rbx/rojo", asset_suffix: "" },
        family: "Rojo",
        default_selected: true,
        docs_url: "https://rojo.space/",
    },
    ToolEntry {
        key: "hoarcekat",
        description: "Storybook-style previewer for isolated UI components, especially useful with React/Roact",
        maintenance: Maintenance::Active,
        kind: ToolKind::StudioPlugin { github_repo: "Kampfkarren/hoarcekat", asset_suffix: ".rbxmx" },
        family: "Testing & extras",
        default_selected: false,
        docs_url: "https://github.com/Kampfkarren/hoarcekat",
    },
];

pub const VSCODE_EXTENSIONS: &[ToolEntry] = &[
    ToolEntry {
        key: "luau-lsp",
        description: "Luau language server - autocomplete, type checking, go-to-definition",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "JohnnyMorganz.luau-lsp" },
        family: "Luau Language Server",
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/luau-lsp",
    },
    ToolEntry {
        key: "vscode-rojo",
        description: "Rojo commands and status from inside the editor - hasn't been updated since 2022, but still functions",
        maintenance: Maintenance::CommunityStable,
        kind: ToolKind::VsCodeExtension { extension_id: "evaera.vscode-rojo" },
        family: "Rojo",
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=evaera.vscode-rojo",
    },
    ToolEntry {
        key: "selene-vscode",
        description: "Inline Selene lint diagnostics",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "Kampfkarren.selene-vscode" },
        family: "Selene",
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=Kampfkarren.selene-vscode",
    },
    ToolEntry {
        key: "stylua-vscode",
        description: "Format Luau files with StyLua on save",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "JohnnyMorganz.stylua" },
        family: "StyLua",
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

/// All entries sharing a family, e.g. "Rojo" pulls together the rokit CLI
/// tool, the Studio sync plugin, and the VS Code extension.
pub fn in_family<'a>(family: &'a str) -> impl Iterator<Item = &'static ToolEntry> + 'a {
    all_setup_entries().filter(move |t| t.family == family)
}
