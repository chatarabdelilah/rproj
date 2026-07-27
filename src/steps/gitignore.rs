use std::fs;
use std::path::Path;

use anyhow::Result;
use crate::ui;

// wally.lock is deliberately NOT here - like Cargo.lock, it should be
// committed so everyone building the project resolves the same package
// versions. Only the regenerable Packages folder and build artifacts go here.
//
// modules/ is likewise NOT ignored: under the git-submodule workflow it
// holds the generated link files and the submodules project file, which
// are project source and must be committed (the submodule *contents*
// are tracked by git as submodule pointers, not as ignorable files).
const ENTRIES: &[&str] = &[
    // Capitalised because that's the folder wally actually creates. Git on
    // a case-sensitive filesystem wouldn't ignore it under any other
    // spelling, so a Linux contributor would be committing every vendored
    // package.
    "Packages/",
    // Wally's second output folder, for `[server-dependencies]`. Generated
    // the same way and just as regenerable, so it's ignored the same way.
    "ServerPackages/",
    "sourcemap.json",
    // Local editor state. Note the tradeoff: rproj writes luau-lsp's
    // ignoreGlobs here for submodule projects, so a teammate cloning the
    // repo won't inherit them and will see the vendored modules/ folder
    // linted until they run rproj themselves.
    ".vscode/",
    // Records how *this* checkout was scaffolded (mode, packages, the tools
    // present at creation). Useful locally, but it's a snapshot of one
    // machine's choices rather than something the project needs to agree
    // on - wally.toml, rokit.toml and default.project.json are what
    // actually define the project for everyone.
    "rproj.toml",
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
        ui::ok(".gitignore already configured");
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
    ui::ok("updated .gitignore");
    Ok(())
}
