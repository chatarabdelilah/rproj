//! Shared checked persistence. Callers supply validation, never replacement bytes.
use anyhow::{Context, Result, ensure};
use rproj_core::project_template::review::{TemplateAction, TemplateReview, TemplateSource};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

use crate::path_identity::identity;

pub(crate) fn capture(path: &Path, limit: Option<u64>) -> Result<TemplateSource> {
    let path = std::path::absolute(path)?;
    ensure!(
        !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir)),
        "Template path contains parent traversal"
    );
    let mut parents = BTreeMap::new();
    for parent in path
        .ancestors()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        parents.insert(parent.to_owned(), identity(parent, true)?);
    }
    let file_identity =
        identity(&path, false).with_context(|| format!("failed to read {}", path.display()))?;
    let original = if file_identity.is_some() {
        let mut bytes = Vec::new();
        fs::File::open(&path)?
            .take(limit.map_or(u64::MAX, |n| n.saturating_add(1)))
            .read_to_end(&mut bytes)?;
        ensure!(
            limit.is_none_or(|n| bytes.len() as u64 <= n),
            "Template exceeds the 2 MiB worker limit; use the terminal editor"
        );
        Some(
            String::from_utf8(bytes)
                .context("Template is not UTF-8; repair the file before editing")?,
        )
    } else {
        None
    };
    let source = TemplateSource {
        path,
        parents,
        identity: file_identity,
        original,
    };
    source.validate()?;
    verify_identities(&source)?;
    Ok(source)
}

fn verify_identities(source: &TemplateSource) -> Result<()> {
    for parent in source
        .path
        .ancestors()
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        ensure!(
            source.parents.get(parent) == Some(&identity(parent, true)?),
            "Template parent changed; Reload from file"
        );
    }
    ensure!(
        identity(&source.path, false)? == source.identity,
        "Template file identity changed; Reload from file"
    );
    Ok(())
}

pub(crate) fn verify_with_limit(source: &TemplateSource, limit: Option<u64>) -> Result<()> {
    source.validate()?;
    ensure!(
        capture(&source.path, limit).with_context(|| format!(
            "The template changed outside this editor: {}. Reload from file",
            source.path.display()
        ))? == *source,
        "The template changed outside this editor: {}. Your draft is retained. Reload from file before saving or resetting.",
        source.path.display()
    );
    Ok(())
}

pub(crate) fn prepare(
    source: &TemplateSource,
    action: TemplateAction,
    validate: impl FnOnce(&serde_json::Value) -> Result<()>,
) -> Result<TemplateReview> {
    prepare_with_limit(source, action, validate, None)
}

pub(crate) fn prepare_with_limit(
    source: &TemplateSource,
    action: TemplateAction,
    validate: impl FnOnce(&serde_json::Value) -> Result<()>,
    limit: Option<u64>,
) -> Result<TemplateReview> {
    verify_with_limit(source, limit)?;
    let review = TemplateReview::prepare(source.clone(), action)?;
    if let TemplateAction::Save(value) = &review.action
        && review.changes()
    {
        validate(value)?;
    }
    verify_with_limit(source, limit)?;
    Ok(review)
}

pub(crate) fn execute(
    review: &TemplateReview,
    before_write: &mut impl FnMut() -> Result<()>,
) -> Result<TemplateSource> {
    execute_with_limit(review, before_write, None)
}

pub(crate) fn execute_with_limit(
    review: &TemplateReview,
    before_write: &mut impl FnMut() -> Result<()>,
    limit: Option<u64>,
) -> Result<TemplateSource> {
    execute_with(review, before_write, limit, |pending, path, existing| {
        if existing {
            pending.persist(path).map_err(|e| e.error)?;
        } else {
            pending.persist_noclobber(path).map_err(|e| e.error)?;
        }
        Ok(())
    })
}

