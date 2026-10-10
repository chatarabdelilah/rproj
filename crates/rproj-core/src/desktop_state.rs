//! Per-user desktop preferences, kept separate from project and machine setup files.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesktopState {
    pub schema_version: u32,
    pub appearance: Appearance,
    pub recent_projects: Vec<PathBuf>,
    pub desktop_executable: Option<PathBuf>,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            appearance: Appearance::System,
            recent_projects: Vec::new(),
            desktop_executable: None,
        }
    }
}

impl DesktopState {
    pub fn path() -> Result<PathBuf> {
        Ok(directories::ProjectDirs::from("", "", "rproj")
            .context("Could not locate the rproj configuration directory")?
            .config_dir()
            .join("desktop.json"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(error).context("Could not read desktop preferences"),
        };
        let mut text = String::new();
        file.take(1024 * 1024 + 1).read_to_string(&mut text)?;
        ensure!(
            text.len() <= 1024 * 1024,
            "Desktop preferences exceed the size limit"
        );
        let mut state: Self = serde_json::from_str(&text)
            .context("Invalid desktop preferences; existing file preserved")?;
        ensure!(
            state.schema_version == 1,
            "Unsupported desktop preferences version; existing file preserved"
        );
        state.recent_projects.truncate(12);
        Ok(state)
    }

    pub fn remember(&mut self, project: &Path) -> Result<()> {
        let project = project
            .canonicalize()
            .context("Project folder is unavailable")?;
        ensure!(project.is_dir(), "Project path is not a folder");
        self.recent_projects.retain(|path| path != &project);
        self.recent_projects.insert(0, project);
        self.recent_projects.truncate(12);
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "Unsupported desktop preferences version"
        );
        let parent = path.parent().context("Preferences path has no parent")?;
        fs::create_dir_all(parent)?;
        let mut pending = tempfile::NamedTempFile::new_in(parent)?;
        pending.write_all(&serde_json::to_vec_pretty(self)?)?;
        pending.as_file().sync_all()?;
        pending
            .persist(path)
            .map_err(|error| error.error)
            .context("Could not save desktop preferences")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_are_separate_and_recent_projects_are_deduplicated() {
        let root = tempfile::tempdir().unwrap();
        let machine = root.path().join("config.toml");
        fs::write(&machine, "# machine setup stays unchanged").unwrap();
        let path = root.path().join("desktop.json");
        let mut state = DesktopState::load(&path).unwrap();
        assert!(!path.exists());
        state.remember(root.path()).unwrap();
        state.remember(root.path()).unwrap();
        state.appearance = Appearance::Dark;
        state.save(&path).unwrap();
        let loaded = DesktopState::load(&path).unwrap();
        assert_eq!(loaded.recent_projects.len(), 1);
        assert_eq!(loaded.appearance, Appearance::Dark);
        assert_eq!(
            fs::read_to_string(machine).unwrap(),
            "# machine setup stays unchanged"
        );
    }
    #[test]
    fn future_preferences_are_rejected_without_writes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("desktop.json");
        let text = r#"{"schema_version":2,"appearance":"system","recent_projects":[],"desktop_executable":null}"#;
        fs::write(&path, text).unwrap();
        assert!(DesktopState::load(&path).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}
