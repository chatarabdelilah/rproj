use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) mod setups;

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
        self.save_at(&path)
    }

    fn save_at(&self, path: &Path) -> Result<()> {
        let text = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let parent = path
            .parent()
            .context("machine configuration has no parent directory")?;
        let mut pending = tempfile::NamedTempFile::new_in(parent)?;
        pending.write_all(text.as_bytes())?;
        pending.as_file().sync_all()?;
        pending
            .persist(path)
            .map_err(|error| error.error)
            .with_context(|| format!("failed to replace {}", path.display()))?;
        Ok(())
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

/// How a project gets dependencies: Wally, or no dependency manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageWorkflow {
    /// The default in the `Default` sense only - which strategy a *project*
    /// gets is always an explicit answer, never this.
    #[default]
    Wally,
    None,
}

impl PackageWorkflow {
    pub const ALL: &'static [Self] = &[Self::Wally, Self::None];
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

fn save_new_setup_at(setup: &SavedSetup, path: &Path) -> Result<()> {
    use std::io::Write;
    let parent = path.parent().context("setup has no parent directory")?;
    fs::create_dir_all(parent)?;
    let text = toml::to_string_pretty(setup)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    temporary.as_file().sync_all()?;
    // The destination may have been created while scaffolding was running.
    temporary.persist_noclobber(path).with_context(|| {
        format!(
            "could not save new setup {}; existing setups are never replaced",
            path.display()
        )
    })?;
    Ok(())
}

/// Reading and writing `<config>/setups/<name>.toml`.
pub struct Setups;

impl Setups {
    pub(crate) fn dir() -> Result<PathBuf> {
        Ok(GlobalConfig::dirs()?.config_dir().join("setups"))
    }

    fn path_for(name: &str) -> Result<PathBuf> {
        setups::setup_path(&Self::dir()?, name)
    }

