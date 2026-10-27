use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};

use crate::graph::ProjectGraph;

pub use rproj_core::creation::validate_name;
pub use rproj_core::saved_setups::unsupported;
use rproj_core::saved_setups::{CapturedSetup, SetupAction};
pub(crate) mod checked;

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
    #[cfg(test)]
    path: PathBuf,
    captured: CapturedSetup,
    pub bytes: Vec<u8>,
    #[cfg(test)]
    pub parsed: Option<toml::Value>,
    pub graph: Option<ProjectGraph>,
    pub problem: Option<String>,
}

impl Document {
    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let captured = checked::capture(root, name)?;
        Ok(Self::from_captured(captured))
    }

    fn from_captured(captured: CapturedSetup) -> Self {
        let decoded = rproj_core::saved_setups::decode(&captured.bytes);
        Self {
            name: captured.name.clone(),
            #[cfg(test)]
            path: captured.root.join(format!("{}.toml", captured.name)),
            bytes: captured.bytes.clone(),
            captured,
            #[cfg(test)]
            parsed: decoded.parsed,
            graph: decoded.graph,
            problem: decoded.problem,
        }
    }

    pub fn save(&mut self, graph: &ProjectGraph) -> Result<bool> {
        let review = checked::prepare(&self.captured, SetupAction::Save(graph.clone()))?;
        let changed = review.document.as_ref().unwrap().replacement.is_some();
        let outcome = checked::completed(checked::execute(&review, &mut |_| Ok(())))?;
        *self = Self::from_captured(outcome.saved.context("Missing saved setup baseline")?);
        Ok(changed)
    }

    pub fn duplicate(&self, name: &str) -> Result<PathBuf> {
        let review = checked::prepare(&self.captured, SetupAction::Duplicate(name.into()))?;
        let outcome = checked::completed(checked::execute(&review, &mut |_| Ok(())))?;
        outcome.destination.context("Missing duplicate destination")
    }

    pub fn rename(&self, name: &str) -> Result<()> {
        let review = checked::prepare(&self.captured, SetupAction::Rename(name.into()))?;
        checked::completed(checked::execute(&review, &mut |_| Ok(())))?;
        Ok(())
    }

    pub fn delete(&self) -> Result<()> {
        let review = checked::prepare(&self.captured, SetupAction::Delete)?;
        checked::completed(checked::execute(&review, &mut |_| Ok(())))?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = "# preserved until edited\nmode = 'like:original'\npackage_workflow = 'wally'\npackages = []\ndropped = ['future-file']\nfuture = { enabled = true }\n[capabilities]\ntest = 'testez'\n";

    #[test]
    fn metadata_fixture_preserves_bytes_and_unknown_toml_types() {
        let bytes = include_bytes!("setups/fixtures/metadata.toml");
        let (_root, mut document) = fixture(bytes);
        let original = document.parsed.clone().unwrap();
        let graph = document.graph.clone().unwrap();
        assert!(!document.save(&graph).unwrap());
        assert_eq!(document.bytes, bytes);
        let mut changed = graph.clone();
        changed.capabilities.remove("format");
        assert!(document.save(&changed).unwrap());
        let saved = document.parsed.as_ref().unwrap();
        for key in ["mode", "team", "future", "dropped", "packages"] {
            assert_eq!(saved[key], original[key], "{key}");
        }
        assert!(saved["capabilities"].get("format").is_none());
        assert_eq!(saved["future"]["release_date"].type_str(), "datetime");
        assert!(!document.save(&changed).unwrap());
    }

    #[test]
    fn reverting_in_memory_and_management_preserve_crlf_and_missing_final_newline() {
        let bytes = SOURCE.replace('\n', "\r\n").trim_end().as_bytes().to_vec();
        let (root, mut document) = fixture(&bytes);
        let original = document.graph.clone().unwrap();
        let mut revised = original.clone();
        revised.capabilities.clear();
        revised = original;
        assert!(!document.save(&revised).unwrap());
        document.duplicate("日本語 copy").unwrap();
        assert_eq!(
            fs::read(root.path().join("日本語 copy.toml")).unwrap(),
            bytes
        );
        assert_eq!(document.bytes, bytes);
    }

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
        let review =
            checked::prepare(&document.captured, SetupAction::Rename("copy".into())).unwrap();
        let outcome = checked::execute(&review, &mut |state| {
            if state == rproj_core::saved_setups::MutationState::Copied {
                fs::write(&document.path, "external = true")?;
            }
            Ok(())
        });
        assert_eq!(
            outcome.status,
            rproj_core::saved_setups::ManagementStatus::Partial
        );
        assert!(outcome.message.unwrap().contains("Both copies"));
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
