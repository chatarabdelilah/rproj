//! `rproj info` - the catalog, browsable.
//!
//! Three entry points, one body of detail pages:
//!
//! - `rproj info <key>` prints one entry and exits. Scriptable, and what
//!   every "try next" hint in the rest of rproj points at.
//! - `rproj info` in a terminal opens a two-level menu: pick a section, then
//!   pick an entry (or type to filter). Replaces printing ~130 catalog rows
//!   and leaving the user to scroll back, find an exact name, and retype it
//!   as an argument - which is a lookup the tool was making the user perform
//!   on its behalf. It also means a detail page gets the whole screen rather
//!   than having to stay terse enough to survive in a list.
//! - `rproj info` with stdin redirected prints the flat listing, because a
//!   prompt with nowhere to read from is a crash. `rproj info > notes.txt`
//!   and CI logs keep working.

use std::io::IsTerminal;

use anyhow::Result;
use inquire::{InquireError, Select};

use crate::catalog::artifacts::{self, Artifact};
use crate::catalog::capabilities;
use crate::catalog::place_template::PLACE_TEMPLATE;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::tool_usage::{self, Usage};
use crate::catalog::wally_packages;
use crate::config::SavedSetup;
use crate::ui;

pub fn run(key: Option<&str>) -> Result<()> {
    match key {
        Some(key) => show_one(key),
        None if std::io::stdin().is_terminal() => browse(),
        None => list_all(),
    }
}

// ---------------------------------------------------------------------------
// Browsing
// ---------------------------------------------------------------------------

/// One row in a section's list: what the picker shows, and what selecting it
/// looks up.
struct Row {
    key: String,
    description: String,
    badge: String,
}

