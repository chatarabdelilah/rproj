use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
        ProjectDirs::from("", "", "rproj")
            .context("could not determine a config directory for this platform")
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
///
/// `None` is a real answer, not the absence of one. Before it existed, a
/// project with no packages silently became a Wally project - and was then
/// offered a `wally.toml` for dependencies it did not have. A tutorial
/// project, or one that vendors by hand, genuinely has no dependency
/// manager, and saying so is what stops the scaffold pretending otherwise.
///
/// Adding the variant is deliberately *not* modelled as `Option<..>`: it is
/// a third case for the code that mounts package folders, and an `Option`
/// would only move the exhaustiveness check somewhere the compiler cannot
/// help.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageWorkflow {
    /// The default in the `Default` sense only - which strategy a *project*
    /// gets is always an explicit answer, never this.
    #[default]
    Wally,
    GitSubmodules,
    None,
}

impl PackageWorkflow {
    pub const ALL: &'static [Self] = &[Self::Wally, Self::GitSubmodules, Self::None];
}

/// Reading and writing `rproj.toml`, the per-project record.
///
/// **The file *is* the project graph** - the decisions the project was built
/// from, rather than the files that came out. That is what lets `rproj
/// upgrade` re-derive from intent, so a changed default reaches a project
/// made months ago instead of stranding it.
///
/// Free functions rather than an `impl`, because the graph type lives in
/// `graph` and owning its persistence here keeps `graph` free of filesystem
/// concerns.
pub mod project_file {
    use super::*;
    use crate::graph::ProjectGraph;

    pub fn path_in(project_dir: &Path) -> PathBuf {
        project_dir.join("rproj.toml")
    }

    pub fn load_from(project_dir: &Path) -> Result<Option<ProjectGraph>> {
        let path = path_in(project_dir);
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let graph: ProjectGraph =
            toml::from_str(&text).with_context(|| format!("failed to parse {}", path.display()))?;
        Ok(Some(graph))
    }

    pub fn save_to(graph: &ProjectGraph, project_dir: &Path) -> Result<()> {
        let path = path_in(project_dir);
        let text = toml::to_string_pretty(graph)?;
        fs::write(&path, text).with_context(|| format!("failed to write {}", path.display()))
    }
}

/// A saved project composition, reusable for the next project.
///
/// Deliberately *not* the old hardcoded presets: this is whatever a real
/// project ended up with, saved under a name you chose, so the set of
/// available setups is yours rather than a curated list that goes stale.
/// A saved setup **is a project graph**. It used to be packages plus a
/// workflow, so `--like` replayed half a composition and then asked three
/// more questions - "reuse my setup" that reuses some of it is a promise
/// half kept. Storing the graph means a saved setup replays straight to the
/// summary.
pub type SavedSetup = crate::graph::ProjectGraph;

/// Reading and writing `<config>/setups/<name>.toml`.
pub struct Setups;

impl Setups {
    fn dir() -> Result<PathBuf> {
        Ok(GlobalConfig::dirs()?.config_dir().join("setups"))
    }

    fn path_for(name: &str) -> Result<PathBuf> {
        Ok(Self::dir()?.join(format!("{name}.toml")))
    }

    pub fn save(setup: &SavedSetup, name: &str) -> Result<PathBuf> {
        let path = Self::path_for(name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, toml::to_string_pretty(setup)?)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(path)
    }

    pub fn load(name: &str) -> Result<Option<SavedSetup>> {
        let path = Self::path_for(name)?;
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        Ok(Some(toml::from_str(&text).with_context(|| {
            format!("failed to parse {}", path.display())
        })?))
    }

    /// Names of every saved setup, sorted. Missing directory is not an
    /// error - it just means none have been saved yet.
    ///
    /// The `is_file` filter is load-bearing: `read_dir` yields directories
    /// too, and without it a directory named `something.toml` is listed as
    /// a setup that then fails to load, because the path exists and so the
    /// `None` branch in `load` never fires.
    pub fn list() -> Vec<String> {
        let Ok(dir) = Self::dir() else {
            return Vec::new();
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter_map(|e| {
                let path = e.path();
                (path.extension()? == "toml")
                    .then(|| path.file_stem()?.to_str().map(str::to_owned))?
            })
            .collect();
        names.sort();
        names
    }
}

/// Persistence for the machine-wide Rojo project template.
pub mod project_template {
    use super::*;

    pub fn path() -> Result<PathBuf> {
        Ok(GlobalConfig::dirs()?
            .config_dir()
            .join("templates")
            .join("default.project.json"))
    }

    pub fn load() -> Result<Option<Value>> {
        load_from(&path()?)
    }

