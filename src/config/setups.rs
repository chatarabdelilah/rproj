use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};

use crate::catalog::{capabilities, wally_packages};
use crate::graph::ProjectGraph;

pub fn validate_name(name: &str) -> Result<(), &'static str> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name
            .chars()
            .any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Use a single name without reserved path characters.");
    }
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    {
        return Err("This name is reserved by Windows.");
    }
    Ok(())
}

fn unlinked(path: &Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = meta.file_type().is_symlink();
                ensure!(
                    !linked,
                    "Linked setup paths are not supported: {}",
                    ancestor.display()
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Cannot inspect {}", ancestor.display()));
            }
        }
    }
    Ok(())
}

pub fn setup_path(root: &Path, name: &str) -> Result<PathBuf> {
    validate_name(name).map_err(anyhow::Error::msg)?;
    let path = root.join(format!("{name}.toml"));
    unlinked(&path)?;
    Ok(path)
}

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub warning: Option<String>,
}

pub fn list(root: &Path) -> Result<Vec<Entry>> {
    unlinked(root)?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error).context("Cannot read saved setup storage"),
    };
    let mut result = vec![];
    for entry in entries {
        let entry = entry.context("Cannot read saved setup entry")?;
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if entry.file_type()?.is_dir() {
            continue;
        }
        let warning = match Document::load(root, name) {
            Ok(document) => document.problem,
            Err(error) => Some(format!("{error:#}")),
        };
        result.push(Entry {
            name: name.into(),
            warning,
        });
    }
    result.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    Ok(result)
}

pub struct Document {
    pub name: String,
    path: PathBuf,
    pub bytes: Vec<u8>,
    pub parsed: Option<toml::Value>,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
}

impl Document {
    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let path = setup_path(root, name)?;
        ensure!(
            fs::metadata(&path)?.is_file(),
            "Setup is not a regular file"
        );
        let bytes = fs::read(&path).with_context(|| format!("Cannot read {}", path.display()))?;
        let parsed = std::str::from_utf8(&bytes)
            .context("Setup is not UTF-8")
            .and_then(|text| toml::from_str::<toml::Value>(text).context("Malformed setup TOML"));
        let (parsed, graph, problem) = match parsed {
            Ok(value) => match value.clone().try_into::<ProjectGraph>() {
                Ok(graph) => (Some(value), Some(graph), None),
                Err(error) => (
                    Some(value),
                    None,
                    Some(format!("Unsupported setup structure: {error}")),
                ),
            },
            Err(error) => (None, None, Some(format!("{error:#}"))),
        };
        Ok(Self {
            name: name.into(),
            path,
            bytes,
            parsed,
            graph,
            problem,
        })
    }

    pub fn check_current(&self) -> Result<()> {
        unlinked(&self.path)?;
        ensure!(
            fs::read(&self.path)
                .context("Setup was removed or cannot be read; refresh before retrying")?
                == self.bytes,
            "Setup changed outside rproj; refresh before retrying. Nothing was overwritten."
        );
        Ok(())
    }

    pub fn save(&mut self, graph: &ProjectGraph) -> Result<bool> {
        let original = self
            .graph
            .as_ref()
            .context("Guided editing is unavailable for this document")?;
        validate_changes(original, graph)?;
        let old = toml::Value::try_from(original)?;
        let new = toml::Value::try_from(graph)?;
        self.check_current()?;
        if old == new {
            return Ok(false);
        }
        let mut merged = self.parsed.clone().context("Malformed setup TOML")?;
        for key in ["package_workflow", "packages", "dropped", "capabilities"] {
            if old.get(key) != new.get(key) {
                merged
                    .as_table_mut()
                    .context("Expected a setup table")?
                    .insert(key.into(), new[key].clone());
            }
        }
        let bytes = toml::to_string_pretty(&merged)?.into_bytes();
        let mut staged = stage(&self.path, &bytes)?;
        self.check_current()?;
        staged.flush()?;
        staged
            .persist(&self.path)
            .context("Could not replace setup; saved content was not changed")?;
        self.bytes = bytes;
        self.parsed = Some(merged);
        self.graph = Some(graph.clone());
        Ok(true)
    }

    pub fn duplicate(&self, name: &str) -> Result<PathBuf> {
        let destination = self.destination(name)?;
        let staged = stage(&destination, &self.bytes)?;
        self.check_current()?;
        staged
            .persist_noclobber(&destination)
            .context("Destination already exists or cannot be created; no setup was replaced")?;
        Ok(destination)
    }

    pub fn rename(&self, name: &str) -> Result<()> {
        if self.name == name {
            return self.check_current();
        }
        ensure!(
            self.name.to_lowercase() != name.to_lowercase(),
            "Case-only rename is not supported; rename to an intermediate distinct name first."
        );
        let destination = self.duplicate(name)?;
        self.remove_after_copy(&destination)
    }

    fn remove_after_copy(&self, destination: &Path) -> Result<()> {
        self.check_current().and_then(|()| fs::remove_file(&self.path).map_err(Into::into))
            .with_context(|| format!("Copy saved at {}, but source could not be removed. Both copies may remain; refresh and inspect them.", destination.display()))
    }

    pub fn delete(&self) -> Result<()> {
        self.check_current()?;
        fs::remove_file(&self.path).context("Could not delete setup")
    }

    fn destination(&self, name: &str) -> Result<PathBuf> {
        let root = self
            .path
            .parent()
            .context("Setup has no storage directory")?;
        let destination = setup_path(root, name)?;
        let collision = fs::read_dir(root)?.try_fold(false, |found, entry| -> Result<bool> {
            Ok(found
                || entry?.file_name().to_string_lossy().to_lowercase()
                    == format!("{name}.toml").to_lowercase())
        })?;
        ensure!(
            !collision,
            "A setup with this name already exists; choose another name."
        );
        Ok(destination)
    }
}

