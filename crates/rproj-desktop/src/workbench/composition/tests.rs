use super::{CompositionDraft, CompositionSession};
use crate::workbench::WatchStatus;
use std::path::PathBuf;

#[test]
fn dirty_choices_invalidate_review_and_keep_original_ownership() {
    let mut session = CompositionSession {
        project: Some(PathBuf::from("original")),
        draft: Some(CompositionDraft::new(Default::default())),
        ..Default::default()
    };
    session
        .draft
        .as_mut()
        .unwrap()
        .toggle_package("promise")
        .unwrap();
    session.changed();
    assert!(session.dirty);
    assert!(session.prepared.is_none());
    assert_eq!(session.project, Some(PathBuf::from("original")));
    session
        .draft
        .as_mut()
        .unwrap()
        .toggle_package("promise")
        .unwrap();
    session.changed();
    assert!(!session.dirty);
}

#[test]
fn reset_invalidates_context_events_and_watch_is_project_specific() {
    let mut session = CompositionSession::default();
    let old = session.begin_load(PathBuf::from("source"));
    session.reset();
    session.accept_context(old, Err("late context".into()));
    assert!(session.error.is_none());
    session.project = Some(PathBuf::from("source"));
    let mut watch = WatchStatus {
        supported: true,
        project: Some(PathBuf::from("other")),
        state: rproj_core::operations::WatchState::Watching,
        message: String::new(),
    };
    assert!(!session.watch_conflict(Some(&watch)));
    watch.project = session.project.clone();
    assert!(session.watch_conflict(Some(&watch)));
    watch.state = rproj_core::operations::WatchState::Stopped;
    assert!(!session.watch_conflict(Some(&watch)));
}