    pub fn read_text() -> Result<Option<String>> {
        read_text_from(&path()?)
    }

    pub fn save(template: &Value) -> Result<PathBuf> {
        save_to(template, &path()?)
    }

    pub fn reset() -> Result<bool> {
        reset_at(&path()?)
    }

    pub(crate) fn reset_at(path: &Path) -> Result<bool> {
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
        Ok(true)
    }

    pub(crate) fn load_from(path: &Path) -> Result<Option<Value>> {
        let Some(text) = read_text_from(path)? else {
            return Ok(None);
        };
        let value = serde_json::from_str(&text).with_context(|| {
            format!(
                "failed to parse {} - run `rproj configure project` to repair or reset it",
                path.display()
            )
        })?;
        Ok(Some(value))
    }

    fn read_text_from(path: &Path) -> Result<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }
        fs::read_to_string(path)
            .map(Some)
            .with_context(|| format!("failed to read {}", path.display()))
    }

    pub(crate) fn save_to(template: &Value, path: &Path) -> Result<PathBuf> {
        let parent = path
            .parent()
            .context("project template path has no parent")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        let text = format!("{}\n", serde_json::to_string_pretty(template)?);
        let mut staged = tempfile::NamedTempFile::new_in(parent)
            .with_context(|| format!("failed to stage {}", path.display()))?;
        staged
            .write_all(text.as_bytes())
            .with_context(|| format!("failed to stage {}", path.display()))?;
        staged
            .as_file()
            .sync_all()
            .with_context(|| format!("failed to sync {}", path.display()))?;
        staged
            .persist(path)
            .with_context(|| format!("failed to replace {}", path.display()))?;
        Ok(path.to_path_buf())
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
        let dir = std::env::temp_dir().join(format!("rproj-list-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("trap.toml")).expect("create the trap");
        fs::write(
            dir.join("real.toml"),
            "packages = []\npackage_workflow = \"wally\"\n",
        )
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
                (path.extension()? == "toml")
                    .then(|| path.file_stem()?.to_str().map(str::to_owned))?
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
        let text = toml::to_string_pretty(&crate::graph::ProjectGraph {
            mode: "guided".into(),
            package_workflow: PackageWorkflow::GitSubmodules,
            ..Default::default()
        })
        .expect("serialise");
        assert!(
            text.contains("package_workflow = \"git-submodules\""),
            "{text}"
        );
        assert!(!text.contains("GitSubmodules"), "{text}");
    }

    #[test]
    fn project_template_round_trips_in_an_injected_location() {
        let dir =
            std::env::temp_dir().join(format!("rproj-template-config-test-{}", std::process::id()));
        let path = dir.join("templates").join("default.project.json");
        let _ = fs::remove_dir_all(&dir);
        let template = serde_json::json!({"name": "ProjectName", "tree": {}});

        project_template::save_to(&template, &path).expect("save template");
        assert_eq!(
            project_template::load_from(&path).expect("load template"),
            Some(template)
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_project_template_names_the_repair_command() {
        let dir = std::env::temp_dir().join(format!(
            "rproj-template-corrupt-test-{}",
            std::process::id()
        ));
        let path = dir.join("default.project.json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create fixture");
        fs::write(&path, "not json").expect("write fixture");

        let error = project_template::load_from(&path).unwrap_err().to_string();
        assert!(error.contains("rproj configure project"), "{error}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resetting_removes_only_the_custom_template() {
        let dir =
            std::env::temp_dir().join(format!("rproj-template-reset-test-{}", std::process::id()));
        let path = dir.join("templates").join("default.project.json");
        let sibling = dir.join("config.toml");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(path.parent().unwrap()).expect("create fixture");
        fs::write(&path, "{}").expect("write template");
        fs::write(&sibling, "last_checked = 'now'").expect("write sibling");

        assert!(project_template::reset_at(&path).expect("reset template"));
        assert!(!path.exists());
        assert!(sibling.exists(), "reset removed unrelated configuration");
        assert!(!project_template::reset_at(&path).expect("repeat reset"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn saving_replaces_an_existing_template() {
        let dir = std::env::temp_dir().join(format!(
            "rproj-template-replace-test-{}",
            std::process::id()
        ));
        let path = dir.join("default.project.json");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create fixture");
        fs::write(&path, "previous contents").expect("write previous template");

        let template = serde_json::json!({ "name": "ProjectName", "tree": {} });
        project_template::save_to(&template, &path).expect("replace template");

        assert_eq!(
            fs::read_to_string(&path).expect("read replacement"),
            "{\n  \"name\": \"ProjectName\",\n  \"tree\": {}\n}\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
