//! What `rproj configure <tool>` can walk you through.
//!
//! Data-driven: every setting below is a data entry describing what it
//! does, what values it accepts and what each value means. `commands::
//! configure` prompts from this table and writes the result to the file
//! named by the tool's `target` - adding a new setting, or a whole new
//! configurable tool, requires no code changes.
//!
//! Option names and accepted values are taken from each tool's own
//! upstream documentation (StyLua's README options table, Selene's lint
//! docs, the luau-lsp extension's own `package.json` contributions), not
//! from memory.

/// One accepted value of a `Choice` setting, with what picking it does.
pub struct ChoiceOption {
    pub value: &'static str,
    pub explanation: &'static str,
}

pub enum SettingKind {
    Bool { default: bool },
    Integer { default: i64 },
    Choice { default: &'static str, options: &'static [ChoiceOption] },
}

pub struct SettingSpec {
    /// The key as it appears in the target file, e.g. `column_width` or
    /// `luau-lsp.inlayHints.parameterNames`.
    pub key: &'static str,
    pub description: &'static str,
    /// TOML table this key belongs under (e.g. Selene's `rules`), or `None`
    /// for a top-level key. Ignored for JSON targets, which are flat.
    pub section: Option<&'static str>,
    pub kind: SettingKind,
}

/// Where a tool's settings get written.
pub enum ConfigTarget {
    /// A TOML file in the project root, e.g. `stylua.toml`.
    ProjectToml { filename: &'static str },
    /// VS Code's per-project `.vscode/settings.json`, a flat JSON object
    /// keyed by dotted setting name.
    VsCodeSettings,
}

pub struct ConfigurableTool {
    /// Matches the tool's catalog key, so `rproj configure stylua` and
    /// `rproj info stylua` refer to the same thing.
    pub key: &'static str,
    pub display_name: &'static str,
    pub summary: &'static str,
    pub target: ConfigTarget,
    pub docs_url: &'static str,
    pub settings: &'static [SettingSpec],
}

/// Selene lint rules worth offering. Every lint takes the same three
/// values, so they share one option list.
const LINT_LEVELS: &[ChoiceOption] = &[
    ChoiceOption { value: "deny", explanation: "Report as an error and fail the lint run" },
    ChoiceOption { value: "warn", explanation: "Report as a warning, but don't fail" },
    ChoiceOption { value: "allow", explanation: "Don't report this at all" },
];

pub const CONFIGURABLE_TOOLS: &[ConfigurableTool] = &[
    ConfigurableTool {
        key: "stylua",
        display_name: "StyLua",
        summary: "Code formatter. These settings decide how your Luau is reshaped on format/save.",
        target: ConfigTarget::ProjectToml { filename: "stylua.toml" },
        docs_url: "https://github.com/JohnnyMorganz/StyLua#options",
        settings: &[
            SettingSpec {
                key: "syntax",
                description: "Which Lua/Luau dialect to parse as. Roblox projects want Luau - it enables type-annotation syntax the other dialects would reject.",
                section: None,
                kind: SettingKind::Choice {
                    default: "Luau",
                    options: &[
                        ChoiceOption { value: "Luau", explanation: "Roblox's Luau, including type annotations (what you want here)" },
                        ChoiceOption { value: "All", explanation: "Accept any supported dialect - StyLua's own default" },
                        ChoiceOption { value: "Lua51", explanation: "Plain Lua 5.1" },
                        ChoiceOption { value: "Lua54", explanation: "Plain Lua 5.4" },
                    ],
                },
            },
            SettingSpec {
                key: "column_width",
                description: "Line length StyLua aims for before wrapping. A guide, not a hard cap - lines can still run over when there's no good break.",
                section: None,
                kind: SettingKind::Integer { default: 120 },
            },
            SettingSpec {
                key: "indent_type",
                description: "Indent with real tab characters or with spaces. Tabs are StyLua's default and let each reader choose their own visual width.",
                section: None,
                kind: SettingKind::Choice {
                    default: "Tabs",
                    options: &[
                        ChoiceOption { value: "Tabs", explanation: "One tab character per level" },
                        ChoiceOption { value: "Spaces", explanation: "indent_width spaces per level" },
                    ],
                },
            },
            SettingSpec {
                key: "indent_width",
                description: "How many columns one indent level is. With indent_type = Tabs this only affects how StyLua estimates line width.",
                section: None,
                kind: SettingKind::Integer { default: 4 },
            },
            SettingSpec {
                key: "quote_style",
                description: "Which quote character string literals get. The AutoPrefer options switch to the other quote when it means fewer backslash escapes.",
                section: None,
                kind: SettingKind::Choice {
                    default: "AutoPreferDouble",
                    options: &[
                        ChoiceOption { value: "AutoPreferDouble", explanation: "Double quotes, unless single quotes need fewer escapes" },
                        ChoiceOption { value: "AutoPreferSingle", explanation: "Single quotes, unless double quotes need fewer escapes" },
                        ChoiceOption { value: "ForceDouble", explanation: "Always double quotes, escapes regardless" },
                        ChoiceOption { value: "ForceSingle", explanation: "Always single quotes, escapes regardless" },
                    ],
                },
            },
            SettingSpec {
                key: "call_parentheses",
                description: "Whether to keep parentheses on calls taking a single string or table, like require\"foo\" or f{...}.",
                section: None,
                kind: SettingKind::Choice {
                    default: "Always",
                    options: &[
                        ChoiceOption { value: "Always", explanation: "Always parenthesise - most explicit, easiest to read" },
                        ChoiceOption { value: "NoSingleString", explanation: "Drop them for a single string argument" },
                        ChoiceOption { value: "NoSingleTable", explanation: "Drop them for a single table argument" },
                        ChoiceOption { value: "None", explanation: "Drop them for both cases" },
                        ChoiceOption { value: "Input", explanation: "Leave exactly as written - no consistency enforced" },
                    ],
                },
            },
            SettingSpec {
                key: "collapse_simple_statement",
                description: "Whether a short single-statement block may be pulled onto one line, e.g. if x then return end.",
                section: None,
                kind: SettingKind::Choice {
                    default: "Never",
                    options: &[
                        ChoiceOption { value: "Never", explanation: "Always keep blocks expanded across lines" },
                        ChoiceOption { value: "FunctionOnly", explanation: "Collapse single-statement function bodies only" },
                        ChoiceOption { value: "ConditionalOnly", explanation: "Collapse single-statement if bodies only" },
                        ChoiceOption { value: "Always", explanation: "Collapse both" },
                    ],
                },
            },
            SettingSpec {
                key: "line_endings",
                description: "Line ending written to disk. Unix (LF) is the safe default even on Windows - it avoids whole-file diffs when teammates are on other platforms.",
                section: None,
                kind: SettingKind::Choice {
                    default: "Unix",
                    options: &[
                        ChoiceOption { value: "Unix", explanation: "LF" },
                        ChoiceOption { value: "Windows", explanation: "CRLF" },
                    ],
                },
            },
            SettingSpec {
                key: "enabled",
                description: "Sort consecutive require assignments alphabetically. Keeps import blocks tidy and cuts down merge conflicts at the top of files.",
                section: Some("sort_requires"),
                kind: SettingKind::Bool { default: true },
            },
        ],
    },
    ConfigurableTool {
        key: "selene",
        display_name: "Selene",
        summary: "Linter. `std` tells it which globals exist; the rest turn individual lints up or down.",
        target: ConfigTarget::ProjectToml { filename: "selene.toml" },
        docs_url: "https://kampfkarren.github.io/selene/usage/configuration.html",
        settings: &[
            SettingSpec {
                key: "std",
                description: "Which standard library to check against. Get this wrong and Selene flags every Roblox global as undefined.",
                section: None,
                kind: SettingKind::Choice {
                    default: "roblox",
                    options: &[
                        ChoiceOption { value: "roblox", explanation: "Roblox globals (game, workspace, script...)" },
                        ChoiceOption { value: "roblox+testez", explanation: "Roblox globals plus TestEZ's describe/it/expect" },
                        ChoiceOption { value: "luau", explanation: "Plain Luau, no Roblox globals" },
                        ChoiceOption { value: "lua51", explanation: "Plain Lua 5.1" },
                    ],
                },
            },
            SettingSpec {
                key: "undefined_variable",
                description: "Using a name that was never defined. Usually a typo or a missing require - worth keeping strict.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "deny", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "unused_variable",
                description: "A local that's assigned but never read. Often leftover code; prefix with _ to intentionally ignore one.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "warn", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "shadowing",
                description: "A local re-using a name already in scope. Legal, but a common source of confusing bugs.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "warn", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "global_usage",
                description: "Reading or writing _G. Shared mutable global state is hard to trace in a multi-script game.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "warn", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "incorrect_standard_library_use",
                description: "Calling a standard-library function with the wrong argument count or types.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "deny", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "mixed_table",
                description: "A table with both array entries and key/value entries. Legal, but #t and ipairs behave surprisingly on them.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "warn", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "multiple_statements",
                description: "More than one statement on a single line. Mostly a readability preference.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "allow", options: LINT_LEVELS },
            },
            SettingSpec {
                key: "roblox_incorrect_roact_usage",
                description: "Roact/React mistakes Selene can spot statically, like an invalid instance name in createElement.",
                section: Some("rules"),
                kind: SettingKind::Choice { default: "deny", options: LINT_LEVELS },
            },
        ],
    },
    ConfigurableTool {
        key: "luau-lsp",
        display_name: "Luau Language Server (VS Code)",
        summary: "Autocomplete, type checking and inlay hints. Sourcemap settings are what make requires resolve.",
        target: ConfigTarget::VsCodeSettings,
        docs_url: "https://github.com/JohnnyMorganz/luau-lsp",
        settings: &[
            SettingSpec {
                key: "luau-lsp.sourcemap.enabled",
                description: "Use a Rojo sourcemap to understand your DataModel. Without this, requires through ReplicatedStorage resolve to nothing and you get no autocomplete on your own modules.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "luau-lsp.sourcemap.autogenerate",
                description: "Let the extension run Rojo itself to keep sourcemap.json fresh as files change, instead of you running a watcher.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "luau-lsp.sourcemap.rojoProjectFile",
                description: "Which project file to build the sourcemap from. rproj scaffolds default.project.json.",
                section: None,
                kind: SettingKind::Choice {
                    default: "default.project.json",
                    options: &[ChoiceOption {
                        value: "default.project.json",
                        explanation: "The project file rproj generates",
                    }],
                },
            },
            SettingSpec {
                key: "luau-lsp.types.roblox",
                description: "Load Roblox's API type definitions, so Instance, Vector3 and the rest are known types.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "luau-lsp.platform.type",
                description: "Which platform's globals and types to assume.",
                section: None,
                kind: SettingKind::Choice {
                    default: "roblox",
                    options: &[
                        ChoiceOption { value: "roblox", explanation: "Roblox - what you want for a Rojo project" },
                        ChoiceOption { value: "standard", explanation: "Plain Luau with no Roblox API" },
                    ],
                },
            },
            SettingSpec {
                key: "luau-lsp.completion.autocompleteEnd",
                description: "Automatically insert the matching `end` when you open a block.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "luau-lsp.completion.imports.enabled",
                description: "Offer to add the require line for you when you complete a module name that isn't imported yet.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "luau-lsp.inlayHints.parameterNames",
                description: "Show parameter names inline at call sites, so f(true, 3) reads as f(enabled: true, count: 3).",
                section: None,
                kind: SettingKind::Choice {
                    default: "literals",
                    options: &[
                        ChoiceOption { value: "none", explanation: "No parameter name hints" },
                        ChoiceOption { value: "literals", explanation: "Only for literal arguments, where it helps most" },
                        ChoiceOption { value: "all", explanation: "For every argument - thorough but noisy" },
                    ],
                },
            },
            SettingSpec {
                key: "luau-lsp.inlayHints.variableTypes",
                description: "Show the inferred type next to variables you didn't annotate.",
                section: None,
                kind: SettingKind::Bool { default: false },
            },
            SettingSpec {
                key: "luau-lsp.inlayHints.functionReturnTypes",
                description: "Show the inferred return type on functions you didn't annotate.",
                section: None,
                kind: SettingKind::Bool { default: false },
            },
            SettingSpec {
                key: "luau-lsp.diagnostics.strictDatamodelTypes",
                description: "Type instances from the sourcemap exactly rather than loosely. Catches real mistakes, but flags code that looks up instances dynamically.",
                section: None,
                kind: SettingKind::Bool { default: false },
            },
            SettingSpec {
                key: "luau-lsp.diagnostics.workspace",
                description: "Report problems across the whole project, not just files you have open. Slower on large projects.",
                section: None,
                kind: SettingKind::Bool { default: false },
            },
            SettingSpec {
                key: "luau-lsp.plugin.enabled",
                description: "Accept live DataModel data from the companion Studio plugin, so instances that exist only in Studio still autocomplete.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
        ],
    },
    ConfigurableTool {
        key: "stylua-vscode",
        display_name: "StyLua (VS Code editor behaviour)",
        summary: "How the editor applies StyLua - the formatting rules themselves live in stylua.toml (`rproj configure stylua`).",
        target: ConfigTarget::VsCodeSettings,
        docs_url: "https://marketplace.visualstudio.com/items?itemName=JohnnyMorganz.stylua",
        settings: &[
            SettingSpec {
                key: "editor.formatOnSave",
                description: "Format every time you save. The usual way teams keep formatting out of code review entirely.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
            SettingSpec {
                key: "stylua.searchParentDirectories",
                description: "Look in parent folders for stylua.toml when the current folder has none.",
                section: None,
                kind: SettingKind::Bool { default: true },
            },
        ],
    },
];

pub fn find(key: &str) -> Option<&'static ConfigurableTool> {
    CONFIGURABLE_TOOLS.iter().find(|t| t.key == key)
}
