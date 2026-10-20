//! Shared terminal/worker planner and single-file checked executor.
use crate::{
    catalog::tool_settings::{self, ConfigTarget, ConfigurableTool},
    steps::vscode,
};

#[cfg(test)]
mod tests;
use anyhow::{Context, Result, ensure};
use rproj_core::tool_configuration::{SettingsContext, SettingsDraft, SettingsReview};
use serde_json::Value;
use std::{collections::BTreeMap, fs, io::ErrorKind, path::Path};

fn identity(path: &Path, directory: bool) -> Result<Option<String>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("Cannot inspect {}", path.display())),
    };
    ensure!(
        !meta.file_type().is_symlink(),
        "{} is linked; use regular project files",
        path.display()
    );
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            meta.file_attributes() & 0x400 == 0,
            "{} is a reparse point; use regular project files",
            path.display()
        );
    }
    ensure!(
        if directory {
            meta.is_dir()
        } else {
            meta.is_file()
        },
        "Unexpected file type at {}",
        path.display()
    );
    #[cfg(windows)]
    {
        use winsafe::{HFILE, co};
        let (handle, _) = HFILE::CreateFile(
            path.to_str()
                .context("Settings path is not valid Unicode")?,
            co::GENERIC::default(),
            Some(co::FILE_SHARE::READ | co::FILE_SHARE::WRITE | co::FILE_SHARE::DELETE),
            None,
            co::DISPOSITION::OPEN_EXISTING,
            co::FILE_ATTRIBUTE::NORMAL,
            Some(co::FILE_FLAG::BACKUP_SEMANTICS | co::FILE_FLAG::OPEN_REPARSE_POINT),
            None,
            None,
        )?;
        let info = handle.GetFileInformationByHandle()?;
        ensure!(
            !info.dwFileAttributes.has(co::FILE_ATTRIBUTE::REPARSE_POINT),
            "Settings path became a reparse point"
        );
        Ok(Some(format!(
            "{}:{}",
            info.dwVolumeSerialNumber,
            info.nFileIndex()
        )))
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Some(format!("{}:{}", meta.dev(), meta.ino())))
    }
    #[cfg(not(any(windows, unix)))]
    {
        Ok(Some(format!("{:?}", meta.created()?)))
    }
}

pub(crate) fn capture(project: &Path, tool: &str) -> Result<SettingsContext> {
    let tool = tool_settings::find(tool).context("Unknown settings target")?;
    // The chosen location may have linked ancestors (for example macOS /var).
    // Refuse a linked project leaf, then capture the canonical root and children.
    ensure!(
        identity(project, true)?.is_some(),
        "The project directory no longer exists."
    );
    let project = project
        .canonicalize()
        .context("Cannot open the captured project")?;
    let mut identities = BTreeMap::new();
    identities.insert(String::new(), identity(&project, true)?);
    if matches!(tool.target, ConfigTarget::VsCodeSettings) {
        identities.insert(".vscode".into(), identity(&project.join(".vscode"), true)?);
    }
    let relative = tool.target.relative();
    let path = project.join(relative);
    let file_identity = identity(&path, false)?;
    let original = if file_identity.is_some() {
        Some(fs::read_to_string(&path).with_context(|| format!("Cannot read {}", path.display()))?)
    } else {
        None
    };
    ensure!(
        identity(&path, false)? == file_identity,
        "Settings file changed while reading; Reload from file"
    );
    identities.insert(relative.into(), file_identity);
    let context = SettingsContext {
        project,
        tool: tool.key.into(),
        identities,
        original,
    };
    context.validate()?;
    Ok(context)
}

pub(crate) fn values(context: &SettingsContext) -> Result<Vec<Option<Value>>> {
    context.validate()?;
    let tool = context.target()?;
    let text = context.original.as_deref().unwrap_or("");
    match tool.target {
        ConfigTarget::ProjectToml { .. } => {
            toml::from_str::<toml::Table>(text)
                .context("could not parse current TOML; repair the file and reload the editor")?;
            Ok(tool_settings::current_toml_values(tool, text))
        }
        ConfigTarget::VsCodeSettings => {
            let object =
                vscode::parse_settings(&context.project.join(tool.target.relative()), text)?;
            Ok(tool
                .settings
                .iter()
                .map(|s| object.get(s.key).cloned())
                .collect())
        }
    }
}

