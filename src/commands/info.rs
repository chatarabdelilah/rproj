//! `rproj info` - the catalog, browsable.
//!
//! Three entry points, one body of detail pages:
//!
//! - `rproj info <key>` prints one entry and exits. Scriptable, and what
//!   every "try next" hint in the rest of rproj points at.
//! - `rproj info` in a terminal opens the full-screen Catalog: pick a section,
//!   then an entry (or type to filter). Replaces printing ~130 catalog rows
//!   and leaving the user to scroll back, find an exact name, and retype it
//!   as an argument - which is a lookup the tool was making the user perform
//!   on its behalf. It also means a detail page gets the whole screen rather
//!   than having to stay terse enough to survive in a list.
//! - `rproj info` with either stream redirected prints the flat listing.
//!   `rproj info > notes.txt`
//!   and CI logs keep working.

use std::io::IsTerminal;

use crate::catalog::artifacts;
use crate::catalog::capabilities;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::tool_usage;
use crate::catalog::wally_packages;
use crate::catalog_view;
use crate::config::Setups;
use anyhow::Result;

pub fn run(key: Option<&str>) -> Result<()> {
    match key {
        Some(key) => show_one(key),
        None if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() => {
            super::catalog_browser::run()
        }
        None => list_all(),
    }
}

// ---------------------------------------------------------------------------
// Detail pages
// ---------------------------------------------------------------------------

/// Full detail on a single catalog entry: description, maintenance status,
/// source/provider, docs. Compare `list_all`, which stays terse on purpose.
fn show_one(key: &str) -> Result<()> {
    if let Some(detail) = catalog_view::lookup(key) {
        println!("{}", detail.plain_text());
    } else {
        println!("No catalog entry named `{key}`. Run `rproj info` with no argument to browse.");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Flat listing (non-interactive)
// ---------------------------------------------------------------------------

/// Terse, categorized listing of the whole catalog - just names and enough
/// to identify each entry (author/version for packages, provider id for
/// tools). No descriptions or maintenance badges here on purpose: run
/// `rproj info <key>` for the full picture on any one entry.
fn list_all() -> Result<()> {
    println!("WALLY PACKAGES");
    for category in wally_packages::Category::ALL {
        println!("  {}", category.label());
        for pkg in wally_packages::in_category(category) {
            println!("    {:<18} {:<18} {}", pkg.key, pkg.author(), pkg.version());
        }
    }

    println!("\nTOOLS");
    for &family in tool_catalog::FAMILY_ORDER {
        let mut entries = tool_catalog::in_family(family).peekable();
        if entries.peek().is_none() {
            continue;
        }
        println!("  {family}");
        for tool in entries {
            println!(
                "    {:<18} {:<32} ({})",
                tool.key,
                tool.kind.provider(),
                tool.kind.label()
            );
        }
    }

    println!("\nCAPABILITIES (what `rproj new` asks about)");
    for capability in capabilities::CAPABILITIES {
        println!(
            "    {:<12} {:<18} {}",
            capability.key,
            capability.default_implementation().display,
            if capability.default_selected {
                "on by default"
            } else {
                "off by default"
            }
        );
    }

    println!("\nGENERATED FILES (rproj info <key> for why a project gets one)");
    for artifact in artifacts::ARTIFACTS {
        // The same cause the detail page reports, so the two views cannot
        // disagree about why a project has a file.
        println!(
            "    {:<24} {}",
            artifact.key,
            catalog_view::cause_line(artifact)
        );
    }

    println!("\nCONFIGURABLE (rproj configure <key>)");
    for tool in tool_settings::CONFIGURABLE_TOOLS {
        println!("    {:<18} {}", tool.key, tool.display_name);
    }

    println!("\nPLACE TEMPLATE (applied to every new project's default.project.json)");
    print_place_template();

    let setups = Setups::list();
    if !setups.is_empty() {
        println!("\nSAVED SETUPS (rproj new <name> --like <setup>)");
        print_saved_setups();
    }

    println!("\nTOPICS");
    for topic in tool_usage::TOPICS {
        println!("    {:<18} {}", topic.key, first_sentence(topic.what));
    }

    println!("\nRun `rproj info <key>` for what it does, the commands to use it, and the gotchas.");
    Ok(())
}

fn print_place_template() {
    println!("{}", catalog_view::place_template_text());
}

fn print_saved_setups() {
    for setup in Setups::list() {
        println!("    {setup}");
    }
}

/// Keeps the listing to one line per entry - the full text is what
/// `rproj info <key>` is for.
fn first_sentence(text: &str) -> &str {
    catalog_view::first_sentence(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_listing_and_details_share_artifact_causes() {
        let artifact = artifacts::find("selene.toml").unwrap();
        let detail = catalog_view::lookup("selene.toml").unwrap();
        assert!(detail.body.contains(&catalog_view::cause_line(artifact)));
    }

    #[test]
    fn catalog_section_labels_are_unique() {
        let sections = catalog_view::sections();
        let mut labels: Vec<&str> = sections.iter().map(|section| section.label()).collect();
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(before, labels.len(), "duplicate section label");
    }
}
