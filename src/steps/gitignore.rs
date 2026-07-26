use std::fs;
use std::path::Path;

use anyhow::Result;

// wally.lock is deliberately NOT here - like Cargo.lock, it should be
// committed so everyone building the project resolves the same package
// versions. Only the regenerable Packages folder and build artifacts go here.
//
// modules/ is likewise NOT ignored: under the git-submodule workflow it
// holds the generated link files and the submodules project file, which
// are project source and must be committed (the submodule *contents*
// are tracked by git as submodule pointers, not as ignorable files).
const ENTRIES: &[&str] = &[
    "packages/",
    "sourcemap.json",
    // Fetched fresh by .lute/check.luau each run and deleted afterwards;
    // listed so an interrupted run can't leave it staged.
    "roblox.d.luau",
    "*.blend1",
    "*.blend2",
    "Thumbs.db",
    ".DS_Store",
];

pub fn ensure_entries(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".gitignore");
    let content = if path.exists() { fs::read_to_string(&path)? } else { String::new() };

    let existing: std::collections::HashSet<&str> =
        content.lines().map(str::trim).collect();
    let missing: Vec<&str> = ENTRIES
        .iter()
        .copied()
        .filter(|e| !existing.contains(e))
        .collect();

    if missing.is_empty() {
        println!("check: .gitignore already configured");
        return Ok(());
    }

    let mut updated = content.clone();
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    for entry in &missing {
        updated.push_str(entry);
        updated.push('\n');
    }
    fs::write(&path, updated)?;
    println!("updated .gitignore");
    Ok(())
}
