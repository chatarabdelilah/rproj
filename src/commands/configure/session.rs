use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use serde_json::Value;

use super::{checked_toml_merge, target_description};
use crate::catalog::tool_settings::{self, ConfigTarget, ConfigurableTool};
use crate::steps::vscode;

pub(super) struct EditSession {
    pub tool: &'static ConfigurableTool,
    pub current: Vec<Option<Value>>,
    pub changes: Vec<Option<Value>>,
    project: PathBuf,
    path: PathBuf,
    baseline: Option<String>,
}

impl EditSession {
    pub fn load(project: &Path, tool: &'static ConfigurableTool) -> Result<Self> {
        let path = project.join(target_description(&tool.target));
        let baseline = read_snapshot(&path)?;
        // Defaults and conflict detection must describe the same read of the file.
        let text = baseline.as_deref().unwrap_or("");
        let current = match tool.target {
            ConfigTarget::ProjectToml { .. } => {
                toml::from_str::<toml::Table>(text).with_context(|| {
                    format!(
                        "could not parse {} - fix or delete it, then re-run",
                        path.display()
                    )
                })?;
                tool_settings::current_toml_values(tool, text)
            }
            ConfigTarget::VsCodeSettings => {
                let settings = vscode::parse_settings(&path, text)?;
                tool.settings
                    .iter()
                    .map(|setting| settings.get(setting.key).cloned())
                    .collect()
            }
        };
        Ok(Self {
            tool,
            changes: vec![None; current.len()],
            current,
            project: project.to_owned(),
            path,
            baseline,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn dirty(&self) -> bool {
        self.changes.iter().any(Option::is_some)
    }

    pub fn value(&self, index: usize) -> Option<&Value> {
        self.changes[index]
            .as_ref()
            .or(self.current[index].as_ref())
    }

    pub fn set(&mut self, index: usize, value: Value) {
        self.changes[index] = (self.current[index].as_ref() != Some(&value)).then_some(value);
    }

    fn ensure_current(&self) -> Result<()> {
        ensure!(
            self.project.is_dir(),
            "The project directory no longer exists."
        );
        ensure!(
            read_snapshot(&self.path)? == self.baseline,
            "The file changed outside this editor: {}. Reopen it or re-run configure before saving.",
            self.path.display()
        );
        Ok(())
    }

    pub fn save(&mut self) -> Result<bool> {
        self.save_with(|pending, path| {
            pending.persist(path).map_err(|error| error.error)?;
            Ok(())
        })
    }

    fn save_with(
        &mut self,
        persist: impl FnOnce(tempfile::NamedTempFile, &Path) -> Result<()>,
    ) -> Result<bool> {
        self.ensure_current()?;
        if !self.dirty() {
            return Ok(false);
        }
        let answers: Vec<_> = self
            .tool
            .settings
            .iter()
            .zip(&self.changes)
            .filter_map(|(setting, value)| value.clone().map(|value| (setting, value)))
            .collect();
        let text = self.baseline.as_deref().unwrap_or("");
        let merged = match self.tool.target {
            ConfigTarget::ProjectToml { .. } => checked_toml_merge(text, &answers)?,
            ConfigTarget::VsCodeSettings => vscode::merge_settings_values(
                vscode::parse_settings(&self.path, text)?,
                &answers
                    .iter()
                    .map(|(setting, value)| (setting.key, value.clone()))
                    .collect::<Vec<_>>(),
            )?,
        };
        let parent = self.path.parent().context("Settings file has no parent")?;
        fs::create_dir_all(parent)?;
        let mut pending = tempfile::NamedTempFile::new_in(parent)?;
        pending.write_all(merged.as_bytes())?;
        pending.as_file().sync_all()?;
        // Catch edits made during merging/writing too. This is optimistic conflict
        // detection; arbitrary external writers do not participate in a file lock.
        self.ensure_current()?;
        persist(pending, &self.path)
            .with_context(|| format!("failed to save {}", self.path.display()))?;
        for (current, change) in self.current.iter_mut().zip(&mut self.changes) {
            if let Some(value) = change.take() {
                *current = Some(value);
            }
        }
        self.baseline = Some(merged);
        Ok(true)
    }
}

fn read_snapshot(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setting(session: &EditSession, key: &str) -> usize {
        session
            .tool
            .settings
            .iter()
            .position(|s| s.key == key)
            .unwrap()
    }

    #[test]
    fn no_op_and_reverted_changes_preserve_original_bytes_for_both_formats() {
        for (tool, relative, original, key, current, changed) in [
            (
                "stylua",
                "stylua.toml",
                "# custom\r\ncolumn_width=91\r\nquote_style=['future']\r\n",
                "column_width",
                json!(91),
                json!(100),
            ),
            (
                "stylua-vscode",
                ".vscode/settings.json",
                "{ \"editor.formatOnSave\" : false, \"custom\": [1, 2] }",
                "editor.formatOnSave",
                json!(false),
                json!(true),
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, original).unwrap();
            let mut session =
                EditSession::load(root.path(), tool_settings::find(tool).unwrap()).unwrap();
            assert!(!session.save().unwrap());
            let index = setting(&session, key);
            session.set(index, changed);
            assert!(session.dirty());
            session.set(index, current);
            assert!(!session.save().unwrap());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
            assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        }
    }

    #[test]
    fn external_replacement_creation_and_deletion_keep_pending_changes() {
        for mode in ["replace", "create-empty", "delete-empty"] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("stylua.toml");
            if mode != "create-empty" {
                fs::write(&path, "").unwrap();
            }
            let mut session =
                EditSession::load(root.path(), tool_settings::find("stylua").unwrap()).unwrap();
            let index = setting(&session, "column_width");
            session.set(index, json!(100));
            match mode {
                "replace" => fs::write(&path, "column_width=120\n").unwrap(),
                "create-empty" => fs::write(&path, "").unwrap(),
                _ => fs::remove_file(&path).unwrap(),
            }
            let external = read_snapshot(&path).unwrap();
            assert!(session.save().unwrap_err().to_string().contains("outside"));
            assert_eq!(read_snapshot(&path).unwrap(), external);
            assert_eq!(session.value(index), Some(&json!(100)));
            assert!(session.dirty());
        }
    }

    #[test]
    fn failed_replacement_cleans_temp_file_and_allows_retry_then_another_save() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        let original = "column_width=91\ncustom=['keep']\n";
        fs::write(&path, original).unwrap();
        let mut session =
            EditSession::load(root.path(), tool_settings::find("stylua").unwrap()).unwrap();
        let index = setting(&session, "column_width");
        session.set(index, json!(100));
        let error = session
            .save_with(|pending, destination| {
                assert_eq!(pending.path().parent(), destination.parent());
                assert!(fs::read_to_string(pending.path()).unwrap().contains("100"));
                anyhow::bail!("injected replacement failure")
            })
            .unwrap_err();
        assert!(format!("{error:#}").contains("injected replacement failure"));
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert!(session.dirty());
        assert_eq!(session.current[index], Some(json!(91)));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        assert!(session.save().unwrap());
        assert!(!session.dirty());
        assert_eq!(session.current[index], Some(json!(100)));
        session.set(index, json!(110));
        assert!(session.save().unwrap());
        let saved: toml::Value = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["column_width"].as_integer(), Some(110));
        assert_eq!(saved["custom"][0].as_str(), Some("keep"));
    }

