//! Bundled, read-only guidance. Identity includes kind because catalog keys can overlap.

use super::{
    CatalogEntry, artifacts, capabilities, package_usage, tool_catalog, tool_settings, tool_usage,
    wally_packages,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    Package,
    Tool,
    Capability,
    File,
    Settings,
    Topic,
}

impl Kind {
    pub const ALL: [Self; 6] = [
        Self::Package,
        Self::Tool,
        Self::Capability,
        Self::File,
        Self::Settings,
        Self::Topic,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Package => "Packages",
            Self::Tool => "Tools",
            Self::Capability => "Capabilities",
            Self::File => "Generated files",
            Self::Settings => "Configurable settings",
            Self::Topic => "Topics",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryId {
    pub kind: Kind,
    pub key: String,
}
impl EntryId {
    pub fn new(kind: Kind, key: impl Into<String>) -> Self {
        Self {
            kind,
            key: key.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: EntryId,
    pub name: String,
    pub description: String,
    pub category: String,
    pub kind_label: String,
}
impl Entry {
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        [
            &self.name,
            &self.id.key,
            &self.description,
            &self.category,
            &self.kind_label,
        ]
        .into_iter()
        .any(|value| value.to_lowercase().contains(&query))
            || self.id.kind.label().to_lowercase().contains(&query)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub heading: String,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Example {
    pub label: String,
    pub code: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    pub url: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Guide {
    pub entry: Entry,
    pub sections: Vec<Section>,
    pub examples: Vec<Example>,
    pub links: Vec<Link>,
    /// Navigation only: related entries are not unconditional installation requirements.
    pub related: Vec<EntryId>,
}
impl Guide {
    fn section(&mut self, heading: impl Into<String>, text: impl Into<String>) {
        let text = text.into();
        if !text.is_empty() {
            self.sections.push(Section {
                heading: heading.into(),
                text,
            });
        }
    }
    fn link(&mut self, label: &str, url: &str) {
        if !url.is_empty() {
            self.links.push(Link {
                label: label.into(),
                url: url.into(),
            });
        }
    }
    fn usage(&mut self, usage: &tool_usage::Usage) {
        self.section("Purpose", usage.what);
        self.section("When useful", usage.when);
        for (command, explanation) in usage.commands {
            if command.starts_with("https://") || command.starts_with("http://") {
                self.link(explanation, command);
            } else {
                self.section(*command, *explanation);
            }
        }
        self.section("Caveats", usage.notes.join("\n\n"));
    }
}

/// All six inventories, sorted by section and display name with stable identity tie-breaks.
pub fn entries() -> Vec<Entry> {
    let mut out = Vec::new();
    let mut push =
        |kind, key: &str, name: &str, description: &str, category: &str, kind_label: &str| {
            out.push(Entry {
                id: EntryId::new(kind, key),
                name: name.into(),
                description: description.into(),
                category: category.into(),
                kind_label: kind_label.into(),
            });
        };
    for p in wally_packages::PACKAGES {
        push(
            Kind::Package,
            p.key,
            p.module_name,
            p.description,
            p.category.label(),
            "Package",
        );
    }
    for t in tool_catalog::all_setup_entries() {
        let name = match t.key {
            "scribe-studio" => "Scribe Studio",
            "stylua" => "StyLua",
            _ => t.key,
        };
        push(
            Kind::Tool,
            t.key,
            name,
            t.description,
            t.family,
            t.kind.label(),
        );
    }
    for c in capabilities::CAPABILITIES {
        push(
            Kind::Capability,
            c.key,
            c.key,
            c.outcome,
            "Project capabilities",
            "Capability",
        );
    }
    for f in artifacts::ARTIFACTS {
        push(
            Kind::File,
            f.key,
            f.key,
            f.description,
            f.category.label(),
            "Generated file",
        );
    }
    for s in tool_settings::CONFIGURABLE_TOOLS {
        push(
            Kind::Settings,
            s.key,
            s.display_name,
            s.summary,
            "Tool configuration",
            "Configurable settings",
        );
    }
    for t in tool_usage::TOPICS {
        push(
            Kind::Topic,
            t.key,
            t.key,
            t.what,
            "Quality workflow",
            "Topic",
        );
    }
    out.sort_by_key(|entry| {
        (
            entry.id.kind,
            entry.name.to_lowercase(),
            entry.id.key.clone(),
        )
    });
    out
}

/// Complete structured content built from authoritative metadata, never terminal text.
pub fn guide(id: &EntryId) -> Option<Guide> {
    let entry = entries().into_iter().find(|entry| &entry.id == id)?;
    let mut g = Guide {
        entry,
        sections: Vec::new(),
        examples: Vec::new(),
        links: Vec::new(),
        related: related(id),
    };
    let key = id.key.as_str();
    match id.kind {
        Kind::Package => {
            let p = wally_packages::find(key)?;
            if let Some(usage) = package_usage::find(key) {
                g.section("When useful", usage.when);
                g.section("Where the code belongs", usage.context);
                let requirements = if usage.imports.is_empty() {
                    "Select Testing with TestEZ. These specs work across Wally and no-manager projects through rproj test; no direct package import is needed.".into()
                } else {
                    format!(
                        "Select Wally and: {}. Examples use the catalog's default aliases; adapt them to your project's declarations. No-manager projects do not install packages.",
                        usage.imports.join(", ")
                    )
                };
                g.section("Example requirements", requirements);
                g.section("Caveats", usage.caveats);
                g.examples.push(Example {
                    label: "Luau example".into(),
                    code: package_usage::example(&usage),
                });
            }
            g.section(
                "Catalog declaration",
                format!(
                    "{}\nRealm: {}\n{}",
                    p.source,
                    match p.realm {
                        wally_packages::Realm::Shared => "Shared",
                        wally_packages::Realm::Server => "Server",
                        wally_packages::Realm::Dev => "Development",
                    },
                    p.maintenance.badge()
                ),
            );
            g.link("Documentation", p.docs_url);
            g.link("Upstream repository", p.git_repo);
        }
        Kind::Tool => {
            let t = tool_catalog::find(key)?;
            g.section(
                "Provider",
                format!(
                    "{}\n{}\n{}",
                    t.kind.provider(),
                    t.kind.label(),
                    t.maintenance.badge()
                ),
            );
            g.link("Documentation", t.docs_url);
            if let Some(usage) = tool_usage::find(key) {
                g.usage(usage);
            }
            if let tool_catalog::ToolKind::StudioPluginManual { install_url, .. } = t.kind {
                if !g.links.iter().any(|link| link.url == install_url) {
                    g.link("Manual installation", install_url);
                }
                g.section("Manual installation","rproj supplies instructions; it cannot install or reliably detect this marketplace plugin.");
            }
            if key.starts_with("theme-") {
                g.section(
                    "VS Code appearance",
                    if key.ends_with("-icons") {
                        "Use Preferences: File Icon Theme in VS Code after installation."
                    } else {
                        "Use Preferences: Color Theme in VS Code after installation."
                    },
                );
                g.section("Scope", "This changes the editor, not the game's UI.");
            }
        }
        Kind::Capability => {
            let c = capabilities::find(key)?;
            g.section(
                "Requirements",
                if c.requires.is_empty() {
                    "No other capability is required.".into()
                } else {
                    c.requires.join(", ")
                },
            );
            g.section(
                "Selection",
                "Off by default; selected explicitly in project composition.",
            );
            for i in c.implementations {
                g.section(
                    i.display,
                    format!(
                        "Implementation: {}\nWorkflows: {}\nTools: {}\nPackages: {}\nFiles: {}",
                        i.key,
                        i.workflows
                            .iter()
                            .map(|w| match w {
                                crate::creation::PackageWorkflow::Wally => "Wally",
                                crate::creation::PackageWorkflow::None => "None",
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                        named(i.tools),
                        named(i.packages),
                        named(i.artifacts)
                    ),
                );
            }
        }
        Kind::File => {
            let f = artifacts::find(key)?;
            g.section("Why it is generated", file_reason(f));
            g.section(
                "Additional requirements",
                f.also_requires
                    .iter()
                    .map(|r| r.describe())
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
            g.section("File presence","Catalog guidance describes generation, not whether this file exists in an inspected project.");
        }
        Kind::Settings => {
            let s = tool_settings::find(key)?;
            let destination = match s.target {
                tool_settings::ConfigTarget::ProjectToml { filename } => filename,
                tool_settings::ConfigTarget::VsCodeSettings => ".vscode/settings.json",
            };
            g.section("Destination", destination);
            g.link("Documentation", s.docs_url);
            g.section("Editing","Suggested defaults below are not current project values. Configure tool settings… provides explicit edits and reviewed saving; it does not install or enable a tool.");
            for setting in s.settings {
                let accepted = match &setting.kind {
                    tool_settings::SettingKind::Bool { .. } => "true or false".into(),
                    tool_settings::SettingKind::Integer { .. } => {
                        "Signed 64-bit whole number".into()
                    }
                    tool_settings::SettingKind::Choice { options, .. } => options
                        .iter()
                        .map(|o| format!("{} — {}", o.value, o.explanation))
                        .collect::<Vec<_>>()
                        .join("\n"),
                };
                g.section(
                    setting.display_key(),
                    format!(
                        "{}\nSuggested default: {}\nAccepted values: {accepted}",
                        setting.description,
                        setting.kind.default_value()
                    ),
                );
            }
        }
        Kind::Topic => g.usage(tool_usage::TOPICS.iter().find(|usage| usage.key == key)?),
    }
    Some(g)
}

fn named(keys: &[&str]) -> String {
    if keys.is_empty() {
        "None".into()
    } else {
        keys.join(", ")
    }
}

pub fn owning_capability(key: &str) -> Option<&'static str> {
    capabilities::CAPABILITIES
        .iter()
        .find(|c| c.implementations.iter().any(|i| i.artifacts.contains(&key)))
        .map(|c| c.key)
}

/// Retains the existing terminal cause wording and precedence.
pub fn file_reason(artifact: &artifacts::Artifact) -> String {
    if artifact.mandatory {
        return "always - every Rojo project has this".into();
    }
    if artifact.housekeeping {
        return "always - written for every project, droppable from the summary".into();
    }
    if let Some(owner) = owning_capability(artifact.key) {
        return format!("when you choose the `{owner}` capability");
    }
    match artifact.key {
        "wally.toml" => "when this project uses Wally".into(),
        "rokit.toml" => "when anything pins a tool version".into(),
        _ => "derived from your answers".into(),
    }
}

/// Same package-only projection used by the existing private worker and detail surfaces.
pub fn package_entries() -> anyhow::Result<Vec<CatalogEntry>> {
    use anyhow::Context;
    wally_packages::PACKAGES
        .iter()
        .map(|p| {
            let guide = package_usage::find(p.key)
                .with_context(|| format!("missing catalog guidance for {}", p.key))?;
            let (identity, _) = p
                .source
                .split_once('@')
                .with_context(|| format!("invalid catalog package identity for {}", p.key))?;
            Ok(CatalogEntry {
                key: p.key.into(),
                title: p.module_name.into(),
                description: p.description.into(),
                docs_url: p.docs_url.into(),
                package: identity.into(),
                example: package_usage::example(&guide),
                when_useful: guide.when.into(),
                context: guide.context.into(),
                caveats: guide.caveats.into(),
            })
        })
        .collect()
}

fn related(id: &EntryId) -> Vec<EntryId> {
    let all: BTreeSet<_> = entries().into_iter().map(|e| e.id).collect();
    let mut result = BTreeSet::new();
    let mut edge = |a: EntryId, b: EntryId| {
        if a != b && all.contains(&a) && all.contains(&b) {
            if &a == id {
                result.insert(b.clone());
            }
            if &b == id {
                result.insert(a);
            }
        }
    };
    for c in capabilities::CAPABILITIES {
        let c_id = EntryId::new(Kind::Capability, c.key);
        for requirement in c.requires {
            edge(c_id.clone(), EntryId::new(Kind::Capability, *requirement));
        }
        for i in c.implementations {
            for (kind, keys) in [
                (Kind::Tool, i.tools),
                (Kind::Package, i.packages),
                (Kind::File, i.artifacts),
            ] {
                for key in keys {
                    edge(c_id.clone(), EntryId::new(kind, *key));
                }
            }
            // Some implementations are apps rather than pinned CLI tools (e.g. Blender).
            edge(c_id.clone(), EntryId::new(Kind::Tool, i.key));
        }
    }
    for t in tool_catalog::all_setup_entries() {
        for sibling in tool_catalog::in_family(t.family) {
            // Broad inventory buckets are not product families.
            if !matches!(t.family, "System apps" | "Testing & extras") {
                edge(
                    EntryId::new(Kind::Tool, t.key),
                    EntryId::new(Kind::Tool, sibling.key),
                );
            }
        }
    }
    for s in tool_settings::CONFIGURABLE_TOOLS {
        let settings = EntryId::new(Kind::Settings, s.key);
        edge(settings.clone(), EntryId::new(Kind::Tool, s.key));
        for file in s.requires_artifacts {
            edge(settings.clone(), EntryId::new(Kind::File, *file));
        }
    }
    for p in wally_packages::PACKAGES {
        if let Some(usage) = package_usage::find(p.key) {
            for import in usage.imports {
                edge(
                    EntryId::new(Kind::Package, p.key),
                    EntryId::new(Kind::Package, *import),
                );
            }
        }
    }
    for f in artifacts::ARTIFACTS {
        for requirement in f.also_requires {
            let other = match requirement {
                artifacts::Requirement::Capability(key) => {
                    Some(EntryId::new(Kind::Capability, *key))
                }
                artifacts::Requirement::App(key) => Some(EntryId::new(Kind::Tool, *key)),
                artifacts::Requirement::Strategy(_) => None,
            };
            if let Some(other) = other {
                edge(EntryId::new(Kind::File, f.key), other);
            }
        }
    }
    for (a, b) in [
        ((Kind::Package, "scribe"), (Kind::Package, "profilestore")),
        ((Kind::Package, "scribe"), (Kind::Tool, "scribe-studio")),
        (
            (Kind::Package, "profilestore"),
            (Kind::Tool, "scribe-studio"),
        ),
        ((Kind::Topic, "ci"), (Kind::Capability, "ci")),
        ((Kind::Topic, "check"), (Kind::Capability, "gate")),
        ((Kind::Topic, "ci"), (Kind::Topic, "check")),
    ] {
        edge(EntryId::new(a.0, a.1), EntryId::new(b.0, b.1));
    }
    result.into_iter().collect()
}

#[cfg(test)]
mod tests;