/// A top-level menu choice.
enum Section {
    /// A filterable list of entries, each with a detail page.
    Entries { label: &'static str, rows: Vec<Row> },
    /// A single page with nothing to pick inside it.
    Page { label: &'static str, render: fn() },
}

impl Section {
    fn label(&self) -> &'static str {
        match self {
            Section::Entries { label, .. } | Section::Page { label, .. } => label,
        }
    }
}

fn sections() -> Vec<Section> {
    let mut sections = vec![
        Section::Entries {
            label: "Packages - libraries a project depends on",
            rows: wally_packages::PACKAGES
                .iter()
                .map(|p| Row {
                    key: p.key.to_string(),
                    description: p.description.to_string(),
                    badge: p.category.label().to_string(),
                })
                .collect(),
        },
        Section::Entries {
            label: "Tools, apps, plugins & extensions - what gets installed",
            // Grouped by family in the source order of FAMILY_ORDER, so
            // Rojo's CLI tool, Studio plugin and VS Code extension sit
            // together rather than being scattered across three kinds.
            rows: tool_catalog::FAMILY_ORDER
                .iter()
                .flat_map(|family| tool_catalog::in_family(family))
                .map(|t| Row {
                    key: t.key.to_string(),
                    description: t.description.to_string(),
                    badge: t.kind.label().to_string(),
                })
                .collect(),
        },
        Section::Entries {
            label: "Capabilities - what a project can be set up to do",
            rows: capabilities::CAPABILITIES
                .iter()
                .map(|c| Row {
                    key: c.key.to_string(),
                    description: c.outcome.to_string(),
                    // The tool, always. A capability that hides what provides
                    // it teaches nothing about the ecosystem.
                    badge: c.default_implementation().display.to_string(),
                })
                .collect(),
        },
        Section::Entries {
            label: "Generated files - what `rproj new` can write, and why",
            rows: artifacts::ARTIFACTS
                .iter()
                .map(|a| Row {
                    key: a.key.to_string(),
                    description: a.description.to_string(),
                    badge: a.category.label().to_string(),
                })
                .collect(),
        },
        Section::Entries {
            label: "Topics - how the pieces fit together",
            rows: tool_usage::TOPICS
                .iter()
                .map(|t| Row {
                    key: t.key.to_string(),
                    description: first_sentence(t.what).to_string(),
                    badge: "topic".to_string(),
                })
                .collect(),
        },
        Section::Entries {
            label: "Configurable - settings rproj can edit for you",
            rows: tool_settings::CONFIGURABLE_TOOLS
                .iter()
                .map(|t| Row {
                    key: t.key.to_string(),
                    description: format!("{} - edit with `rproj configure {}`", t.display_name, t.key),
                    badge: "configure".to_string(),
                })
                .collect(),
        },
        Section::Page {
            label: "Place template - the instance tree every project starts with",
            render: print_place_template,
        },
    ];

    // Only when there are any: an empty section is a dead end that teaches
    // the user nothing about why it is empty.
    if !SavedSetup::list().is_empty() {
        sections.push(Section::Page {
            label: "Saved setups - package compositions you saved",
            render: print_saved_setups,
        });
    }
    sections
}

/// Section menu -> entry list -> detail page, looping until the user leaves.
///
/// Detail pages print above the next prompt rather than replacing it, so the
/// page stays readable while the menu returns underneath. That is also why
/// there is no "press enter to continue" step: the next menu *is* the
/// continue, and an extra keystroke per lookup is the friction this command
/// exists to remove.
fn browse() -> Result<()> {
    let sections = sections();
    println!("The rproj catalog. Pick a section, then type to filter.\n");

    loop {
        let labels: Vec<String> = sections.iter().map(|s| s.label().to_string()).collect();
        let Some(picked) = ask("What do you want to look up?", labels, false)? else {
            return Ok(());
        };
        let Some(section) = sections.iter().find(|s| s.label() == picked) else {
            return Ok(());
        };

        match section {
            Section::Page { render, .. } => {
                println!();
                render();
                println!();
            }
            Section::Entries { rows, label } => browse_entries(label, rows)?,
        }
    }
}

/// The entry list for one section. Stays here until the user backs out, so
/// looking up four packages is four selections rather than four round trips
/// through the section menu.
fn browse_entries(label: &str, rows: &[Row]) -> Result<()> {
    const BACK: &str = "← back to the sections";
    loop {
        let mut options: Vec<String> = rows
            .iter()
            .map(|r| ui::option_line(&r.key, &r.description, &r.badge))
            .collect();
        options.push(BACK.to_string());

        let Some(picked) = ask(label, options, true)? else {
            return Ok(());
        };
        if picked == BACK {
            return Ok(());
        }
        println!();
        show_one(ui::option_key(&picked))?;
        println!();
    }
}

/// A `Select` that treats Esc and Ctrl-C as "I'm done" rather than an error.
///
/// Leaving a browser is not a failure, and reporting it as one would print a
/// red error line with a cause chain for the ordinary way out.
fn ask(prompt: &str, options: Vec<String>, compact_echo: bool) -> Result<Option<String>> {
    let mut select = Select::new(prompt, options).with_page_size(ui::page_size());
    if compact_echo {
        // Echo just the key, not the whole `key - description (badge)` line -
        // the detail page below it is about to repeat all of that.
        select = select.with_formatter(&ui::compact_select_answer);
    }
    match select.prompt() {
        Ok(choice) => Ok(Some(choice)),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

// ---------------------------------------------------------------------------
// Detail pages
// ---------------------------------------------------------------------------

/// Full detail on a single catalog entry: description, maintenance status,
/// source/provider, docs. Compare `list_all`, which stays terse on purpose.
fn show_one(key: &str) -> Result<()> {
    if let Some(pkg) = wally_packages::find(key) {
        heading(pkg.key);
        println!("category: {}", pkg.category.label());
        println!("status:   {}", pkg.maintenance.badge());
        println!("source:   {}", pkg.source);
        println!("docs:     {}", pkg.docs_url);
        println!();
        println!("{}", pkg.description);
        if let Some(usage) = tool_usage::find(key) {
            print_usage(usage);
        }
        return Ok(());
    }
    if let Some(tool) = tool_catalog::find(key) {
        heading(tool.key);
        println!("family:   {}", tool.family);
        println!("kind:     {}", tool.kind.label());
        println!("provider: {}", tool.kind.provider());
        println!("status:   {}", tool.maintenance.badge());
        println!("docs:     {}", tool.docs_url);
        println!();
        println!("{}", tool.description);
        if let Some(usage) = tool_usage::find(key) {
            print_usage(usage);
        }
        if tool_settings::find(key).is_some() {
            println!("\nConfigure: rproj configure {key}");
        }
        return Ok(());
    }
    // Before artifacts, because a capability is the level a user is more
    // likely to be asking about - "what is lint" beats "what is selene.toml".
    if let Some(capability) = capabilities::find(key) {
        print_capability(capability);
        return Ok(());
    }
    // Checked after tools and packages: an artifact key like `wally.toml`
    // can't collide with either, and putting it last keeps the common
    // lookups first.
    if let Some(artifact) = artifacts::find(key) {
        print_artifact(artifact);
        return Ok(());
    }
    if let Some(usage) = tool_usage::find(key) {
        heading(usage.key);
        print_usage(usage);
        return Ok(());
    }
    println!("No catalog entry named `{key}`. Run `rproj info` with no argument to browse.");
    Ok(())
}

fn heading(title: &str) {
    println!("{title}");
    println!("{}", "-".repeat(title.chars().count()));
}

/// Why a project has this file - which is the question the artifact model
/// created and nothing answered until now. `requires` says when it can be
/// offered at all; `entailed_by` says when it stops being a question, and
/// carries the reason the picker prints.
/// Why a project has this file, in one line.
///
/// No artifact is asked about directly any more, so this names what *causes*
/// it instead: a capability, the dependency strategy, or the fact that every
/// project gets it. That is the question the artifact model created and
/// nothing answered until the capability layer existed.
fn cause_line(artifact: &Artifact) -> String {
    if artifact.mandatory {
        return "always - every Rojo project has this".to_string();
    }
    if artifact.housekeeping {
        return "always - written for every project, droppable from the summary".to_string();
    }
    if let Some(owner) = owning_capability(artifact.key) {
        return format!("when you choose the `{owner}` capability");
    }
    match artifact.key {
        "wally.toml" => "when this project uses Wally".to_string(),
        "modules" => "when this project vendors packages as git submodules".to_string(),
        "rokit.toml" => "when anything pins a tool version".to_string(),
        _ => "derived from your answers".to_string(),
    }
}

/// The capability whose implementation writes this artifact, if any. The
/// edge lives only in `capabilities`, so this reads it rather than
/// duplicating it.
fn owning_capability(key: &str) -> Option<&'static str> {
    capabilities::CAPABILITIES
        .iter()
        .find(|c| {
            c.implementations
                .iter()
                .any(|i| i.artifacts.contains(&key))
        })
        .map(|c| c.key)
}

fn print_artifact(artifact: &Artifact) {
    heading(artifact.key);
    println!("category: {}", artifact.category.label());
    println!("written:  {}", cause_line(artifact));
    if !artifact.also_requires.is_empty() {
        let needs: Vec<String> = artifact.also_requires.iter().map(|r| r.describe()).collect();
        println!("needs:    {}", needs.join(", and "));
    }
    println!();
    println!("{}", artifact.description);

    if let Some(owner) = owning_capability(artifact.key) {
        println!("\nTo not have this file, don't choose `{owner}`.");
        println!("Run `rproj info {owner}` for what that capability does.");
    }
}

/// A capability: what it gets you, what provides it, and what it writes.
///
/// The page that makes "shows its work" true after the fact - someone who
/// enabled Linting three weeks ago and now wants to configure it needs to
/// find the word Selene somewhere.
fn print_capability(capability: &'static capabilities::Capability) {
    heading(capability.key);
    println!("provides: {}", capability.outcome);
    if !capability.requires.is_empty() {
        println!("needs:    {}", capability.requires.join(", "));
    }
    println!(
        "default:  {}",
        if capability.default_selected { "on" } else { "off" }
    );

    for implementation in capability.implementations {
        println!("\n  {} ({})", implementation.display, implementation.key);
        if !implementation.tools.is_empty() {
            println!("    pins:    {}", implementation.tools.join(", "));
        }
        if !implementation.packages.is_empty() {
            println!("    adds:    {}", implementation.packages.join(", "));
        }
        if !implementation.artifacts.is_empty() {
            println!("    writes:  {}", implementation.artifacts.join(", "));
        }
        // The tool's own page is where the commands and the gotchas live.
        if tool_catalog::find(implementation.key).is_some()
            || tool_usage::find(implementation.key).is_some()
        {
            println!("    more:    rproj info {}", implementation.key);
        }
    }

    if capability.needs_an_implementation_prompt() {
        println!("\n`rproj new` asks which of these to use, because there is a real choice.");
    }
}

/// The "how do I actually use this" half of an entry. Printed after the
/// identifying details, since someone running `rproj info tarmac` almost
/// always wants this rather than the version string.
fn print_usage(usage: &Usage) {
    println!("\n{}", usage.what);
    println!("\nWhen: {}", usage.when);
    if !usage.commands.is_empty() {
        println!("\nCommands:");
        let width = usage.commands.iter().map(|(c, _)| c.len()).max().unwrap_or(0);
        for (cmd, explanation) in usage.commands {
            println!("  {cmd:<width$}  {explanation}");
        }
    }
    if !usage.notes.is_empty() {
        println!("\nWorth knowing:");
        for note in usage.notes {
            println!("  - {note}");
        }
    }
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
            println!("    {:<18} {:<32} ({})", tool.key, tool.kind.provider(), tool.kind.label());
        }
    }