    #[test]
    fn invalid_merge_preserves_file_and_pending_changes() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("selene.toml");
        let original = "std = \"roblox\"\nnotes = '''\nstd = \"example\"\n'''\n";
        fs::write(&path, original).unwrap();
        let mut session =
            EditSession::load(root.path(), tool_settings::find("selene").unwrap()).unwrap();
        session.set(0, json!("roblox+testez"));
        assert!(
            session
                .save()
                .unwrap_err()
                .to_string()
                .contains("cannot safely edit")
        );
        assert!(session.dirty());
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn absent_json_is_created_only_on_save_and_deleted_project_is_not_recreated() {
        let root = tempfile::tempdir().unwrap();
        let mut session =
            EditSession::load(root.path(), tool_settings::find("stylua-vscode").unwrap()).unwrap();
        assert!(!session.save().unwrap());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        session.set(0, json!(true));
        assert!(session.save().unwrap());
        assert!(session.path().is_file());
        let project = root.path().join("removed");
        fs::create_dir(&project).unwrap();
        let mut session =
            EditSession::load(&project, tool_settings::find("stylua").unwrap()).unwrap();
        session.set(0, json!("Luau"));
        fs::remove_dir(&project).unwrap();
        assert!(
            session
                .save()
                .unwrap_err()
                .to_string()
                .contains("no longer exists")
        );
        assert!(!project.exists());
        assert!(session.dirty());
    }
}
