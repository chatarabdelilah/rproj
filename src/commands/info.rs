use anyhow::Result;

use crate::catalog::place_template::PLACE_TEMPLATE;
use crate::catalog::tool_catalog;
use crate::catalog::tool_settings;
use crate::catalog::wally_packages;

pub fn run(key: Option<&str>) -> Result<()> {
    match key {
        Some(key) => show_one(key),
        None => list_all(),
    }
}

/// Full detail on a single catalog entry: description, maintenance status,
/// source/provider, docs. Compare `list_all`, which stays terse on purpose.
fn show_one(key: &str) -> Result<()> {
    if let Some(pkg) = wally_packages::find(key) {
        println!("{}", pkg.key);
        println!("{}", "-".repeat(pkg.key.len()));
        println!("category: {}", pkg.category.label());
        println!("status:   {}", pkg.maintenance.badge());
        println!("source:   {}", pkg.source);
        println!("docs:     {}", pkg.docs_url);
        println!();
        println!("{}", pkg.description);
        return Ok(());
    }
    if let Some(tool) = tool_catalog::find(key) {
        println!("{}", tool.key);
        println!("{}", "-".repeat(tool.key.len()));
        println!("family:   {}", tool.family);
        println!("kind:     {}", tool.kind.label());
        println!("provider: {}", tool.kind.provider());
        println!("status:   {}", tool.maintenance.badge());
        println!("docs:     {}", tool.docs_url);
        println!();
        println!("{}", tool.description);
        if tool_settings::find(key).is_some() {
            println!("\nConfigurable: run `rproj configure {key}` to walk through its settings.");
        }
        return Ok(());
    }
    println!("No catalog entry named `{key}`. Run `rproj info` with no argument to list everything.");
    Ok(())
}

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

    println!("\nCONFIGURABLE (rproj configure <key>)");
    for tool in tool_settings::CONFIGURABLE_TOOLS {
        println!("    {:<18} {}", tool.key, tool.display_name);
    }

    println!("\nPLACE TEMPLATE (applied to every new project's default.project.json)");
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

    println!("\nRun `rproj info <key>` for full details (description, maintenance status, docs) on any entry.");
    Ok(())
}
