use super::{Prepared, SettingsLoaded, SettingsReview, SettingsSession, tool_settings};
use crate::workbench::{Operation, WatchStatus};
use rproj_core::tool_configuration::{SettingsContext, SettingsDraft};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub(super) fn loaded(project: PathBuf, tool: &str) -> SettingsLoaded {
    let target = tool_settings::find(tool).unwrap();
    let mut identities = BTreeMap::from([
        ("".into(), Some("fixture-root".into())),
        (target.target.relative().into(), None),
    ]);
    if matches!(target.target, tool_settings::ConfigTarget::VsCodeSettings) {
        identities.insert(".vscode".into(), None);
    }
    SettingsLoaded {
        context: SettingsContext {
            project,
            tool: tool.into(),
            identities,
            original: None,
        },
        draft: SettingsDraft::new(vec![None; target.settings.len()]),
        blocked: BTreeMap::new(),
    }
}
pub(super) fn session(project: PathBuf, tool: &str) -> SettingsSession {
    SettingsSession {
        visible: true,
        project: Some(project.clone()),
        tool: tool.into(),
        loaded: Some(loaded(project, tool)),
        support: Some(Ok(())),
        ..Default::default()
    }
}
pub(super) fn review(state: &mut SettingsSession) {
    let loaded = state.loaded.as_ref().unwrap();
    let review = SettingsReview {
        context: loaded.context.clone(),
        draft: loaded.draft.clone(),
        contents: Some("{\"editor.formatOnSave\":true}\n".into()),
    };
    let rows = Arc::new(review.diff().unwrap());
    state.prepared = Some(Prepared { review, rows });
    state.reviewing = true;
}

#[test]
fn absent_defaults_are_explicit_and_reverting_does_not_stage_values() {
    let mut state = session(std::env::current_dir().unwrap(), "stylua-vscode");
    assert!(!state.dirty());
    assert!(state.ready().is_none());
    state.set(0, json!(true));
    review(&mut state);
    assert!(state.ready().is_some());
    state.set(1, json!(true));
    assert!(state.ready().is_none());
    let loaded = state.loaded.as_mut().unwrap();
    loaded.draft.revert(0);
    loaded.draft.revert(1);
    assert!(!state.dirty());
}

#[test]
fn custom_null_requires_consent_and_blocked_fields_never_accept_changes() {
    let mut state = session(std::env::current_dir().unwrap(), "stylua-vscode");
    state.loaded.as_mut().unwrap().draft.current[0] = Some(json!(null));
    state.set(0, json!(true));
    assert!(!state.dirty());
    state.consent.insert(0);
    state.set(0, json!(true));
    assert!(state.dirty());
    state
        .loaded
        .as_mut()
        .unwrap()
        .blocked
        .insert(1, "Writer removes this key".into());
    state.set(1, json!(false));
    assert!(state.loaded.as_ref().unwrap().draft.changes[1].is_none());
}

#[test]
fn loading_uncertainty_and_review_ownership_prevent_submission() {
    let mut state = session(std::env::current_dir().unwrap(), "stylua-vscode");
    state.set(0, json!(true));
    review(&mut state);
    state.loading = true;
    assert!(state.ready().is_none());
    state.loading = false;
    state.reload_required = true;
    state.set(1, json!(true));
    assert!(state.loaded.as_ref().unwrap().draft.changes[1].is_none());
    assert!(state.ready().is_none());
    state.reload_required = false;
    state.project = Some(std::env::temp_dir().join("another"));
    assert!(state.ready().is_none());
}

#[test]
fn save_is_not_cancellable_and_other_watch_ownership_is_not_a_conflict() {
    assert!(!Operation::SaveSettings.cancellable());
    assert!(Operation::PrepareSettings.cancellable());
    let state = session(std::env::current_dir().unwrap(), "stylua-vscode");
    let mut watch = WatchStatus {
        supported: true,
        project: state.project.clone(),
        state: rproj_core::operations::WatchState::Watching,
        message: String::new(),
    };
    assert!(state.watch_conflict(Some(&watch)));
    watch.project = Some(std::env::temp_dir());
    assert!(!state.watch_conflict(Some(&watch)));
}