fn execute_with(
    review: &TemplateReview,
    before_write: &mut impl FnMut() -> Result<()>,
    limit: Option<u64>,
    persist: impl FnOnce(tempfile::NamedTempFile, &Path, bool) -> Result<()>,
) -> Result<TemplateSource> {
    review.validate()?;
    verify_with_limit(&review.source, limit)?;
    if !review.changes() {
        return Ok(review.source.clone());
    }
    // Staging may create directories, so report potential mutation first.
    before_write()?;
    verify_with_limit(&review.source, limit)?;
    let mut expected = review.source.clone();
    match review.action {
        TemplateAction::Save(_) => {
            for parent in expected
                .path
                .ancestors()
                .skip(1)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
            {
                if expected.parents.get(parent) == Some(&None) {
                    verify_with_limit(&expected, limit)?;
                    fs::create_dir(parent)
                        .context("Could not claim template directory; Reload from file")?;
                    expected
                        .parents
                        .insert(parent.to_owned(), identity(parent, true)?);
                }
            }
            verify_with_limit(&expected, limit)?;
            let contents = review.replacement.as_ref().context("Missing Save bytes")?;
            let pending = crate::file_replace::stage(
                &expected.path,
                contents.as_bytes(),
                expected.original.is_some(),
            )?;
            verify_with_limit(&expected, limit)?;
            persist(pending, &expected.path, expected.original.is_some())
                .with_context(|| format!("failed to replace {}", expected.path.display()))?;
        }
        TemplateAction::Reset => {
            verify_with_limit(&expected, limit)?;
            fs::remove_file(&expected.path).context("Could not remove the template override")?;
        }
    }
    let saved = capture(&expected.path, limit)?;
    ensure!(
        saved.parents == expected.parents
            && match review.action {
                TemplateAction::Save(_) => saved.original == review.replacement,
                TemplateAction::Reset => saved.original.is_none(),
            },
        "Template changed before completion could be confirmed; Reload from file"
    );
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed() -> serde_json::Value {
        let mut value = rproj_core::project_template::builtin_project_template();
        value["servePort"] = serde_json::json!(40001);
        value
    }

    #[test]
    fn noop_is_byte_preserving_and_missing_builtin_creates_nothing() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("missing/templates/default.project.json");
        let builtin = rproj_core::project_template::builtin_project_template();
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Save(builtin.clone()), |_| {
            panic!("no-op validates no tools")
        })
        .unwrap();
        assert!(!review.changes());
        execute(&review, &mut || panic!("no mutation")).unwrap();
        assert!(!path.parent().unwrap().exists());
        let path = root.path().join("custom.json");
        let text = format!("  {}  \r\n", serde_json::to_string(&builtin).unwrap());
        fs::write(&path, &text).unwrap();
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Save(builtin), |_| panic!("no-op")).unwrap();
        assert_eq!(
            execute(&review, &mut || panic!("no mutation")).unwrap(),
            source
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn preparation_is_read_only_and_callback_failure_precedes_directory_creation() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested/template.json");
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        assert!(execute(&review, &mut || anyhow::bail!("report failed")).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let mut reported = false;
        let saved = execute(&review, &mut || {
            assert!(!path.parent().unwrap().exists());
            reported = true;
            Ok(())
        })
        .unwrap();
        assert!(reported);
        assert_eq!(saved.value().unwrap(), changed());
    }

    #[test]
    fn missing_parent_or_target_races_block_before_replacement() {
        for target in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("nested/template.json");
            let source = capture(&path, None).unwrap();
            let review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
            let result = execute(&review, &mut || {
                fs::create_dir(path.parent().unwrap())?;
                if target {
                    fs::write(&path, "external")?;
                }
                Ok(())
            });
            assert!(result.is_err());
            assert_eq!(
                fs::read_to_string(&path).ok(),
                target.then(|| "external".into())
            );
        }
    }

    #[test]
    fn same_bytes_replaced_file_and_parent_identities_are_stale() {
        for parent in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let dir = root.path().join("templates");
            fs::create_dir(&dir).unwrap();
            let path = dir.join("template.json");
            fs::write(&path, "malformed retained bytes").unwrap();
            let source = capture(&path, None).unwrap();
            let review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
            if parent {
                fs::rename(&dir, root.path().join("old")).unwrap();
                fs::create_dir(&dir).unwrap();
            } else {
                fs::rename(&path, dir.join("old.json")).unwrap();
            }
            fs::write(&path, "malformed retained bytes").unwrap();
            assert!(execute(&review, &mut || panic!("stale source")).is_err());
            assert!(
                prepare(&source, TemplateAction::Reset, |_| panic!(
                    "reset uses no tool"
                ))
                .is_err()
            );
        }
    }

    #[test]
    fn replacement_failure_retains_original_and_cleans_stage() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        fs::write(&path, "original bytes").unwrap();
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
        assert!(
            execute_with(&review, &mut || Ok(()), None, |_, _, _| anyhow::bail!(
                "replacement failure"
            ))
            .is_err()
        );
        assert_eq!(capture(&path, None).unwrap(), source);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn reset_repairs_malformed_input_without_validation_and_preserves_siblings() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        fs::write(&path, "malformed").unwrap();
        fs::write(root.path().join("config.toml"), "keep").unwrap();
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Reset, |_| {
            panic!("reset uses no tools")
        })
        .unwrap();
        assert!(execute(&review, &mut || Ok(())).unwrap().original.is_none());
        assert_eq!(
            fs::read_to_string(root.path().join("config.toml")).unwrap(),
            "keep"
        );
    }

    #[test]
    fn review_tampering_and_oversized_input_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        fs::write(&path, "source").unwrap();
        assert!(capture(&path, Some(5)).is_err());
        let source = capture(&path, Some(6)).unwrap();
        let mut review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
        review.replacement = Some("arbitrary output".into());
        assert!(execute(&review, &mut || panic!("tampering")).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "source");
    }

    #[test]
    fn readonly_source_permissions_are_not_bypassed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("template.json");
        fs::write(&path, "original").unwrap();
        let source = capture(&path, None).unwrap();
        let review = prepare(&source, TemplateAction::Save(changed()), |_| Ok(())).unwrap();
        let permissions = fs::metadata(&path).unwrap().permissions();
        let mut readonly = permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(&path, readonly).unwrap();
        let result = execute(&review, &mut || Ok(()));
        fs::set_permissions(&path, permissions).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn junction_parent_is_refused_without_touching_its_target() {
        use std::os::windows::process::CommandExt;
        let root = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let link = root.path().join("templates");
        let output = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(target.path())
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = capture(&link.join("template.json"), None);
        fs::remove_dir(&link).unwrap();
        assert!(result.is_err());
        assert_eq!(fs::read_dir(target.path()).unwrap().count(), 0);
    }
}
