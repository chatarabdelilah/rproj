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

fn show_one(key: &str) -> Result<()> {
    if let Some(preset) = presets::find(key) {
        println!("{}  (preset)", preset.label);
        println!("  {}", preset.description);
        println!("  packages: {}", preset.packages.join(", "));
        return Ok(());
    }
    if let Some(pkg) = wally_packages::find(key) {
        println!("{}  [{}]", pkg.key, pkg.category.label());
        println!("  {}", pkg.description);
        println!("  status: {}", pkg.maintenance.badge());
        println!("  source: {}", pkg.source);
        println!("  docs:   {}", pkg.docs_url);
        return Ok(());
    }
    if let Some(tool) = tool_catalog::find(key) {
        println!("{}", tool.key);
        println!("  {}", tool.description);
        println!("  status: {}", tool.maintenance.badge());
        println!("  docs:   {}", tool.docs_url);
        return Ok(());
    }
    println!("No catalog entry named `{key}`. Run `rproj info` with no argument to list everything.");
    Ok(())
}

fn list_all() -> Result<()> {
    println!("Presets:\n");
    for preset in presets::PRESETS {
        println!("    {:<20} {}", preset.key, preset.description);
    }

    println!("\nWally packages:\n");
    for category in wally_packages::Category::ALL {
        println!("  {}:", category.label());
        for pkg in wally_packages::in_category(category) {
            println!("    {:<14} {} ({})", pkg.key, pkg.description, pkg.maintenance.badge());
        }
    }

    println!("\nTools (see `rproj setup`):\n");
    for tool in tool_catalog::all_setup_entries() {
        println!("    {:<16} {} ({})", tool.key, tool.description, tool.maintenance.badge());
    }

    println!("\nRun `rproj info <key>` for details on any entry.");
    Ok(())
}
