use std::path::Path;

use anyhow::Result;

use crate::steps::run_in;
use crate::ui;

/// Every scaffolded project should be a git repo - required outright for
/// the git-submodule package workflow, and just generally part of a
/// professional setup regardless of which package workflow is in use.
pub fn ensure_repo_init(project_dir: &Path) -> Result<()> {
    if project_dir.join(".git").exists() {
        ui::ok("git repo already initialized");
        return Ok(());
    }
    run_in("git", &["init"], Some(project_dir))
}

/// Adds `repo_url` as a submodule under `modules/submodules/<dir>` -
/// nested under an extra `submodules/` level rather than directly under
/// `modules/`, matching the structure used by littensy/fishing-minigame (a
/// real project consuming several of this catalog's packages this same
/// way). Multiple wally-catalog packages can share the same underlying repo
/// (monorepos like littensy/charm), so callers should dedupe by `dir`
/// before calling this - each repo only needs to be cloned once.
pub fn add_submodule(project_dir: &Path, repo_url: &str, dir: &str) -> Result<()> {
    let rel_path = format!("modules/submodules/{dir}");
    if project_dir.join(&rel_path).exists() {
        ui::ok(&format!("{rel_path} already present"));
        return Ok(());
    }
    // Cloning several repos takes long enough that silence reads as a
    // hang, and git's own progress output is captured, so say what's
    // happening before starting rather than only after.
    ui::ok(&format!("cloning {dir}"));
    run_in("git", &["submodule", "add", repo_url, &rel_path], Some(project_dir))
}
