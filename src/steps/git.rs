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
    git_init_result(run_in("git", &["init"], Some(project_dir)), project_dir)
}

fn git_init_result(result: Result<()>, project_dir: &Path) -> Result<()> {
    result.map_err(|error| {
        // A vanished working directory also produces NotFound; preserve that error.
        if project_dir.is_dir()
            && error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|cause| cause.kind() == std::io::ErrorKind::NotFound)
        {
            error.context(
                "Git was not found on PATH; it is required to initialize this project. \
                 Install Git from https://git-scm.com/install/, ensure it is on PATH, \
                 open a new terminal and check `git --version`. Inspect the directory \
                 left by this attempt, then retry project creation in a new destination; \
                 rproj will not overwrite the existing directory",
            )
        } else {
            error
        }
    })
}

/// Fetches the contents of every submodule the project declares.
///
/// A plain `git clone` records each submodule's commit and leaves its
/// directory empty. `modules/submodules/default.project.json` maps
/// straight into those directories, and rojo refuses to build a sourcemap
/// for a `$path` it can't turn into an instance - so `rproj watch` on a
/// fresh clone of a submodule project printed "Watching for changes" and
/// then died on `File $path: ./charm/packages/charm/src`, having watched
/// nothing. Reproduced by cloning a scaffolded project without
/// `--recurse-submodules`, which is what `git clone <url>` does by default.
///
/// This is the submodule half of what `wally::sync` does for Wally
/// projects, whose `Packages/` is absent from a fresh clone for the same
/// reason (gitignored). Wally projects recovered on their own; submodule
/// ones did not, and the asymmetry was invisible on any machine where the
/// project had been scaffolded rather than cloned.
///
/// `add_submodule` can't stand in for this: its "already present" check is
/// a directory-exists check, and after a clone the directory exists and is
/// empty.
pub fn sync_submodules(project_dir: &Path) -> Result<()> {
    if !project_dir.join(".gitmodules").exists() {
        return Ok(());
    }
    run_in(
        "git",
        &["submodule", "update", "--init", "--recursive"],
        Some(project_dir),
    )?;
    ui::ok("submodules synced");
    Ok(())
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
    run_in(
        "git",
        &["submodule", "add", repo_url, &rel_path],
        Some(project_dir),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_init_preserves_permission_and_command_failures() {
        let root = tempfile::tempdir().unwrap();
        let denied = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            .context("failed to spawn `git`");
        let command = anyhow::anyhow!("`git init` failed: permission denied writing .git/config");
        for original in [denied, command] {
            let before = format!("{original:#}");
            let after = git_init_result(Err(original), root.path()).unwrap_err();
            assert_eq!(format!("{after:#}"), before);
        }
    }

    #[test]
    fn absent_project_directory_does_not_report_missing_git() {
        let root = tempfile::tempdir().unwrap();
        let absent = root.path().join("absent-project");
        let error = ensure_repo_init(&absent).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("failed to spawn `git`"), "{message}");
        assert!(!message.contains("Git was not found on PATH"), "{message}");
        assert!(!absent.exists());
    }
}
