use super::{
    Category, SETUP_PROTOCOL, Selection, SetupAction, SetupContext, SetupReport, SetupRequest,
    SetupReview, SetupSession, Status, Step, WatchStatus,
};
use std::path::PathBuf;

pub(super) fn session(root: PathBuf) -> SetupSession {
    let selection = Selection {
        root: root.join("projects"),
        apps: Default::default(),
        tools: Default::default(),
        plugins: Default::default(),
        extensions: Default::default(),
    };
    let context = SetupContext {
        config_path: root.join("machine.toml"),
        original: None,
        saved: selection.clone(),
        identities: vec![rproj_core::machine_setup::PathIdentity {
            path: root,
            directory: true,
            identity: Some("fixture".into()),
        }],
        detected: Default::default(),
    };
    SetupSession {
        visible: true,
        support: Some(Ok(())),
        context: Some(context),
        selection: Some(selection),
        ..Default::default()
    }
}
pub(super) fn prepare(state: &mut SetupSession) {
    state.prepared = Some(
        SetupReview::new(
            state.context.clone().unwrap(),
            state.selection.clone().unwrap(),
        )
        .unwrap(),
    );
    state.step = Step::Review;
}

#[test]
fn selections_preserve_unknown_and_inactive_choices_and_invalidate_review() {
    let mut state = session(std::env::temp_dir());
    state
        .context
        .as_mut()
        .unwrap()
        .saved
        .plugins
        .insert("future".into());
    state
        .selection
        .as_mut()
        .unwrap()
        .plugins
        .extend(["future".into(), "blender-plugin".into()]);
    state.toggle(Category::Blender, "blender-plugin");
    assert!(
        state
            .selection
            .as_ref()
            .unwrap()
            .plugins
            .contains("blender-plugin")
    );
    prepare(&mut state);
    state.toggle(Category::Apps, "blender");
    assert!(state.dirty() && state.prepared.is_none());
    state.toggle(Category::Blender, "blender-plugin");
    assert!(
        !state
            .selection
            .as_ref()
            .unwrap()
            .plugins
            .contains("blender-plugin")
    );
    assert!(state.selection.as_ref().unwrap().plugins.contains("future"));
    state.toggle(Category::Plugins, "future");
    assert!(state.selection.as_ref().unwrap().plugins.contains("future"));
}

#[test]
fn frozen_and_stale_drafts_cannot_be_submitted_or_changed() {
    let mut state = session(std::env::temp_dir());
    prepare(&mut state);
    assert!(state.ready().is_some());
    state.loading = true;
    state.toggle(Category::Apps, "git");
    assert!(!state.dirty());
    assert!(state.ready().is_none());
    state.loading = false;
    state.reload_required = true;
    state.toggle(Category::Apps, "git");
    assert!(!state.dirty());
    assert!(state.ready().is_none());
    state.reload_required = false;
    state.support = Some(Err("Matching distribution required".into()));
    assert!(state.ready().is_none());
    state.support = None;
    assert!(state.ready().is_none());
}

#[test]
fn any_active_or_unknown_watch_blocks_setup_regardless_of_project() {
    use rproj_core::operations::WatchState;
    for state in [
        WatchState::Preparing,
        WatchState::Watching,
        WatchState::Stopping,
        WatchState::Unknown,
        WatchState::Unresponsive,
    ] {
        assert!(SetupSession::watch_conflict(Some(&WatchStatus {
            supported: true,
            project: Some("other".into()),
            state,
            message: String::new()
        })));
    }
    assert!(!SetupSession::watch_conflict(Some(&WatchStatus {
        supported: true,
        project: None,
        state: WatchState::Stopped,
        message: String::new()
    })));
}

#[test]
fn monotonic_progress_retains_machine_identity_and_expands_only_new_failures() {
    let mut state = session(std::env::temp_dir());
    prepare(&mut state);
    state.loading = true;
    let request = SetupRequest {
        protocol: SETUP_PROTOCOL,
        id: "one".into(),
        action: SetupAction::Apply(state.prepared.clone().unwrap()),
    };
    let mut report = SetupReport::initial(&request);
    report.sequence = 1;
    assert!(!state.accept_progress(Box::new(report.clone())));
    report.sequence = 2;
    report.statuses[0] = Status::Failed;
    assert!(state.accept_progress(Box::new(report.clone())));
    assert!(!state.accept_progress(Box::new(report.clone())));
    report.id = "stale".into();
    report.sequence = 3;
    assert!(!state.accept_progress(Box::new(report)));
    assert_eq!(state.progress.as_ref().unwrap().sequence, 2);
}
