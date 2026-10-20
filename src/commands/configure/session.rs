use super::checked;
use crate::catalog::tool_settings::ConfigurableTool;
use anyhow::Result;
use rproj_core::tool_configuration::{SettingsContext, SettingsDraft};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub(super) struct EditSession {
    pub tool: &'static ConfigurableTool,
    draft: SettingsDraft,
    path: PathBuf,
    context: SettingsContext,
}
impl std::ops::Deref for EditSession {
    type Target = SettingsDraft;
    fn deref(&self) -> &Self::Target {
        &self.draft
    }
}
impl std::ops::DerefMut for EditSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.draft
    }
}
impl EditSession {
    pub fn load(project: &Path, tool: &'static ConfigurableTool) -> Result<Self> {
        let context = checked::capture(project, tool.key)?;
        let draft = SettingsDraft::new(checked::values(&context)?);
        Ok(Self {
            tool,
            path: context.project.join(tool.target.relative()),
            draft,
            context,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn dirty(&self) -> bool {
        self.draft.dirty()
    }
    pub fn value(&self, index: usize) -> Option<&Value> {
        self.draft.value(index)
    }
    pub fn set(&mut self, index: usize, value: Value) {
        self.draft.set(index, value);
    }
    pub fn save(&mut self) -> Result<bool> {
        let review = checked::prepare(&self.context, &self.draft)?;
        let changed = review.contents.is_some();
        let saved = checked::execute(&review, &mut || Ok(()))?;
        self.draft = SettingsDraft::new(checked::values(&saved)?);
        self.context = saved;
        Ok(changed)
    }
    #[cfg(test)]
    fn save_with(
        &mut self,
        persist: impl FnOnce(tempfile::NamedTempFile, &Path) -> Result<()>,
    ) -> Result<bool> {
        let review = checked::prepare(&self.context, &self.draft)?;
        let changed = review.contents.is_some();
        let saved = checked::execute_with(&review, &mut || Ok(()), persist)?;
        self.draft = SettingsDraft::new(checked::values(&saved)?);
        self.context = saved;
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    fn read_snapshot(path: &std::path::Path) -> std::io::Result<Option<String>> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    #[test]
    fn captured_writer_fixtures_before_extraction() {
        use crate::catalog::tool_settings;
        use serde_json::json;
        let tool = tool_settings::find("stylua").unwrap();
        let width = tool
            .settings
            .iter()
            .find(|s| s.key == "column_width")
            .unwrap();
        assert_eq!(
            tool_settings::checked_toml_merge(
                "# team\ncolumn_width = 91 # keep\ncustom = 'yes'\n",
                &[(width, json!(120))]
            )
            .unwrap(),
            "# team\ncolumn_width = 120   # keep\ncustom = 'yes'\n"
        );
        let values = serde_json::from_str(r#"{"custom":[1,2],"luau-lsp.plugin.enabled":false,"luau-lsp.studioPlugin.enabled":true}"#).unwrap();
        assert_eq!(
            crate::steps::vscode::merge_settings_values(
                values,
                &[("editor.formatOnSave", json!(true))]
            )
            .unwrap(),
            "{\n  \"custom\": [\n    1,\n    2\n  ],\n  \"editor.formatOnSave\": true,\n  \"luau-lsp.studioPlugin.enabled\": true\n}\n"
        );
    }
    use super::*;
    use crate::catalog::tool_settings;
    use serde_json::json;
    use std::fs;

    #[test]
    #[cfg(windows)]
    fn background_owner_refuses_save_and_keeps_the_pending_draft() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        fs::write(&path, "column_width=91\ncustom='keep'\n").unwrap();
        let tool = tool_settings::find("stylua").unwrap();
        let mut session = EditSession::load(root.path(), tool).unwrap();
        let index = tool
            .settings
            .iter()
            .position(|setting| setting.key == "column_width")
            .unwrap();
        session.set(index, json!(100));
        crate::background_watch::with_unresponsive_fixture(root.path(), || {
            assert!(
                session
                    .save()
                    .unwrap_err()
                    .to_string()
                    .contains("Stop background Watch")
            );
            assert!(session.dirty());
            assert_eq!(
                fs::read_to_string(&path).unwrap(),
                "column_width=91\ncustom='keep'\n"
            );
        });
        assert!(session.save().unwrap());
        assert!(fs::read_to_string(path).unwrap().contains("custom"));
    }

    #[test]
    fn a_read_only_destination_keeps_pending_changes_for_retry() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        fs::write(&path, "column_width=91\n").unwrap();
        let mut session =
            EditSession::load(root.path(), tool_settings::find("stylua").unwrap()).unwrap();
        let index = setting(&session, "column_width");
        session.set(index, json!(100));
        let original = fs::metadata(&path).unwrap().permissions();
        let mut readonly = original.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = session.save();
        fs::set_permissions(&path, original).unwrap();
        assert!(result.unwrap_err().to_string().contains("read-only"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "column_width=91\n");
        assert!(session.dirty());
        assert!(session.save().unwrap());
    }

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

    #[cfg(windows)]
    #[test]
    fn locked_destination_keeps_source_and_draft_then_allows_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("stylua.toml");
        let original = "column_width=91\n";
        fs::write(&path, original).unwrap();
        let mut session =
            EditSession::load(root.path(), tool_settings::find("stylua").unwrap()).unwrap();
        let index = setting(&session, "column_width");
        session.set(index, json!(100));
        // Permit reads and writes, but hold back FILE_SHARE_DELETE so rename fails.
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        assert!(
            session
                .save()
                .unwrap_err()
                .to_string()
                .contains("failed to save")
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        assert!(session.dirty());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
        drop(held);
        assert!(session.save().unwrap());
        assert!(fs::read_to_string(&path).unwrap().contains("100"));
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
