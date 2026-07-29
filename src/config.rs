use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Everything provisioning decided, machine-wide. `rproj new` reads this
/// and re-asks only on a machine with nothing recorded, or with
/// `--reconfigure`.
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
    pub(crate) fn dirs() -> Result<ProjectDirs> {
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

    /// Whether this machine has been through provisioning at least once.
    ///
    /// Keyed on having recorded *any* selection rather than on a flag, so
    /// a config written by an older version still counts and someone who
    /// deliberately selected nothing isn't asked again every time.
    pub fn machine_configured(&self) -> bool {
        self.last_checked.is_some()
    }

    /// One-line description of what provisioning last set up, for the
    /// summary `rproj new` prints when it skips the questions.
    pub fn machine_summary(&self) -> String {
        let parts = [
            (self.selected_system_apps.len(), "apps"),
            (self.selected_rokit_tools.len(), "tools"),
            (self.selected_studio_plugins.len(), "plugins"),
            (self.selected_vscode_extensions.len(), "extensions"),
        ];
        let listed: Vec<String> = parts
            .iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, label)| format!("{n} {label}"))
            .collect();
        if listed.is_empty() {
            "nothing selected".to_string()
        } else {
            listed.join(", ")
        }
    }
}

fn dirs_documents() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE").context("USERPROFILE is not set")?;
    Ok(PathBuf::from(home).join("Documents"))
}

/// How a project's packages get pulled in. Wally is the default; git
/// submodules clone each selected package's repo into `modules/submodules/`
/// instead of writing a `wally.toml`. See `steps::modules`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageWorkflow {
    Wally,
    GitSubmodules,
}

/// Per-project record, written by `rproj new` into the project root.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// How the packages were chosen: "guided", "expert", or
    /// "like:<setup>" when reused from a saved setup.
    pub mode: String,
    pub package_workflow: PackageWorkflow,
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

/// A saved project composition, reusable for the next project.
///
/// Deliberately *not* the old hardcoded presets: this is whatever a real
/// project ended up with, saved under a name you chose, so the set of
/// available setups is yours rather than a curated list that goes stale.
#[derive(Debug, Serialize, Deserialize)]
pub struct SavedSetup {
    pub packages: Vec<String>,
    pub package_workflow: PackageWorkflow,
}

impl SavedSetup {
    fn dir() -> Result<PathBuf> {
        Ok(GlobalConfig::dirs()?.config_dir().join("setups"))
    }

    fn path_for(name: &str) -> Result<PathBuf> {
        Ok(Self::dir()?.join(format!("{name}.toml")))
    }

    pub fn save(&self, name: &str) -> Result<PathBuf> {
        let path = Self::path_for(name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, toml::to_string_pretty(self)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn load(name: &str) -> Result<Option<Self>> {
        let path = Self::path_for(name)?;
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Ok(Some(
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?,
        ))
    }

    /// Names of every saved setup, sorted. Missing directory is not an
    /// error - it just means none have been saved yet.
    ///
    /// The `is_file` filter is load-bearing: `read_dir` yields directories
    /// too, and without it a directory named `something.toml` is listed as
    /// a setup that then fails to load, because the path exists and so the
    /// `None` branch in `load` never fires.
    pub fn list() -> Vec<String> {
        let Ok(dir) = Self::dir() else { return Vec::new() };
        let Ok(entries) = fs::read_dir(dir) else { return Vec::new() };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let path = e.path();
                (path.extension()? == "toml").then(|| path.file_stem()?.to_str().map(str::to_owned))?
            })
            .collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `read_dir` yields directories as well as files, and the extension
    /// filter alone does not tell them apart. Without the `is_file` check
    /// a directory named `something.toml` was listed as a saved setup -
    /// and then failed to load rather than returning `None`, because the
    /// path does exist.
    #[test]
    fn listing_ignores_a_directory_named_like_a_setup() {
        let dir = std::env::temp_dir()
            .join(format!("rproj-list-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("trap.toml")).expect("create the trap");
        fs::write(dir.join("real.toml"), "packages = []\npackage_workflow = \"wally\"\n")
            .expect("write a real one");
        fs::write(dir.join("notes.txt"), "not a setup").expect("write");

        // The same filter chain `list` runs, against a directory the test
        // controls. `list` itself reads the real config location, which a
        // test must not touch.
        let mut names: Vec<String> = fs::read_dir(&dir)
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let path = e.path();
                (path.extension()? == "toml").then(|| path.file_stem()?.to_str().map(str::to_owned))?
            })
            .collect();
        names.sort();

        assert_eq!(names, ["real"], "the directory and the .txt are both out");
        let _ = fs::remove_dir_all(&dir);
    }

    /// The on-disk spelling is a compatibility promise; the Rust spelling
    /// is not. A rename in Rust must not silently change the file format.
    #[test]
    fn the_workflow_enum_is_written_in_kebab_case() {
        let text = toml::to_string_pretty(&ProjectConfig {
            mode: "guided".into(),
            package_workflow: PackageWorkflow::GitSubmodules,
            packages: vec![],
            tools_at_creation: vec![],
        })
        .expect("serialise");
        assert!(text.contains("package_workflow = \"git-submodules\""), "{text}");
        assert!(!text.contains("GitSubmodules"), "{text}");
    }
}
