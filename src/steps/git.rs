use std::path::Path;

use anyhow::Result;

use crate::steps::run_in;
use crate::ui;

/// Initialize version control for a scaffolded project.
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