    println!("\nCAPABILITIES (what `rproj new` asks about)");
    for capability in capabilities::CAPABILITIES {
        println!(
            "    {:<12} {:<18} {}",
            capability.key,
            capability.default_implementation().display,
            if capability.default_selected { "on by default" } else { "off by default" }
        );
    }

    println!("\nGENERATED FILES (rproj info <key> for why a project gets one)");
    for artifact in artifacts::ARTIFACTS {
        // The same cause the detail page reports, so the two views cannot
        // disagree about why a project has a file.
        println!("    {:<24} {}", artifact.key, cause_line(artifact));
    }

    println!("\nCONFIGURABLE (rproj configure <key>)");
    for tool in tool_settings::CONFIGURABLE_TOOLS {
        println!("    {:<18} {}", tool.key, tool.display_name);
    }

    println!("\nPLACE TEMPLATE (applied to every new project's default.project.json)");
    print_place_template();

    let setups = SavedSetup::list();
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
    for spec in PLACE_TEMPLATE {
        let location = match spec.parent {
            Some(parent) => format!("{parent}.{}", spec.name),
            None => spec.name.to_string(),
        };
        println!("  {location} ({})", spec.class_name);
        for prop in spec.properties {
            println!("    {:<26} {}", prop.name, prop.value.display());
        }
    }
}

