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
use std::{collections::BTreeMap, fs, path::Path};

use crate::path_identity::identity;

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
        capture(&context.project, &context.tool).map_err(|error| anyhow::anyhow!(
            "Could not revalidate captured settings: {error:#}. Reload from file"
        ))? == *context,
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

/// Diagnose field-specific writer refusals from captured bytes, without filesystem reads.
pub(crate) fn field_problems(context: &SettingsContext) -> Result<BTreeMap<usize, String>> {
    use crate::catalog::tool_settings::SettingKind;
    let tool = context.target()?;
    let current = values(context)?;
    let mut problems = BTreeMap::new();
    for (index, setting) in tool.settings.iter().enumerate() {
        let mut probe = SettingsDraft::new(current.clone());
        let mut value = setting.kind.default_value();
        if current[index].as_ref() == Some(&value) {
            value = match &setting.kind {
                SettingKind::Bool { default } => serde_json::json!(!default),
                SettingKind::Integer { default } => {
                    serde_json::json!(
                        default
                            .checked_add(1)
                            .unwrap_or_else(|| default.saturating_sub(1))
                    )
                }
                SettingKind::Choice { options, .. } => {
                    let Some(other) = options.iter().find(|o| Some(o.value) != value.as_str())
                    else {
                        continue;
                    };
                    serde_json::json!(other.value)
                }
            };
        }
        probe.set(index, value);
        if let Err(error) = merge(context, &probe, tool) {
            problems.insert(index, format!("{error:#}"));
        }
    }
    Ok(problems)
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
