use std::path::Path;

use anyhow::Result;

use crate::steps::run_in;

/// Every scaffolded project should be a git repo - required outright for
/// the git-submodule package workflow, and just generally part of a
/// professional setup regardless of which package workflow is in use.
pub fn ensure_repo_init(project_dir: &Path) -> Result<()> {
    if project_dir.join(".git").exists() {
        println!("check: git repo already initialized");
        return Ok(());
    }
    run_in("git", &["init"], Some(project_dir))
}

/// Adds `repo_url` as a submodule under `Modules/<folder_name>`. Multiple
/// wally-catalog packages can share the same underlying repo (monorepos
/// like littensy/charm), so callers should dedupe by `repo_url` before
/// calling this - each repo only needs to be added once.
pub fn add_submodule(project_dir: &Path, repo_url: &str, folder_name: &str) -> Result<()> {
    let rel_path = format!("Modules/{folder_name}");
    if project_dir.join(&rel_path).exists() {
        println!("check: {rel_path} already added as a submodule");
        return Ok(());
    }
    run_in("git", &["submodule", "add", repo_url, &rel_path], Some(project_dir))
}
