use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Everything `rproj setup` decided, machine-wide. `rproj new` reads this
/// and never re-asks about tools - see the plan's "setup does everything,
/// new just uses what's there" decision.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub roblox_projects_root: Option<PathBuf>,
    #[serde(default)]
    pub selected_system_apps: Vec<String>,
    #[serde(default)]
    pub selected_rokit_tools: Vec<String>,
    #[serde(default)]
    pub selected_studio_plugins: Vec<String>,
    #[serde(default)]
    pub selected_vscode_extensions: Vec<String>,
    #[serde(default)]
    pub last_checked: Option<String>,
}

impl GlobalConfig {
    fn dirs() -> Result<ProjectDirs> {
        ProjectDirs::from("", "", "rproj").context("could not determine a config directory for this platform")
    }

    fn path() -> Result<PathBuf> {
        Ok(Self::dirs()?.config_dir().join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self)?;
        fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn projects_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.roblox_projects_root {
            return Ok(root.clone());
        }
        let documents = dirs_documents()?;
        Ok(documents.join("RobloxProjects"))
    }

    pub fn blender_enabled(&self) -> bool {
        self.selected_system_apps.iter().any(|k| k == "blender")
    }
}

fn dirs_documents() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE").context("USERPROFILE is not set")?;
    Ok(PathBuf::from(home).join("Documents"))
}

/// Per-project record, written by `rproj new` into the project root.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub mode: String,
    pub preset_key: Option<String>,
    pub packages: Vec<String>,
    /// Snapshot of which machine-wide tool keys were active when this
    /// project was created (for future "what did I pick before" reference).
    pub tools_at_creation: Vec<String>,
}

impl ProjectConfig {
    pub fn path_in(project_dir: &Path) -> PathBuf {
        project_dir.join("rproj.toml")
    }

    pub fn load_from(project_dir: &Path) -> Result<Option<Self>> {
        let path = Self::path_in(project_dir);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Ok(Some(
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?,
        ))
    }

    pub fn save_to(&self, project_dir: &Path) -> Result<()> {
        let path = Self::path_in(project_dir);
        let text = toml::to_string_pretty(self)?;
        fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))
    }
}