fn stage(path: &Path, bytes: &[u8]) -> Result<tempfile::NamedTempFile> {
    unlinked(path)?;
    let mut staged =
        tempfile::NamedTempFile::new_in(path.parent().context("Missing storage directory")?)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    Ok(staged)
}

pub(super) fn replace(path: &Path, bytes: &[u8]) -> Result<()> {
    unlinked(path)?;
    fs::create_dir_all(path.parent().context("Missing setup directory")?)?;
    let staged = stage(path, bytes)?;
    unlinked(path)?;
    staged.persist(path).context("Could not save setup")?;
    Ok(())
}

pub fn unsupported(graph: &ProjectGraph) -> bool {
    graph
        .packages
        .iter()
        .any(|key| wally_packages::find(key).is_none())
        || graph.capabilities.iter().any(|(key, value)| {
            capabilities::find(key)
                .is_none_or(|capability| capability.implementation(value).is_none())
        })
}

fn validate_changes(original: &ProjectGraph, graph: &ProjectGraph) -> Result<()> {
    ensure!(
        original.mode == graph.mode,
        "Setup provenance must not change"
    );
    if unsupported(original) {
        ensure!(
            original.packages == graph.packages
                && original.capabilities == graph.capabilities
                && original.package_workflow == graph.package_workflow
                && original.dropped == graph.dropped,
            "Unsupported choices are read-only; their editing consequences cannot be resolved safely."
        );
    }
    ensure!(
        graph.testing_is_compatible(),
        "Jest Roblox requires Wally; select TestEZ or disable Testing."
    );
    for (key, implementation) in &graph.capabilities {
        if let Some(capability) = capabilities::find(key)
            && capability.implementation(implementation).is_some()
        {
            ensure!(
                capability
                    .implementations_for(graph.package_workflow)
                    .iter()
                    .any(|item| item.key == implementation),
                "{key}/{implementation} is incompatible with the selected dependencies"
            );
            ensure!(
                capability
                    .requires
                    .iter()
                    .all(|required| graph.capabilities.contains_key(*required)),
                "{key} requires {}",
                capability.requires.join(", ")
            );
        }
    }
    if graph.package_workflow == super::PackageWorkflow::None && !graph.packages.is_empty() {
        bail!("Dependency-free setups cannot contain packages");
    }
    if graph.package_workflow == super::PackageWorkflow::GitSubmodules {
        ensure!(
            wally_packages::unvendorable_in_closure(&graph.package_set()).is_empty(),
            "Some selected packages require Wally"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "# preserved until edited\nmode = 'like:original'\npackage_workflow = 'wally'\npackages = []\ndropped = ['future-file']\nfuture = { enabled = true }\n[capabilities]\ntest = 'testez'\n";

    fn fixture(bytes: &[u8]) -> (tempfile::TempDir, Document) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("sample.toml"), bytes).unwrap();
        let document = Document::load(root.path(), "sample").unwrap();
        (root, document)
    }

    #[test]
    fn opening_and_unchanged_save_preserve_bytes() {
        let (_root, mut document) = fixture(SOURCE.as_bytes());
        let graph = document.graph.clone().unwrap();
        assert!(!document.save(&graph).unwrap());
        assert_eq!(fs::read(&document.path).unwrap(), SOURCE.as_bytes());
        assert_eq!(graph.mode, "like:original");
    }

    #[test]
    fn changed_save_preserves_unknown_fields_exclusions_and_provenance() {
        let (_root, mut document) = fixture(SOURCE.as_bytes());
        let mut graph = document.graph.clone().unwrap();
        graph.capabilities.clear();
        assert!(document.save(&graph).unwrap());
        let parsed = document.parsed.as_ref().unwrap();
        assert_eq!(parsed["future"]["enabled"].as_bool(), Some(true));
        assert_eq!(parsed["mode"].as_str(), Some("like:original"));
        assert_eq!(parsed["dropped"][0].as_str(), Some("future-file"));
        assert!(!document.save(&graph).unwrap());
    }

    #[test]
    fn unsupported_choices_are_never_normalized() {
        let source = SOURCE.replace("packages = []", "packages = ['future-package']");
        let (_root, mut document) = fixture(source.as_bytes());
        let mut graph = document.graph.clone().unwrap();
        assert!(unsupported(&graph));
        assert!(!document.save(&graph).unwrap());
        graph.packages.clear();
        assert!(document.save(&graph).is_err());
        assert_eq!(document.bytes, source.as_bytes());
    }

    #[test]
    fn external_changes_block_all_mutations() {
        let (root, mut document) = fixture(SOURCE.as_bytes());
        fs::write(&document.path, b"changed = true").unwrap();
        let graph = document.graph.clone().unwrap();
        assert!(document.save(&graph).is_err());
        assert!(document.duplicate("copy").is_err());
        assert!(document.rename("renamed").is_err());
        assert!(document.delete().is_err());
        assert!(!root.path().join("copy.toml").exists());
        assert_eq!(fs::read(&document.path).unwrap(), b"changed = true");
    }

    #[test]
    fn malformed_documents_support_byte_preserving_management() {
        let (root, document) = fixture(b"invalid [ TOML\xff");
        assert!(document.graph.is_none());
        assert!(document.problem.is_some());
        document.duplicate("copy").unwrap();
        assert_eq!(
            fs::read(root.path().join("copy.toml")).unwrap(),
            document.bytes
        );
        assert!(document.duplicate("COPY").is_err());
        document.rename("renamed").unwrap();
        assert!(!document.path.exists());
        let renamed = Document::load(root.path(), "renamed").unwrap();
        renamed.delete().unwrap();
        assert!(!root.path().join("renamed.toml").exists());
    }

    #[test]
    fn rename_noop_case_only_and_partial_failure() {
        let (root, document) = fixture(SOURCE.as_bytes());
        document.rename("sample").unwrap();
        assert!(document.rename("SAMPLE").is_err());
        let copy = document.duplicate("copy").unwrap();
        fs::write(&document.path, "external = true").unwrap();
        let error = document.remove_after_copy(&copy).unwrap_err();
        assert!(error.to_string().contains("Both copies"));
        assert!(root.path().join("copy.toml").exists());
        assert!(document.path.exists());
    }

    #[test]
    fn invalid_jest_can_be_repaired_but_not_saved_unchanged() {
        let source = SOURCE
            .replace("'wally'", "'none'")
            .replace("'testez'", "'jest-roblox'");
        let (_root, mut document) = fixture(source.as_bytes());
        let mut graph = document.graph.clone().unwrap();
        assert!(document.save(&graph).is_err());
        graph.capabilities.insert("test".into(), "testez".into());
        assert!(document.save(&graph).unwrap());
    }

    #[test]
    fn missing_storage_does_not_create_directories_and_listing_is_sorted() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        assert!(list(&missing).unwrap().is_empty());
        assert!(!missing.exists());
        for name in ["z", "Alpha", "beta"] {
            fs::write(root.path().join(format!("{name}.toml")), "invalid [").unwrap();
        }
        let entries = list(root.path()).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "beta", "z"]
        );
        assert!(entries.iter().all(|entry| entry.warning.is_some()));
    }

    #[test]
    fn names_reject_traversal_and_windows_reserved_forms() {
        for name in [
            "", "..", "../x", "a\\b", "C:foo", "NUL", "con.txt", "COM1", "LPT9", "COM¹", "CONIN$",
            "a.", "a ",
        ] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        for name in ["sample", "COM10", "my setup", "日本語"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn linked_files_and_storage_are_rejected() {
        use std::os::unix::fs::symlink;
        let (root, document) = fixture(SOURCE.as_bytes());
        symlink(&document.path, root.path().join("linked.toml")).unwrap();
        assert!(Document::load(root.path(), "linked").is_err());
        let alias = root.path().join("alias");
        symlink(root.path(), &alias).unwrap();
        assert!(list(&alias).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn junction_storage_and_locked_destination_are_refused() {
        use std::os::windows::fs::OpenOptionsExt;
        let (root, mut document) = fixture(SOURCE.as_bytes());
        let alias = root.path().join("linked-storage");
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&alias)
            .arg(root.path())
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        assert!(list(&alias).is_err());
        assert!(Document::load(&alias, "sample").is_err());
        std::fs::remove_dir(&alias).unwrap();
        let handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&document.path)
            .unwrap();
        let mut graph = document.graph.clone().unwrap();
        graph.capabilities.clear();
        assert!(document.save(&graph).is_err());
        assert_eq!(document.bytes, SOURCE.as_bytes());
        assert_eq!(fs::read(&document.path).unwrap(), SOURCE.as_bytes());
        drop(handle);
        assert!(document.save(&graph).unwrap());
    }
}
