use anyhow::Result;

use crate::catalog::presets;
use crate::catalog::tool_catalog;
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
    if let Some(preset) = presets::find(key) {
        println!("{} (preset)", preset.label);
        println!("{}", "-".repeat(preset.label.len() + 9));
        println!("packages: {}", preset.packages.join(", "));
        println!();
        println!("{}", preset.description);
        return Ok(());
    }
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
    println!("PRESETS");
    for preset in presets::PRESETS {
        println!("  {:<22} {} packages", preset.key, preset.packages.len());
    }

    println!("\nWALLY PACKAGES");
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

    println!("\nRun `rproj info <key>` for full details (description, maintenance status, docs) on any entry.");
    Ok(())
}