fn verify(context: &SettingsContext) -> Result<()> {
    context.validate()?;
    ensure!(
        capture(&context.project, &context.tool)? == *context,
        "The file or project changed outside this editor; Reload from file before saving"
    );
    Ok(())
}

pub(crate) fn prepare(context: &SettingsContext, draft: &SettingsDraft) -> Result<SettingsReview> {
    verify(context)?;
    let tool = context.target()?;
    draft.validate(tool)?;
    ensure!(
        values(context)? == draft.current,
        "Draft baseline changed; Reload from file"
    );
    let contents = if draft.dirty() {
        Some(merge(context, draft, tool)?)
    } else {
        None
    };
    verify(context)?;
    let review = SettingsReview {
        context: context.clone(),
        draft: draft.clone(),
        contents,
    };
    review.validate()?;
    Ok(review)
}

fn merge(
    context: &SettingsContext,
    draft: &SettingsDraft,
    tool: &ConfigurableTool,
) -> Result<String> {
    let answers: Vec<_> = tool
        .settings
        .iter()
        .zip(&draft.changes)
        .filter_map(|(s, v)| v.clone().map(|v| (s, v)))
        .collect();
    let text = context.original.as_deref().unwrap_or("");
    let merged = match tool.target {
        ConfigTarget::ProjectToml { .. } => tool_settings::checked_toml_merge(text, &answers)?,
        ConfigTarget::VsCodeSettings => vscode::merge_settings_values(
            vscode::parse_settings(&context.project.join(tool.target.relative()), text)?,
            &answers
                .iter()
                .map(|(s, v)| (s.key, v.clone()))
                .collect::<Vec<_>>(),
        )?,
    };
    let mut proposed = context.clone();
    proposed.original = Some(merged.clone());
    // Parsing the proposed bytes does not need a real file identity.
    proposed
        .identities
        .insert(tool.target.relative().into(), Some("proposed".into()));
    let actual = values(&proposed)?;
    for ((setting, wanted), actual) in tool.settings.iter().zip(&draft.changes).zip(actual) {
        if let Some(wanted) = wanted {
            ensure!(
                actual.as_ref() == Some(wanted),
                "Cannot save {}: the writer removes or changes this setting when its replacement exists. Revert this change; other edits can still be saved.",
                setting.display_key()
            );
        }
    }
    Ok(merged)
}

pub(crate) fn execute(
    review: &SettingsReview,
    before_write: &mut impl FnMut() -> Result<()>,
) -> Result<SettingsContext> {
    execute_with(review, before_write, |pending, path| {
        pending.persist(path).map_err(|e| e.error)?;
        Ok(())
    })
}

pub(super) fn execute_with(
    review: &SettingsReview,
    before_write: &mut impl FnMut() -> Result<()>,
    persist: impl FnOnce(tempfile::NamedTempFile, &Path) -> Result<()>,
) -> Result<SettingsContext> {
    review.validate()?;
    let _guard = if review.contents.is_some() {
        Some(crate::background_watch::mutation_guard(
            Some(&review.context.project),
            false,
        )?)
    } else {
        None
    };
    ensure!(
        prepare(&review.context, &review.draft)? == *review,
        "Reviewed settings changed; prepare Review again"
    );
    let Some(contents) = &review.contents else {
        return Ok(review.context.clone());
    };
    before_write()?;
    verify(&review.context)?;
    let mut expected = review.context.clone();
    // Claim only this known missing parent, exclusively; do not accept a racing creator.
    if expected.identities.get(".vscode") == Some(&None) {
        let parent = expected.project.join(".vscode");
        fs::create_dir(&parent)
            .context("Could not create .vscode; Reload from file before retrying")?;
        expected
            .identities
            .insert(".vscode".into(), identity(&parent, true)?);
    }
    verify(&expected)?;
    let path = expected.project.join(expected.target()?.target.relative());
    let pending =
        crate::file_replace::stage(&path, contents.as_bytes(), expected.original.is_some())?;
    verify(&expected)?;
    persist(pending, &path).with_context(|| format!("failed to save {}", path.display()))?;
    let saved = capture(&expected.project, &expected.tool)?;
    ensure!(
        saved.original.as_ref() == Some(contents),
        "Saved file changed before confirmation; Reload from file to inspect the outcome"
    );
    Ok(saved)
}
