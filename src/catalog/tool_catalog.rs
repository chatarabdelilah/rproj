use super::Maintenance;

/// How to tell whether a system app is already installed.
///
/// Separate from *how it is installed*, because for at least one app those
/// are different questions. `winget list --id Roblox.RobloxStudio -e` exits
/// 20 ("no installed package found") on a machine with Studio installed and
/// running, because Studio installs per-user through its own bootstrapper
/// into `%LOCALAPPDATA%\Roblox\Versions\` and never registers anywhere
/// winget looks.
///
/// That one wrong answer used to cause two visible failures in a single
/// run: setup tried to reinstall Studio (hitting winget's stale-hash issue,
/// which the user would otherwise never have reached), and then skipped
/// `rojo plugin install` as "Studio isn't installed" - while installing
/// other plugins into the Studio plugins folder that only exists because
/// Studio *is* installed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detect {
    /// `winget list --id <id> -e`. Correct for anything winget itself
    /// installed, which is everything except the Roblox apps' own
    /// bootstrapper.
    Winget,
    /// An executable at `%<env_var>%\<subdir>\<exe>`, or one level deeper
    /// under a versioned directory - which is how Roblox lays out
    /// `Versions\version-<hash>\`.
    ExeUnder {
        env_var: &'static str,
        subdir: &'static str,
        exe: &'static str,
    },
}

pub enum ToolKind {
    /// Installed via `winget`; detected via `detect`, which is not always
    /// the same thing - see `Detect`.
    SystemApp {
        winget_id: &'static str,
        detect: Detect,
    },
    /// Installed/detected via `rokit add <source>`, pinned in the project's rokit.toml.
    RokitTool { rokit_source: &'static str },
    /// Installed/detected via `code --install-extension <id>`.
    VsCodeExtension { extension_id: &'static str },
    /// A Roblox Studio plugin. Rojo's own plugin installs via `rojo plugin install`;
    /// anything else is installed by downloading a GitHub release asset into the
    /// Studio plugins folder (see steps::studio_plugin).
    StudioPlugin { github_repo: &'static str, asset_suffix: &'static str },
    /// The official Roblox Blender add-on - installed into Blender itself,
    /// not Roblox Studio, via headless Python (see steps::blender). Only
    /// relevant if Blender is also selected, and surfaced as a contextual
    /// follow-up in the plugins step rather than the system-apps picker.
    BlenderAddon { github_repo: &'static str },
}

impl ToolKind {
    pub fn label(&self) -> &'static str {
        match self {
            ToolKind::SystemApp { .. } => "system app",
            ToolKind::RokitTool { .. } => "rokit tool",
            ToolKind::VsCodeExtension { .. } => "vscode extension",
            ToolKind::StudioPlugin { .. } => "studio plugin",
            ToolKind::BlenderAddon { .. } => "blender add-on",
        }
    }

    /// The underlying package/extension/repo identifier, e.g. "Git.Git" or
    /// "rojo-rbx/rojo". Used for the compact `rproj info` listing.
    pub fn provider(&self) -> &'static str {
        match self {
            ToolKind::SystemApp { winget_id, .. } => winget_id,
            ToolKind::RokitTool { rokit_source } => rokit_source,
            ToolKind::VsCodeExtension { extension_id } => extension_id,
            ToolKind::StudioPlugin { github_repo, .. } => github_repo,
            ToolKind::BlenderAddon { github_repo } => github_repo,
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
    "Tarmac",
    "Mantle",
    "Luau Language Server",
    "Blender",
    "Themes",
    "Testing & extras",
];

pub const SYSTEM_APPS: &[ToolEntry] = &[
    ToolEntry {
        key: "git",
        description: "Version control - required for any real team workflow",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Git.Git", detect: Detect::Winget },
        family: "System apps",
        default_selected: true,
        docs_url: "https://git-scm.com/",
    },
    ToolEntry {
        key: "vscode",
        description: "Code editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Microsoft.VisualStudioCode", detect: Detect::Winget },
        family: "System apps",
        default_selected: true,
        docs_url: "https://code.visualstudio.com/",
    },
    ToolEntry {
        key: "studio",
        description: "Roblox Studio, the game editor",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp {
            winget_id: "Roblox.RobloxStudio",
            // The one app winget cannot see - see `Detect`.
            detect: Detect::ExeUnder {
                env_var: "LOCALAPPDATA",
                subdir: "Roblox/Versions",
                exe: "RobloxStudioBeta.exe",
            },
        },
        family: "System apps",
        default_selected: true,
        docs_url: "https://create.roblox.com/",
    },
    ToolEntry {
        key: "roblox",
        description: "Roblox client - needed to play/test published games",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "Roblox.Roblox", detect: Detect::Winget },
        family: "System apps",
        default_selected: true,
        docs_url: "https://www.roblox.com/",
    },
    ToolEntry {
        key: "blender",
        description: "3D modeling tool, for building custom meshes/animations (optional, heavier install)",
        maintenance: Maintenance::Active,
        kind: ToolKind::SystemApp { winget_id: "BlenderFoundation.Blender", detect: Detect::Winget },
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
        description: "Standalone Luau runtime - runs the project's `.lute/check.luau` quality gate locally and in CI",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "luau-lang/lute" },
        family: "Lute",
        default_selected: true,
        docs_url: "https://lute.luau.org/",
    },
    ToolEntry {
        key: "luau-lsp-cli",
        description: "Luau language server as a command-line type checker - this is what `luau-lsp analyze` in the quality gate runs, separate from the VS Code extension of the same name",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "JohnnyMorganz/luau-lsp" },
        family: "Luau Language Server",
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/luau-lsp",
    },
    ToolEntry {
        key: "tarmac",
        description: "Roblox's own asset-sync tool - uploads images/sounds/meshes and generates Luau references for them",
        maintenance: Maintenance::Active,
        kind: ToolKind::RokitTool { rokit_source: "Roblox/tarmac" },
        family: "Tarmac",
        default_selected: false,
        docs_url: "https://github.com/Roblox/tarmac",
    },
    ToolEntry {
        key: "mantle",
        description: "Infrastructure-as-code deployment tool for Roblox places - no longer maintained upstream (the author's own words: \"do not expect responses to tickets, bug fixes, or new features\"), but still the most complete option for scripted deploys",
        maintenance: Maintenance::Legacy,
        kind: ToolKind::RokitTool { rokit_source: "blake-mealey/mantle" },
        family: "Mantle",
        default_selected: false,
        docs_url: "https://mantledeploy.vercel.app/",
    },
];

