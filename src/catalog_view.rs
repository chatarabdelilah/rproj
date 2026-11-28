use std::fmt::Write;

use crate::catalog::artifacts;
use crate::catalog::capabilities;
use crate::catalog::package_usage;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::tool_usage::{self, Usage};
use crate::catalog::wally_packages;
pub use rproj_core::catalog::browser::{file_reason as cause_line, owning_capability};

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
        if guide.imports.is_empty() {
            placement = format!(
                "{}\n\nSelect Testing with TestEZ. These specs work across Wally and no-manager projects through rproj test; no direct package import is needed.",
                guide.context
            );
        } else {
            placement.push_str("\n\nSelect Wally to install these dependencies. No-manager projects do not install packages.");
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
            DetailSection {
                heading: "Upstream repository",
                text: package.git_repo.into(),
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
        body.push_str("\ndefault:  off (selected explicitly)");
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