    pub fn save(setup: &SavedSetup, name: &str) -> Result<PathBuf> {
        let path = Self::path_for(name)?;
        setups::replace(&path, toml::to_string_pretty(setup)?.as_bytes())?;
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

    pub fn save_new(setup: &SavedSetup, name: &str) -> Result<PathBuf> {
        let path = Self::path_for(name)?;
        save_new_setup_at(setup, &path)?;
        Ok(path)
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
    use anyhow::ensure;

    pub fn path() -> Result<PathBuf> {
        Ok(GlobalConfig::dirs()?
            .config_dir()
            .join("templates")
            .join("default.project.json"))
    }

    pub fn load() -> Result<Option<Value>> {
        load_from(&path()?)
    }

    pub(crate) struct EditSession {
        path: PathBuf,
        baseline: Option<String>,
    }

    impl EditSession {
        pub fn open() -> Result<Self> {
            Self::open_at(&path()?)
        }

        pub fn open_at(path: &Path) -> Result<Self> {
            Ok(Self {
                path: path.to_owned(),
                baseline: read_text_from(path)?,
            })
        }

        pub fn text(&self) -> Option<&str> {
            self.baseline.as_deref()
        }

        fn ensure_current(&self) -> Result<()> {
            ensure!(
                read_text_from(&self.path)? == self.baseline,
                "The template changed outside this editor: {}. Your draft is retained. Reopen Template Explorer to load the current file before saving or resetting.",
                self.path.display()
            );
            Ok(())
        }

        pub fn save(
            &mut self,
            template: &Value,
            validate: impl FnOnce(&Value) -> Result<()>,
        ) -> Result<PathBuf> {
            self.ensure_current()?;
            validate(template)?;
            self.ensure_current()?;
            let text = format!("{}\n", serde_json::to_string_pretty(template)?);
            let staged =
                crate::file_replace::stage(&self.path, text.as_bytes(), self.baseline.is_some())
                    .with_context(|| format!("failed to stage {}", self.path.display()))?;
            // Validation and staging may take time; refuse newly changed input before replacement.
            self.ensure_current()?;
            staged
                .persist(&self.path)
                .map_err(|error| error.error)
                .with_context(|| format!("failed to replace {}", self.path.display()))?;
            self.baseline = Some(text);
            Ok(self.path.clone())
        }

        pub fn reset(&mut self) -> Result<bool> {
            self.ensure_current()?;
            if self.baseline.is_none() {
                return Ok(false);
            }
            fs::remove_file(&self.path)
                .with_context(|| format!("failed to remove {}", self.path.display()))?;
            self.baseline = None;
            Ok(true)
        }
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
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn external_changes_creation_and_deletion_refuse_save_and_reset() {
            for scenario in ["changed", "created", "deleted"] {
                let root = tempfile::tempdir().unwrap();
                let path = root.path().join("template.json");
                if scenario != "created" {
                    fs::write(&path, "original bytes\n").unwrap();
                }
                let mut session = EditSession::open_at(&path).unwrap();
                let baseline = session.baseline.clone();
                if scenario == "deleted" {
                    fs::remove_file(&path).unwrap();
                } else {
                    fs::write(&path, "external bytes\n").unwrap();
                }
                let external = read_text_from(&path).unwrap();
                let value = serde_json::json!({"draft": true});
                let error = session
                    .save(&value, |_| panic!("conflict must precede validation"))
                    .unwrap_err();
                assert!(
                    error.to_string().contains("changed outside this editor"),
                    "{scenario}: {error}"
                );
                assert!(
                    session
                        .reset()
                        .unwrap_err()
                        .to_string()
                        .contains("changed outside this editor")
                );
                assert_eq!(session.baseline, baseline);
                assert_eq!(read_text_from(&path).unwrap(), external);
                assert_eq!(
                    fs::read_dir(root.path()).unwrap().count(),
                    usize::from(external.is_some())
                );
            }
        }

        #[test]
        fn validation_time_changes_are_refused_without_write_artifacts() {
            for scenario in ["changed", "created", "deleted"] {
                let root = tempfile::tempdir().unwrap();
                let path = root.path().join("template.json");
                if scenario != "created" {
                    fs::write(&path, "original\n").unwrap();
                }
                let mut session = EditSession::open_at(&path).unwrap();
                let baseline = session.baseline.clone();
                let error = session
                    .save(&serde_json::json!({"draft": true}), |_| {
                        if scenario == "deleted" {
                            fs::remove_file(&path)?;
                        } else {
                            fs::write(&path, "external during validation\n")?;
                        }
                        Ok(())
                    })
                    .unwrap_err();
                assert!(
                    error.to_string().contains("changed outside this editor"),
                    "{scenario}: {error}"
                );
                let external = read_text_from(&path).unwrap();
                assert_eq!(
                    external.as_deref(),
                    (scenario != "deleted").then_some("external during validation\n")
                );
                assert_eq!(session.baseline, baseline);
                assert_eq!(
                    fs::read_dir(root.path()).unwrap().count(),
                    usize::from(external.is_some())
                );
            }
        }

        #[test]
        fn successful_saves_and_reset_refresh_the_session_baseline() {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("templates/template.json");
            let mut session = EditSession::open_at(&path).unwrap();
            assert_eq!(session.text(), None);
            for port in [40000, 40001] {
                let value = serde_json::json!({"servePort": port, "unknown": "preserve café"});
                session.save(&value, |_| Ok(())).unwrap();
                assert_eq!(session.text().unwrap(), fs::read_to_string(&path).unwrap());
                assert_eq!(load_from(&path).unwrap(), Some(value));
            }
            assert!(session.reset().unwrap());
            assert_eq!(session.text(), None);
            assert!(!session.reset().unwrap());
            session
                .save(&serde_json::json!({"afterReset": true}), |_| Ok(()))
                .unwrap();
            assert_eq!(
                load_from(&path).unwrap(),
                Some(serde_json::json!({"afterReset": true}))
            );
            assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        }

        #[test]
        fn rejected_validation_keeps_original_and_allows_retry() {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("template.json");
            fs::write(&path, "malformed input to repair\n").unwrap();
            let mut session = EditSession::open_at(&path).unwrap();
            let value = serde_json::json!({"repaired": true});
            session
                .save(&value, |_| anyhow::bail!("refused by validator"))
                .unwrap_err();
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "malformed input to repair\n"
            );
            assert_eq!(session.text(), Some("malformed input to repair\n"));
            assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
            session.save(&value, |_| Ok(())).unwrap();
            assert_eq!(load_from(&path).unwrap(), Some(value));
        }

        #[test]
        fn unreadable_template_is_not_treated_as_absent() {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("template.json");
            fs::create_dir(&path).unwrap();
            fs::write(path.join("owned"), "preserve").unwrap();
            assert!(
                EditSession::open_at(&path)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains("failed to read")
            );
            assert_eq!(fs::read_to_string(path.join("owned")).unwrap(), "preserve");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn machine_configuration_save_replaces_complete_document() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config.toml");
        std::fs::write(&path, "last_checked = 'old'\n").unwrap();
        let config = super::GlobalConfig {
            last_checked: Some("new".into()),
            ..Default::default()
        };
        config.save_at(&path).unwrap();
        let read: super::GlobalConfig =
            toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read.last_checked.as_deref(), Some("new"));
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn machine_configuration_failed_replacement_preserves_source() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("config.toml");
        let original = "last_checked = 'old'\n";
        std::fs::write(&path, original).unwrap();
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(super::GlobalConfig::default().save_at(&path).is_err());
        drop(held);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
    use super::*;

    #[test]
    fn new_setup_save_is_atomic_and_never_replaces_an_existing_file() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("setups/example.toml");
        let graph = SavedSetup::default();
        save_new_setup_at(&graph, &path).unwrap();
        let before = fs::read(&path).unwrap();
        assert!(save_new_setup_at(&graph, &path).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

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

    /// Supported workflow values round-trip through project records.
    #[test]
    fn the_workflow_enum_is_written_in_kebab_case() {
        for (workflow, spelling) in [
            (PackageWorkflow::Wally, "wally"),
            (PackageWorkflow::None, "none"),
        ] {
            let graph = crate::graph::ProjectGraph {
                package_workflow: workflow,
                ..Default::default()
            };
            let text = toml::to_string_pretty(&graph).expect("serialise");
            assert!(
                text.contains(&format!("package_workflow = \"{spelling}\"")),
                "{text}"
            );
            let decoded: crate::graph::ProjectGraph = toml::from_str(&text).expect("deserialise");
            assert_eq!(decoded.package_workflow, workflow);
        }
    }

    #[test]
    fn retired_workflow_is_rejected_without_defaulting_to_wally() {
        let error =
            toml::from_str::<crate::graph::ProjectGraph>("package_workflow = 'git-submodules'")
                .unwrap_err()
                .to_string();
        assert!(
            error.contains("unknown variant `git-submodules`"),
            "{error}"
        );
        assert!(error.contains("wally") && error.contains("none"), "{error}");
    }
    #[test]
    fn project_template_round_trips_in_an_injected_location() {
        let dir =
            std::env::temp_dir().join(format!("rproj-template-config-test-{}", std::process::id()));
        let path = dir.join("templates").join("default.project.json");
        let _ = fs::remove_dir_all(&dir);
        let template = serde_json::json!({"name": "ProjectName", "tree": {}});

        project_template::EditSession::open_at(&path)
            .unwrap()
            .save(&template, |_| Ok(()))
            .expect("save template");
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

        let mut session = project_template::EditSession::open_at(&path).unwrap();
        assert!(session.reset().expect("reset template"));
        assert!(!path.exists());
        assert!(sibling.exists(), "reset removed unrelated configuration");
        assert!(!session.reset().expect("repeat reset"));
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
        project_template::EditSession::open_at(&path)
            .unwrap()
            .save(&template, |_| Ok(()))
            .expect("replace template");

        assert_eq!(
            fs::read_to_string(&path).expect("read replacement"),
            "{\n  \"name\": \"ProjectName\",\n  \"tree\": {}\n}\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
