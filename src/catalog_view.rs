use std::fmt::Write;

use crate::catalog::artifacts::{self, Artifact};
use crate::catalog::capabilities;
use crate::catalog::package_usage;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::tool_usage::{self, Usage};
use crate::catalog::wally_packages;

struct DetailSection {
    heading: &'static str,
    text: String,
}

fn detail_sections(sections: Vec<DetailSection>) -> String {
    sections
        .into_iter()
        .filter(|s| !s.text.is_empty())
        .map(|s| format!("## {}\n{}", s.heading, s.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}
use crate::config::Setups;

#[derive(Clone, Debug)]
pub struct CatalogEntry {
    pub key: String,
    pub description: String,
    pub badge: String,
}

#[derive(Clone, Debug)]
pub struct CatalogDetail {
    pub title: String,
    pub body: String,
}

impl CatalogDetail {
    pub fn plain_text(&self) -> String {
        format!(
            "{}\n{}\n{}",
            self.title,
            "-".repeat(self.title.chars().count()),
            self.body
        )
    }
}

#[derive(Clone, Debug)]
pub enum CatalogSection {
    Entries {
        label: String,
        entries: Vec<CatalogEntry>,
    },
    Page {
        label: String,
        detail: CatalogDetail,
    },
}

#[cfg(test)]
impl CatalogSection {
    pub fn label(&self) -> &str {
        match self {
            Self::Entries { label, .. } | Self::Page { label, .. } => label,
        }
    }
}

pub fn sections() -> Vec<CatalogSection> {
    let mut sections = vec![
        CatalogSection::Entries {
            label: "Packages - libraries a project depends on".into(),
            entries: wally_packages::PACKAGES
                .iter()
                .map(|package| CatalogEntry {
                    key: package.key.into(),
                    description: package.description.into(),
                    badge: package.category.label().into(),
                })
                .collect(),
        },
        CatalogSection::Entries {
            label: "Tools, apps, plugins & extensions - what gets installed".into(),
            entries: tool_catalog::FAMILY_ORDER
                .iter()
                .flat_map(|family| tool_catalog::in_family(family))
                .map(|tool| CatalogEntry {
                    key: tool.key.into(),
                    description: tool.description.into(),
                    badge: tool.kind.label().into(),
                })
                .collect(),
        },
        CatalogSection::Entries {
            label: "Capabilities - what a project can be set up to do".into(),
            entries: capabilities::CAPABILITIES
                .iter()
                .map(|capability| CatalogEntry {
                    key: capability.key.into(),
                    description: capability.outcome.into(),
                    badge: capability.default_implementation().display.into(),
                })
                .collect(),
        },
        CatalogSection::Entries {
            label: "Generated files - what `rproj new` can write, and why".into(),
            entries: artifacts::ARTIFACTS
                .iter()
                .map(|artifact| CatalogEntry {
                    key: artifact.key.into(),
                    description: artifact.description.into(),
                    badge: artifact.category.label().into(),
                })
                .collect(),
        },
        CatalogSection::Entries {
            label: "Topics - how the pieces fit together".into(),
            entries: tool_usage::TOPICS
                .iter()
                .map(|topic| CatalogEntry {
                    key: topic.key.into(),
                    description: first_sentence(topic.what).into(),
                    badge: "topic".into(),
                })
                .collect(),
        },
        CatalogSection::Entries {
            label: "Configurable - settings rproj can edit for you".into(),
            entries: tool_settings::CONFIGURABLE_TOOLS
                .iter()
                .map(|tool| CatalogEntry {
                    key: tool.key.into(),
                    description: format!(
                        "{} - edit with `rproj configure {}`",
                        tool.display_name, tool.key
                    ),
                    badge: "configure".into(),
                })
                .collect(),
        },
    ];
    let setups = Setups::list();
    if !setups.is_empty() {
        sections.push(CatalogSection::Page {
            label: "Saved setups - package compositions you saved".into(),
            detail: CatalogDetail {
                title: "Saved setups".into(),
                body: setups
                    .into_iter()
                    .map(|setup| format!("  {setup}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            },
        });
    }
    sections
}

pub fn lookup(key: &str) -> Option<CatalogDetail> {
    if let Some(package) = wally_packages::find(key) {
        let guide = package_usage::find(key)?;
        let mut placement = format!(
            "{}\n\nWally imports use the generated aliases below. Select: {}.",
            guide.context,
            if guide.imports.is_empty() {
                package.key.into()
            } else {
                guide.imports.join(", ")
            }
        );
        let dependencies = guide.imports.iter().map(|key| (*key).to_owned()).collect();
        if guide.imports.is_empty() {
            placement = format!(
                "{}\n\nSelect Testing with TestEZ. These specs work across Wally, Git submodules, and no-manager projects through rproj test; no direct package import is needed.",
                guide.context
            );
        } else if package.submodule.is_some()
            && wally_packages::unvendorable_in_closure(&dependencies).is_empty()
        {
            placement.push_str("\n\nGit submodules: replace the Wally imports with:\n");
            for dependency in guide.imports {
                placement.push_str(&package_usage::import(
                    wally_packages::find(dependency).expect("guide dependency"),
                    true,
                ));
                placement.push('\n');
            }
            placement.push_str("Submodules track repository commits, not these Wally versions; verify the checked-out API. No-manager projects do not install this package.");
        } else {
            placement.push_str("\n\nUse Wally: this example's dependency tree is not supported by rproj's submodule workflow. No-manager projects do not install it.");
        }
        let body = detail_sections(vec![
            DetailSection {
                heading: "Purpose",
                text: package.description.into(),
            },
            DetailSection {
                heading: "When useful",
                text: guide.when.into(),
            },
            DetailSection {
                heading: "Requirements / placement",
                text: placement,
            },
            DetailSection {
                heading: "Example",
                text: package_usage::example(&guide),
            },
            DetailSection {
                heading: "Caveats",
                text: guide.caveats.into(),
            },
            DetailSection {
                heading: "Package",
                text: format!(
                    "{}\n{}\n{}",
                    package.source,
                    package.category.label(),
                    package.maintenance.badge()
                ),
            },
            DetailSection {
                heading: "Official documentation",
                text: package.docs_url.into(),
            },
        ]);
        return Some(CatalogDetail {
            title: package.key.into(),
            body,
        });
    }
    if let Some(tool) = tool_catalog::find(key) {
        let mut body = format!(
            "## Purpose\n{}\n\n## Tool\nfamily:   {}\nkind:     {}\nprovider: {}\nstatus:   {}\n\n## Official documentation\n{}",
            tool.description,
            tool.family,
            tool.kind.label(),
            tool.kind.provider(),
            tool.maintenance.badge(),
            tool.docs_url
        );
        if let Some(usage) = tool_usage::find(key) {
            append_guidance(&mut body, usage);
        }
        if key.starts_with("theme-") {
            body.push_str("\n\n## Usage steps\nIn VS Code, open the Command Palette and choose Preferences: Color Theme");
            if key.ends_with("-icons") {
                body.push_str(" (use Preferences: File Icon Theme for this icon set)");
            }
            body.push_str(
                ". Select the installed theme. This changes the editor, not the game's UI.",
            );
        }
        if tool_settings::find(key).is_some() {
            let _ = write!(body, "\n\nConfigure: rproj configure {key}");
        }
        return Some(CatalogDetail {
            title: tool.key.into(),
            body,
        });
    }
    if let Some(capability) = capabilities::find(key) {
        let mut body = format!("provides: {}", capability.outcome);
        if !capability.requires.is_empty() {
            let _ = write!(body, "\nneeds:    {}", capability.requires.join(", "));
        }
        let _ = write!(
            body,
            "\ndefault:  {}",
            if capability.default_selected {
                "on"
            } else {
                "off"
            }
        );
        for implementation in capability.implementations {
            let _ = write!(
                body,
                "\n\n  {} ({})",
                implementation.display, implementation.key
            );
            if !implementation.tools.is_empty() {
                let _ = write!(body, "\n    pins:    {}", implementation.tools.join(", "));
            }
            if !implementation.packages.is_empty() {
                let _ = write!(
                    body,
                    "\n    adds:    {}",
                    implementation.packages.join(", ")
                );
            }
            if !implementation.artifacts.is_empty() {
                let _ = write!(
                    body,
                    "\n    writes:  {}",
                    implementation.artifacts.join(", ")
                );
            }
            if tool_catalog::find(implementation.key).is_some()
                || tool_usage::find(implementation.key).is_some()
            {
                let _ = write!(body, "\n    more:    rproj info {}", implementation.key);
            }
        }
        if capability.implementations.len() > 1 {
            body.push_str(
                "\n\n`rproj new` asks which of these to use, because there is a real choice.",
            );
        }
        return Some(CatalogDetail {
            title: capability.key.into(),
            body,
        });
    }
    if let Some(artifact) = artifacts::find(key) {
        let mut body = format!(
            "category: {}\nwritten:  {}",
            artifact.category.label(),
            cause_line(artifact)
        );
        if !artifact.also_requires.is_empty() {
            let needs = artifact
                .also_requires
                .iter()
                .map(|requirement| requirement.describe())
                .collect::<Vec<_>>();
            let _ = write!(body, "\nneeds:    {}", needs.join(", and "));
        }
        let _ = write!(body, "\n\n{}", artifact.description);
        if let Some(owner) = owning_capability(artifact.key) {
            let _ = write!(
                body,
                "\n\nTo not have this file, don't choose `{owner}`.\nRun `rproj info {owner}` for what that capability does."
            );
        }
        return Some(CatalogDetail {
            title: artifact.key.into(),
            body,
        });
    }
    tool_usage::find(key).map(|usage| {
        let mut body = String::new();
        append_usage(&mut body, usage);
        CatalogDetail {
            title: usage.key.into(),
            body: body.trim_start().to_string(),
        }
    })
}

fn append_usage(body: &mut String, usage: &Usage) {
    let _ = write!(body, "## Purpose\n{}", usage.what);
    append_guidance(body, usage);
}

fn append_guidance(body: &mut String, usage: &Usage) {
    let _ = write!(body, "\n\n## When useful\n{}", usage.when);
    if !usage.commands.is_empty() {
        body.push_str("\n\n## Usage steps");
        let width = usage
            .commands
            .iter()
            .map(|(command, _)| command.len())
            .max()
            .unwrap_or(0);
        for (command, explanation) in usage.commands {
            let _ = write!(body, "\n  {command:<width$}  {explanation}");
        }
    }
    if !usage.notes.is_empty() {
        body.push_str("\n\n## Caveats");
        for note in usage.notes {
            let _ = write!(body, "\n  - {note}");
        }
    }
}

pub fn cause_line(artifact: &Artifact) -> String {
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
        "modules" => "when this project vendors packages as git submodules".into(),
        "rokit.toml" => "when anything pins a tool version".into(),
        _ => "derived from your answers".into(),
    }
}

fn owning_capability(key: &str) -> Option<&'static str> {
    capabilities::CAPABILITIES
        .iter()
        .find(|capability| {
            capability
                .implementations
                .iter()
                .any(|implementation| implementation.artifacts.contains(&key))
        })
        .map(|capability| capability.key)
}

pub fn first_sentence(text: &str) -> &str {
    match text.find(". ") {
        Some(index) => &text[..=index],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_resolves_to_the_same_detail_model() {
        for section in sections() {
            if let CatalogSection::Entries { label, entries } = section {
                assert!(!entries.is_empty(), "{label} has no entries");
                for entry in entries {
                    assert!(lookup(&entry.key).is_some(), "{} in {label}", entry.key);
                }
            }
        }
    }

    #[test]
    fn direct_detail_keeps_the_plain_contract() {
        let detail = lookup("rojo").expect("rojo");
        let text = detail.plain_text();
        assert!(text.starts_with("rojo\n----"));
        assert!(text.contains("rojo serve"));
    }

    #[test]
    fn artifact_causes_are_actionable() {
        for artifact in artifacts::ARTIFACTS {
            assert_ne!(cause_line(artifact), "derived from your answers");
        }
    }
}
