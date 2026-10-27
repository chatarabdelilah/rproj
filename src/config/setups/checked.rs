//! One checked executor for the terminal and the private desktop worker.
use super::{setup_path, unlinked};
use crate::path_identity::identity;
use anyhow::{Context, Result, ensure};
use rproj_core::saved_setups::*;
use std::{fs, path::Path};

pub(crate) fn capture(root: &Path, name: &str) -> Result<CapturedSetup> {
    unlinked(root)?;
    let root = root
        .canonicalize()
        .context("Cannot read saved setup storage")?;
    let storage_identity = identity(&root, true)?.context("Saved setup storage disappeared")?;
    let path = setup_path(&root, name)?;
    let source_identity = identity(&path, false)?.context("Saved setup disappeared")?;
    let bytes = fs::read(&path).with_context(|| format!("Cannot read {}", path.display()))?;
    ensure!(
        identity(&root, true)?.as_ref() == Some(&storage_identity)
            && identity(&path, false)?.as_ref() == Some(&source_identity),
        "Saved setup changed while reading; Reload from file"
    );
    Ok(CapturedSetup {
        root,
        name: name.into(),
        storage_identity,
        source_identity,
        bytes,
    })
}

pub(crate) fn verify(source: &CapturedSetup) -> Result<()> {
    ensure!(
        capture(&source.root, &source.name)? == *source,
        "Saved setup or storage changed; refresh with Reload from file before retrying"
    );
    Ok(())
}

fn destination(source: &CapturedSetup, name: &str) -> Result<std::path::PathBuf> {
    let path = setup_path(&source.root, name)?;
    let folded = format!("{name}.toml").to_lowercase();
    for entry in fs::read_dir(&source.root)? {
        ensure!(
            entry?.file_name().to_string_lossy().to_lowercase() != folded,
            "A setup with this name already exists; choose another name."
        );
    }
    ensure!(
        identity(&path, false)?.is_none(),
        "Destination appeared; review again"
    );
    Ok(path)
}

pub(crate) fn prepare(source: &CapturedSetup, action: SetupAction) -> Result<ManagementReview> {
    verify(source)?;
    let (document, target) = match &action {
        SetupAction::Save(graph) => (Some(DocumentReview::prepare(&source.bytes, graph)?), None),
        SetupAction::Rename(name) if name == &source.name => {
            (None, Some(setup_path(&source.root, name)?))
        }
        SetupAction::Rename(name) | SetupAction::Duplicate(name) => {
            (None, Some(destination(source, name)?))
        }
        SetupAction::Delete => (None, None),
    };
    let review = ManagementReview {
        source: source.clone(),
        action,
        document,
        destination: target,
    };
    review.validate()?;
    verify(source)?;
    Ok(review)
}

pub(crate) fn execute(
    review: &ManagementReview,
    progress: &mut impl FnMut(MutationState) -> Result<()>,
) -> ManagementOutcome {
    let mut outcome = ManagementOutcome {
        status: ManagementStatus::Failed,
        mutation: MutationState::NotStarted,
        source: review
            .source
            .root
            .join(format!("{}.toml", review.source.name)),
        destination: review.destination.clone(),
        saved: None,
        message: None,
    };
    let result = execute_inner(review, &mut outcome, progress);
    match result {
        Ok(()) => outcome.status = ManagementStatus::Succeeded,
        Err(error) => {
            outcome.status = match outcome.mutation {
                MutationState::NotStarted => ManagementStatus::Failed,
                MutationState::Copied if matches!(review.action, SetupAction::Rename(_)) => {
                    ManagementStatus::Partial
                }
                _ => ManagementStatus::Uncertain,
            };
            outcome.saved = None;
            outcome.message = Some(if outcome.status == ManagementStatus::Partial {
                format!(
                    "Copy saved at {}, but source could not be removed. Both copies may remain; refresh and inspect them. {error:#}",
                    review.destination.as_ref().unwrap().display()
                )
            } else {
                format!("{error:#}")
            });
        }
    }
    outcome
}

fn execute_inner(
    review: &ManagementReview,
    outcome: &mut ManagementOutcome,
    progress: &mut impl FnMut(MutationState) -> Result<()>,
) -> Result<()> {
    review.validate()?;
    ensure!(
        prepare(&review.source, review.action.clone())? == *review,
        "Reviewed setup action changed; review again"
    );
    let source = &review.source;
    if matches!(&review.action, SetupAction::Rename(name) if name == &source.name)
        || matches!(&review.action, SetupAction::Save(_))
            && review.document.as_ref().unwrap().replacement.is_none()
    {
        outcome.saved = Some(source.clone());
        return Ok(());
    }
    outcome.mutation = MutationState::MayHaveBegun;
    progress(outcome.mutation)?;
    // The callback may persist a worker report or allow a concurrent writer to run.
    ensure!(
        prepare(source, review.action.clone())? == *review,
        "Setup changed before staging; Reload from file"
    );
    match &review.action {
        SetupAction::Save(_) => {
            let contents = review
                .document
                .as_ref()
                .unwrap()
                .replacement
                .as_ref()
                .unwrap();
            let pending = crate::file_replace::stage(&outcome.source, contents.as_bytes(), true)?;
            verify(source)?;
            pending
                .persist(&outcome.source)
                .context("Could not replace saved setup")?;
            outcome.mutation = MutationState::Completed;
            let saved = capture(&source.root, &source.name)?;
            ensure!(
                saved.storage_identity == source.storage_identity
                    && saved.bytes == contents.as_bytes(),
                "Saved setup changed before confirmation; Reload from file"
            );
            outcome.saved = Some(saved);
        }
        SetupAction::Rename(name) | SetupAction::Duplicate(name) => {
            let target = review.destination.as_ref().unwrap();
            let pending = crate::file_replace::stage(target, &source.bytes, false)?;
            verify(source)?;
            ensure!(
                destination(source, name)? == *target,
                "Destination changed before copying"
            );
            pending.persist_noclobber(target).context(
                "Destination already exists or cannot be created; no setup was replaced",
            )?;
            outcome.mutation = MutationState::Copied;
            progress(outcome.mutation)?;
            let saved = capture(&source.root, name)?;
            ensure!(
                saved.storage_identity == source.storage_identity && saved.bytes == source.bytes,
                "Destination changed after copying; inspect both paths"
            );
            if matches!(review.action, SetupAction::Rename(_)) {
                verify(source)?;
                verify(&saved)?;
                fs::remove_file(&outcome.source).context("Could not remove rename source")?;
            }
            outcome.mutation = MutationState::Completed;
            outcome.saved = Some(saved);
        }
        SetupAction::Delete => {
            verify(source)?;
            fs::remove_file(&outcome.source).context("Could not delete setup")?;
            outcome.mutation = MutationState::Completed;
            ensure!(
                identity(&outcome.source, false)?.is_none(),
                "Setup reappeared after deletion; Reload from file"
            );
        }
    }
    Ok(())
}

pub(super) fn completed(outcome: ManagementOutcome) -> Result<ManagementOutcome> {
    ensure!(
        outcome.status == ManagementStatus::Succeeded,
        "{}",
        outcome
            .message
            .as_deref()
            .unwrap_or("Saved setup outcome is uncertain; Reload from file")
    );
    Ok(outcome)
}

#[cfg(test)]
mod tests;