fn print_saved_setups() {
    for setup in SavedSetup::list() {
        println!("    {setup}");
    }
}

/// Keeps the listing to one line per entry - the full text is what
/// `rproj info <key>` is for.
fn first_sentence(text: &str) -> &str {
    match text.find(". ") {
        Some(i) => &text[..=i],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every section must have something in it. An empty list is a dead end
    /// that says nothing about why it is empty - which is why saved setups
    /// are only added when some exist.
    #[test]
    fn no_section_is_empty() {
        for section in sections() {
            if let Section::Entries { label, rows } = &section {
                assert!(!rows.is_empty(), "{label} has no entries");
            }
        }
    }

    /// Every row must resolve to a detail page. A row whose key `show_one`
    /// cannot find would print "No catalog entry named ..." for something
    /// the menu just offered.
    #[test]
    fn every_row_resolves_to_a_detail_page() {
        for section in sections() {
            if let Section::Entries { label, rows } = &section {
                for row in rows {
                    let found = wally_packages::find(&row.key).is_some()
                        || tool_catalog::find(&row.key).is_some()
                        || capabilities::find(&row.key).is_some()
                        || artifacts::find(&row.key).is_some()
                        || tool_usage::find(&row.key).is_some();
                    assert!(found, "{label}: `{}` has no detail page", row.key);
                }
            }
        }
    }

    /// The rendered line is what selection matches on, so it has to round-trip
    /// through `option_key` even after width truncation.
    #[test]
    fn every_row_survives_rendering_as_a_picker_option() {
        for section in sections() {
            if let Section::Entries { rows, .. } = &section {
                for row in rows {
                    let line = ui::option_line(&row.key, &row.description, &row.badge);
                    assert_eq!(ui::option_key(&line), row.key, "{line}");
                }
            }
        }
    }

    /// The back option is matched by exact string, so it must not collide
    /// with a rendered row - a package called that would be unselectable.
    #[test]
    fn the_back_option_cannot_collide_with_an_entry() {
        for section in sections() {
            if let Section::Entries { rows, .. } = &section {
                for row in rows {
                    assert_ne!(
                        ui::option_line(&row.key, &row.description, &row.badge),
                        "← back to the sections"
                    );
                }
            }
        }
    }

    /// Every artifact page answers *why does my project have this file*, and
    /// the answer names a cause the user can act on - a capability they
    /// chose, the strategy they picked, or "every project gets it". A file
    /// whose page said only "derived from your answers" would be the receipt
    /// problem all over again.
    #[test]
    fn every_artifact_names_the_thing_that_causes_it() {
        for artifact in artifacts::ARTIFACTS {
            let line = cause_line(artifact);
            assert!(
                line != "derived from your answers",
                "{} has no nameable cause",
                artifact.key
            );
            assert!(line.chars().next().is_some_and(|c| c.is_lowercase()), "{line}");
        }
    }

    /// The three shapes, each pinned to the entry that motivated it.
    #[test]
    fn the_cause_line_distinguishes_capability_from_strategy_from_always() {
        let line = |key: &str| cause_line(artifacts::find(key).expect(key));

        assert_eq!(line("src"), "always - every Rojo project has this");
        assert_eq!(line("selene.toml"), "when you choose the `lint` capability");
        assert_eq!(line("wally.toml"), "when this project uses Wally");
        assert!(line(".gitignore").starts_with("always -"), "{}", line(".gitignore"));
    }

    /// A capability page must name what actually provides it - the whole
    /// point of the layer is that the user still learns the tool's name.
    #[test]
    fn every_capability_resolves_to_a_detail_page_naming_its_tool() {
        for capability in capabilities::CAPABILITIES {
            let implementation = capability.default_implementation();
            assert!(!implementation.display.is_empty(), "{}", capability.key);
            for key in implementation.artifacts {
                assert_eq!(
                    owning_capability(key),
                    Some(capability.key),
                    "{key} should be owned by {}",
                    capability.key
                );
            }
        }
    }

    /// Section labels are matched by exact string to find the section again,
    /// so two sections may not share one.
    #[test]
    fn section_labels_are_unique() {
        let mut labels: Vec<&str> = sections().iter().map(|s| s.label()).collect();
        let before = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(before, labels.len(), "duplicate section label");
    }
}
