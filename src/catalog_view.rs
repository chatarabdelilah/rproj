use std::fmt::Write;

use crate::catalog::artifacts::{self, Artifact};
use crate::catalog::capabilities;
use crate::catalog::place_template::PLACE_TEMPLATE;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::tool_usage::{self, Usage};
use crate::catalog::wally_packages;
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
        CatalogSection::Page {
            label: "Place template - the instance tree every project starts with".into(),
            detail: CatalogDetail {
                title: "Place template".into(),
                body: place_template_text(),
            },
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
        let mut body = format!(
            "category: {}\nstatus:   {}\nsource:   {}\ndocs:     {}\n\n{}",
            package.category.label(),
            package.maintenance.badge(),
            package.source,
            package.docs_url,
            package.description
        );
        if let Some(usage) = tool_usage::find(key) {
            append_usage(&mut body, usage);
        }
        return Some(CatalogDetail {
            title: package.key.into(),
            body,
        });
    }
    if let Some(tool) = tool_catalog::find(key) {
        let mut body = format!(
            "family:   {}\nkind:     {}\nprovider: {}\nstatus:   {}\ndocs:     {}\n\n{}",
            tool.family,
            tool.kind.label(),
            tool.kind.provider(),
            tool.maintenance.badge(),
            tool.docs_url,
            tool.description
        );
        if let Some(usage) = tool_usage::find(key) {
            append_usage(&mut body, usage);
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
    let _ = write!(body, "\n\n{}\n\nWhen: {}", usage.what, usage.when);
    if !usage.commands.is_empty() {
        body.push_str("\n\nCommands:");
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
        body.push_str("\n\nWorth knowing:");
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

pub fn place_template_text() -> String {
    let mut text = String::new();
    for spec in PLACE_TEMPLATE {
        let location = match spec.parent {
            Some(parent) => format!("{parent}.{}", spec.name),
            None => spec.name.into(),
        };
        let _ = writeln!(text, "  {location} ({})", spec.class_name);
        for property in spec.properties {
            let _ = writeln!(
                text,
                "    {:<26} {}",
                property.name,
                property.value.display()
            );
        }
    }
    text.trim_end().into()
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
