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

use serde_json::{json, Value};

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

impl SettingSpec {
    /// How the setting is named on screen. Sectioned keys have to be shown
    /// dotted: StyLua's only sectioned setting is called `enabled`, and a
    /// prompt headed by that word alone tells you nothing about what it
    /// enables.
    pub fn display_key(&self) -> String {
        match self.section {
            Some(section) => format!("{section}.{}", self.key),
            None => self.key.to_string(),
        }
    }
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

impl SettingKind {
    /// The value this setting takes when nobody has chosen one.
    pub fn default_value(&self) -> Value {
        match self {
            SettingKind::Bool { default } => json!(default),
            SettingKind::Integer { default } => json!(default),
            SettingKind::Choice { default, .. } => json!(default),
        }
    }
}

/// Renders `(setting, value)` pairs as TOML.
///
/// Top-level keys must come before any `[table]` header: in TOML every key
/// after a header belongs to that table, so emitting them in declaration
/// order would silently nest a top-level key under whatever section
/// preceded it.
pub fn render_toml(answers: &[(&SettingSpec, Value)]) -> String {
    let mut out = String::new();
    for (setting, value) in answers.iter().filter(|(s, _)| s.section.is_none()) {
        out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
    }

    let mut sections: Vec<&str> = answers.iter().filter_map(|(s, _)| s.section).collect();
    sections.dedup();
    for section in sections {
        out.push_str(&format!("\n[{section}]\n"));
        for (setting, value) in answers.iter().filter(|(s, _)| s.section == Some(section)) {
            out.push_str(&format!("{} = {}\n", setting.key, toml_value(value)));
        }
    }
    out
}

fn toml_value(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

/// Applies `answers` to an existing config file, rewriting only the lines
/// that hold a setting the catalog describes and leaving everything else -
/// comments, blank lines, keys and whole tables rproj knows nothing about -
/// byte for byte as it found them.
///
/// Re-rendering the file from the catalog instead was silently destructive.
/// A scaffolded `selene.toml` carries an `exclude` key that no `SettingSpec`
/// describes, and `rproj configure selene` deleted it - putting the project
/// straight back to the 2335-findings lint run that `exclude` exists to
/// prevent. Verified by running configure against a scaffolded project:
/// `exclude` was gone from the file afterwards.
///
/// Line-based rather than parse-and-reserialise so comments survive, which
/// also makes accepting every default a byte-exact no-op.
pub fn merge_toml(existing: &str, answers: &[(&SettingSpec, Value)]) -> String {
    if existing.trim().is_empty() {
        return render_toml(answers);
    }

    let mut out = String::new();
    let mut section: Option<String> = None;
    let mut replaced = vec![false; answers.len()];

    for line in existing.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            section = Some(name.trim().to_string());
            out.push_str(line);
            out.push('\n');
            continue;
        }

        let key = match trimmed.split_once('=') {
            Some((key, _)) if !trimmed.starts_with('#') => key.trim(),
            _ => {
                out.push_str(line);
                out.push('\n');
                continue;
            }
        };
        match answers.iter().position(|(spec, _)| spec.key == key && spec.section == section.as_deref()) {
            Some(i) => {
                replaced[i] = true;
                out.push_str(&format!("{key} = {}\n", toml_value(&answers[i].1)));
            }
            None => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    // A setting the file didn't carry yet - a new entry in the catalog, or a
    // hand-written config that only set some of them.
    for (i, (setting, value)) in answers.iter().enumerate() {
        if replaced[i] {
            continue;
        }
        let line = format!("{} = {}", setting.key, toml_value(value));
        out = match setting.section {
            None => insert_top_level(&out, &line),
            Some(section) => insert_into_section(&out, section, &line),
        };
    }
    out
}

/// Adds `line` directly under `[section]`, creating the table at the end of
/// the file if it isn't there. Appending to the file blindly would put the
/// key in whichever table happened to come last.
fn insert_into_section(toml: &str, section: &str, line: &str) -> String {
    let header = format!("[{section}]");
    match toml.lines().position(|l| l.trim() == header) {
        Some(i) => {
            let mut lines: Vec<&str> = toml.lines().collect();
            lines.insert(i + 1, line);
            format!("{}\n", lines.join("\n"))
        }
        None => format!("{toml}\n{header}\n{line}\n"),
    }
}

/// What each of `tool`'s settings is currently set to in `existing`.
///
/// `rproj configure` offers these as the prompt defaults. Offering the
/// catalog defaults instead meant pressing enter through the walkthrough
/// reverted every choice already made - including `std = "roblox+testez"`,
/// which a TestEZ project needs and which reverting breaks.
pub fn current_toml_values(tool: &ConfigurableTool, existing: &str) -> Vec<Option<Value>> {
    // A whole document, not a single value: `str::parse::<toml::Value>`
    // parses one value and rejects the rest of the file.
    let Ok(doc) = toml::from_str::<toml::Table>(existing) else {
        return tool.settings.iter().map(|_| None).collect();
    };
    tool.settings
        .iter()
        .map(|setting| {
            let value = match setting.section {
                Some(section) => doc.get(section)?.as_table()?.get(setting.key)?,
                None => doc.get(setting.key)?,
            };
            toml_to_json(value)
        })
        .collect()
}

/// Only the shapes a `SettingKind` can hold. Anything else (an array, a
/// nested table) isn't a value this walkthrough could have written, so it's
/// left alone rather than offered back as a default.
fn toml_to_json(value: &toml::Value) -> Option<Value> {
    match value {
        toml::Value::Boolean(b) => Some(json!(b)),
        toml::Value::Integer(i) => Some(json!(i)),
        toml::Value::String(s) => Some(json!(s)),
        _ => None,
    }
}

/// The config file a tool gets at scaffold time: every setting at its
/// documented default, with `overrides` applied by key.
///
/// This exists so the scaffolded file and `rproj configure` can't disagree.
/// They used to: the scaffold hardcoded `indent_type = "Spaces"` and no
/// `syntax`, while configure's defaults were `Tabs` and `Luau` - so
/// running configure and accepting every default silently reformatted the
/// whole project. One table now feeds both.
pub fn default_toml(tool_key: &str, overrides: &[(&str, &str)]) -> Option<String> {
    let tool = find(tool_key)?;
    if !matches!(tool.target, ConfigTarget::ProjectToml { .. }) {
        return None;
    }
    let answers: Vec<(&SettingSpec, Value)> = tool
        .settings
        .iter()
        .map(|setting| {
            let value = overrides
                .iter()
                .find(|(key, _)| *key == setting.key)
                .map(|(_, v)| json!(v))
                .unwrap_or_else(|| setting.kind.default_value());
            (setting, value)
        })
        .collect();
    Some(render_toml(&answers))
}

/// Inserts a raw top-level `key = value` line before the first `[section]`
/// header.
///
/// Appending would be wrong: in TOML every key after a header belongs to
/// that table, so an `exclude` tacked onto the end of a file with a
/// `[rules]` section becomes `rules.exclude` and is silently ignored.
pub fn insert_top_level(toml: &str, line: &str) -> String {
    // A file whose very first line is a header has no `\n[` to find, and
    // appending would drop the key into whichever table ends the file.
    if toml.starts_with('[') {
        return format!("{line}\n\n{toml}");
    }
    match toml.find("\n[") {
        // `i` is the newline immediately before the header. Slicing
        // through it keeps the blank line that already separated the
        // sections, and the trailing `\n\n` restores one after the
        // inserted key so the header isn't left crowded against it.
        Some(i) => format!("{}{line}\n\n{}", &toml[..=i], &toml[i + 1..]),
        None => format!("{toml}{line}\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The scaffolded file and `rproj configure` accepting every default
    /// must produce the same thing, or configure silently reformats the
    /// project the first time it's run.
    #[test]
    fn scaffolded_defaults_match_configure_defaults() {
        for tool in CONFIGURABLE_TOOLS {
            let ConfigTarget::ProjectToml { .. } = tool.target else { continue };
            let scaffolded = default_toml(tool.key, &[]).expect("project-toml tool renders");
            let answers: Vec<(&SettingSpec, Value)> =
                tool.settings.iter().map(|s| (s, s.kind.default_value())).collect();
            assert_eq!(scaffolded, render_toml(&answers), "{} diverged", tool.key);
        }
    }

    /// Luau type annotations are a parse error under StyLua's default
    /// `syntax = "All"`, so the scaffolded config has to set it.
    #[test]
    fn stylua_defaults_select_luau_syntax() {
        let toml = default_toml("stylua", &[]).unwrap();
        assert!(toml.contains(r#"syntax = "Luau""#), "{toml}");
    }

    /// Selene reports every Roblox global as undefined without the right
    /// std, and TestEZ projects need its globals too.
    #[test]
    fn selene_std_can_be_overridden_for_testez() {
        let plain = default_toml("selene", &[]).unwrap();
        assert!(plain.contains(r#"std = "roblox""#), "{plain}");

        let testez = default_toml("selene", &[("std", "roblox+testez")]).unwrap();
        assert!(testez.contains(r#"std = "roblox+testez""#), "{testez}");
    }

    /// A top-level key emitted after a [table] header silently becomes
    /// part of that table.
    #[test]
    fn top_level_keys_precede_any_section_header() {
        let toml = default_toml("selene", &[]).unwrap();
        let first_header = toml.find('[').expect("selene has a [rules] section");
        assert!(
            toml[..first_header].contains("std ="),
            "std must appear before the first header:\n{toml}"
        );
    }

    /// An inserted key landing after a header silently becomes part of
    /// that table - `exclude` would turn into `rules.exclude` and do
    /// nothing at all.
    #[test]
    fn inserted_keys_land_above_the_first_section() {
        let base = default_toml("selene", &[]).unwrap();
        let with_exclude = insert_top_level(&base, r#"exclude = ["modules/submodules/**"]"#);
        let first_header = with_exclude.find("
[").expect("selene has a [rules] section");
        assert!(with_exclude[..first_header].contains("exclude ="), "{with_exclude}");
        assert!(with_exclude.contains("[rules]"), "sections must survive:
{with_exclude}");
        assert!(
            with_exclude.contains("

[rules]"),
            "the header should keep a blank line above it:
{with_exclude}"
        );
    }

    /// A file with no sections at all still has to get the key.
    #[test]
    fn inserted_keys_work_without_any_section() {
        let out = insert_top_level("std = \"roblox\"
", "exclude = []");
        assert!(out.contains("exclude = []"), "{out}");
    }

    /// Answers built from what the file already says, i.e. what `rproj
    /// configure` sends to the writer when you press enter through it.
    fn enter_through(tool: &'static ConfigurableTool, existing: &str) -> Vec<(&'static SettingSpec, Value)> {
        let current = current_toml_values(tool, existing);
        tool.settings
            .iter()
            .zip(current)
            .map(|(spec, current)| (spec, current.unwrap_or_else(|| spec.kind.default_value())))
            .collect()
    }

    /// The scaffolded `exclude` is what keeps selene out of `Packages/`.
    /// Configure used to re-render the file from the catalog, which deleted
    /// it and put the project back to thousands of vendored-code findings.
    #[test]
    fn merging_keeps_keys_the_catalog_does_not_describe() {
        let tool = find("selene").unwrap();
        let existing = default_toml("selene", &[]).unwrap();
        let existing = insert_top_level(&existing, r#"exclude = ["Packages/**"]"#);

        let merged = merge_toml(&existing, &enter_through(tool, &existing));
        assert!(merged.contains(r#"exclude = ["Packages/**"]"#), "{merged}");
    }

    /// Accepting every prompt has to be a no-op. Anything else means the
    /// walkthrough rewrites your project just for being opened.
    #[test]
    fn enter_through_configure_changes_nothing() {
        for tool in CONFIGURABLE_TOOLS {
            let ConfigTarget::ProjectToml { .. } = tool.target else { continue };
            // A file carrying every kind of thing configure has to preserve:
            // a comment, an unmanaged top-level key, a non-default managed
            // value, an unmanaged key inside a managed table, and an
            // unmanaged table.
            let mut existing = default_toml(tool.key, &[]).unwrap();
            existing = insert_top_level(&existing, "# hand-written\nunmanaged_top = 7");
            existing.push_str("\n[unmanaged_table]\nkey = \"value\"\n");

            let merged = merge_toml(&existing, &enter_through(tool, &existing));
            assert_eq!(merged, existing, "{} rewrote its config", tool.key);
        }
    }

    /// A changed answer has to actually land, and only on its own line.
    #[test]
    fn merging_rewrites_only_the_answered_line() {
        let tool = find("selene").unwrap();
        let existing = "# keep me\nstd = \"roblox\"\nexclude = [\"Packages/**\"]\n\n[rules]\nshadowing = \"warn\"\n";
        let mut answers = enter_through(tool, existing);
        for (spec, value) in answers.iter_mut() {
            if spec.key == "shadowing" {
                *value = json!("allow");
            }
        }
        let merged = merge_toml(existing, &answers);
        assert!(merged.contains("# keep me"), "{merged}");
        assert!(merged.contains(r#"exclude = ["Packages/**"]"#), "{merged}");
        assert!(merged.contains(r#"shadowing = "allow""#), "{merged}");
        assert!(!merged.contains(r#"shadowing = "warn""#), "{merged}");
    }

    /// A setting the file predates has to be added under its own table,
    /// not appended wherever the file happens to end.
    #[test]
    fn a_missing_sectioned_setting_lands_under_its_own_header() {
        let tool = find("selene").unwrap();
        let existing = "std = \"roblox\"\n";
        let merged = merge_toml(existing, &enter_through(tool, existing));
        let parsed = toml::from_str::<toml::Table>(&merged).expect("still valid TOML");
        assert_eq!(parsed["rules"]["shadowing"].as_str(), Some("warn"), "{merged}");
        assert_eq!(parsed["std"].as_str(), Some("roblox"), "{merged}");
    }

    /// An empty or absent file has nothing to merge into.
    #[test]
    fn merging_into_nothing_renders_the_defaults() {
        let tool = find("stylua").unwrap();
        let answers = enter_through(tool, "");
        assert_eq!(merge_toml("", &answers), default_toml("stylua", &[]).unwrap());
    }

    /// Pressing enter through `rproj configure` proposes these, so reading
    /// them back wrong is how a project's own settings get reverted.
    #[test]
    fn current_values_come_from_the_file_not_the_catalog() {
        let tool = find("selene").unwrap();
        let existing = "std = \"roblox+testez\"\n\nexclude = [\"Packages/**\"]\n\n[rules]\nunused_variable = \"allow\"\n";
        let current = current_toml_values(tool, existing);
        assert_eq!(current.len(), tool.settings.len());

        let by_key = |key: &str, section: Option<&str>| {
            let i = tool
                .settings
                .iter()
                .position(|s| s.key == key && s.section == section)
                .expect("setting exists");
            current[i].clone()
        };
        assert_eq!(by_key("std", None), Some(json!("roblox+testez")));
        assert_eq!(by_key("unused_variable", Some("rules")), Some(json!("allow")));
        // Not in the file: the catalog default has to fill in.
        assert_eq!(by_key("shadowing", Some("rules")), None);
    }

    /// Every tool's key must be resolvable, or `rproj configure <key>` and
    /// the `rproj info` cross-link point at nothing.
    #[test]
    fn every_configurable_tool_is_findable_by_key() {
        for tool in CONFIGURABLE_TOOLS {
            assert!(find(tool.key).is_some(), "{} not findable", tool.key);
        }
    }
}
