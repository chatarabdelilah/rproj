use std::fs;
use std::path::Path;

use anyhow::Result;

const ENTRIES: &[&str] = &["packages/", "wally.lock"];

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
