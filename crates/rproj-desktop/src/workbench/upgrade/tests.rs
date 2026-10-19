use super::{
    Preparation, UpgradeOutcome, UpgradePreparation, UpgradeReview, UpgradeSession, UpgradeStatus,
    WriteState,
};
use crate::workbench::WatchStatus;
use rproj_core::upgrade::{SkippedFile, UpgradeFile};
use std::path::PathBuf;

pub(super) fn review(project: PathBuf) -> UpgradeReview {
    UpgradeReview {
        project,
        identity: "fixture".into(),
        originals: [
            (".luaurc".into(), Some("old\r\n".into())),
            ("tests/.luaurc".into(), Some("// custom".into())),
        ]
        .into(),
        files: vec![UpgradeFile {
            relative: ".luaurc".into(),
            contents: "new\t世界\n".into(),
            reason: "Managed strict mode; custom settings kept".into(),
            creating: false,
        }],
        skipped: vec![SkippedFile {
            relative: "tests/.luaurc".into(),
            reason: "Comments could not be safely merged".into(),
        }],
    }
}

#[test]
fn captured_review_and_result_survive_browsing_until_explicit_refresh() {
    let project = std::env::temp_dir();
    let mut state = UpgradeSession::default();
    state.begin(project.clone());
    state.loading = false;
    let Preparation::Prepared(prepared) = Preparation::from_worker(UpgradePreparation::Prepared(
        Box::new(review(project.clone())),
    )) else {
        panic!()
    };
    state.prepared = Some(*prepared);
    let captured = state.ready().unwrap().clone();
    state.visible = false;
    assert_eq!(state.ready(), Some(&captured));
    state.result = Some(Ok(UpgradeOutcome {
        project: project.clone(),
        status: UpgradeStatus::Succeeded,
        writes: WriteState::Completed,
        message: None,
    }));
    assert!(state.ready().is_none());
    assert!(state.prepared.is_some());
    let revision = state.revision;
    state.begin(project);
    assert!(state.revision > revision && state.result.is_none() && state.prepared.is_none());
}

#[test]
fn diffs_keep_control_markers_positions_and_skips_separate() {
    let Preparation::Prepared(prepared) = Preparation::from_worker(UpgradePreparation::Prepared(
        Box::new(review(std::env::temp_dir())),
    )) else {
        panic!()
    };
    assert_eq!(prepared.documents.len(), 1);
    let rows = &prepared.documents[0];
    assert!(
        rows.iter()
            .any(|r| r.text == "-old [CRLF]" && r.old_line == Some(1))
    );
    assert!(
        rows.iter()
            .any(|r| r.text == "+new\\t世界" && r.new_line == Some(1))
    );
    assert_eq!(prepared.review.skipped.len(), 1);
}

#[test]
fn malformed_review_is_a_visible_failure_instead_of_an_empty_applicable_diff() {
    let mut malformed = review(std::env::temp_dir());
    malformed.originals.remove(".luaurc");
    assert!(matches!(
        Preparation::from_worker(UpgradePreparation::Prepared(Box::new(malformed))),
        Preparation::Failed(_)
    ));
}

#[test]
fn only_the_captured_watch_owner_blocks_apply() {
    let project = std::env::temp_dir();
    let mut state = UpgradeSession {
        project: Some(project.clone()),
        ..Default::default()
    };
    let mut watch = WatchStatus {
        supported: true,
        project: Some(project),
        state: rproj_core::operations::WatchState::Watching,
        message: "fixture".into(),
    };
    assert!(state.watch_conflict(Some(&watch)));
    watch.project.as_mut().unwrap().push("another");
    assert!(!state.watch_conflict(Some(&watch)));
    state.project = watch.project.clone();
    watch.state = rproj_core::operations::WatchState::Stopping;
    assert!(state.watch_conflict(Some(&watch)));
    watch.state = rproj_core::operations::WatchState::Stopped;
    assert!(!state.watch_conflict(Some(&watch)));
}