/// Plugins shown in the plugins step, contextually filtered to what's
/// relevant given the tools already picked (e.g. the Blender add-on only
/// makes sense if Blender was selected). Not all of these install into
/// Roblox Studio specifically - the Blender add-on installs into Blender.
pub const PLUGINS: &[ToolEntry] = &[
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
        kind: ToolKind::StudioPlugin { github_repo: "Kampfkarren/hoarcekat", asset_suffix: ".rbxm" },
        family: "Testing & extras",
        default_selected: false,
        docs_url: "https://github.com/Kampfkarren/hoarcekat",
    },
    ToolEntry {
        key: "luau-lsp-plugin",
        description: "Companion for the Luau language server - exposes Studio's live DataModel (instances not in your Rojo files) for better autocomplete",
        maintenance: Maintenance::Active,
        kind: ToolKind::StudioPlugin { github_repo: "JohnnyMorganz/luau-lsp", asset_suffix: ".rbxm" },
        family: "Luau Language Server",
        default_selected: true,
        docs_url: "https://github.com/JohnnyMorganz/luau-lsp/blob/main/editors/README.md",
    },
    ToolEntry {
        key: "blender-plugin",
        description: "Roblox's official Blender add-on (\"Roblox Upload\") - uploads meshes/assets to Roblox via the Open Cloud API",
        maintenance: Maintenance::Active,
        kind: ToolKind::BlenderAddon { github_repo: "Roblox/roblox-blender-plugin" },
        family: "Blender",
        default_selected: true,
        docs_url: "https://create.roblox.com/docs/art/modeling/roblox-blender-plugin",
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
    ToolEntry {
        key: "roblox-ui",
        description: "Explorer panel for your Rojo project inside VS Code - browse the file tree as the Roblox instances it becomes",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "filiptibell.roblox-ui" },
        family: "Rojo",
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=filiptibell.roblox-ui",
    },
    ToolEntry {
        key: "testez-companion",
        description: "Runs your TestEZ tests from VS Code and shows the results inline, talking to a Studio-side plugin",
        maintenance: Maintenance::CommunityStable,
        kind: ToolKind::VsCodeExtension { extension_id: "tacheometrist.testez-companion" },
        family: "Testing & extras",
        default_selected: false,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=tacheometrist.testez-companion",
    },
    ToolEntry {
        key: "github-actions",
        description: "Shows CI workflow runs and their logs in the editor, and gives you completion when editing workflow YAML",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "github.vscode-github-actions" },
        family: "Testing & extras",
        default_selected: true,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=github.vscode-github-actions",
    },
    ToolEntry {
        key: "theme-one-dark",
        description: "Atom One Dark Theme, by Mahmoud Ali - a faithful port of Atom's iconic dark theme",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "akamud.vscode-theme-onedark" },
        family: "Themes",
        default_selected: false,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=akamud.vscode-theme-onedark",
    },
    ToolEntry {
        key: "theme-monospace",
        description: "Monospace Theme, by Keksi - a minimalistic theme with subtle purple accents",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "keksiqc.idx-monospace-theme" },
        family: "Themes",
        default_selected: false,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=keksiqc.idx-monospace-theme",
    },
    ToolEntry {
        key: "theme-horizon",
        description: "Horizon Theme, by Alexander Nanberg - a warm dual (light/dark) theme",
        maintenance: Maintenance::Active,
        kind: ToolKind::VsCodeExtension { extension_id: "alexandernanberg.horizon-theme-vscode" },
        family: "Themes",
        default_selected: false,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=alexandernanberg.horizon-theme-vscode",
    },
];

pub fn all_setup_entries() -> impl Iterator<Item = &'static ToolEntry> {
    SYSTEM_APPS
        .iter()
        .chain(ROKIT_TOOLS.iter())
        .chain(PLUGINS.iter())
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
