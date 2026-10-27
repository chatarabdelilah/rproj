use super::*;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("setups 世界");
    let config = temp.path().join("config.toml");
    (temp, root, config)
}
fn call(root: &Path, config: &Path, action: SavedAction) -> SavedReport {
    let ipc = tempfile::tempdir().unwrap();
    let request = SavedRequest {
        protocol: SAVED_SETUPS_PROTOCOL,
        id: "fixture".into(),
        action,
    };
    write_atomic(&ipc.path().join("request.json"), &request).unwrap();
    let result = job_at(ipc.path(), root, config);
    let report: SavedReport = read_json(&ipc.path().join("report.json")).unwrap();
    report.check(&request).unwrap();
    assert_eq!(
        result.is_ok(),
        report.status == Some(ManagementStatus::Succeeded)
    );
    report
}
#[test]
fn browsing_missing_storage_and_machine_context_creates_nothing() {
    let (_temp, root, config) = fixture();
    let report = call(&root, &config, SavedAction::Browse);
    let listing = report.listing.unwrap();
    assert!(listing.entries.is_empty());
    assert!(listing.environment.is_none());
    assert!(listing.environment_problem.is_some());
    assert!(!root.exists());
    assert!(!config.exists());
}
#[test]
fn malformed_documents_remain_manageable_and_reuse_is_readonly() {
    let (_temp, root, config) = fixture();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Broken.toml"), b"invalid [ \xff").unwrap();
    fs::write(
        root.join("Known.toml"),
        b"package_workflow='none'\ncustom=1\n",
    )
    .unwrap();
    let listing = call(&root, &config, SavedAction::Browse).listing.unwrap();
    assert_eq!(listing.entries.len(), 2);
    assert!(listing.entries[0].warning.is_some());
    let source = call(&root, &config, SavedAction::Load("Broken".into()))
        .loaded
        .unwrap();
    assert!(source.graph.is_none());
    assert!(source.edit_problem.is_some());
    let prepared = call(
        &root,
        &config,
        SavedAction::Prepare {
            source: source.source.clone(),
            action: SetupAction::Duplicate("Copy".into()),
        },
    )
    .prepared
    .unwrap();
    assert!(!root.join("Copy.toml").exists());
    let outcome = call(
        &root,
        &config,
        SavedAction::Execute(Box::new(prepared.review)),
    )
    .outcome
    .unwrap();
    assert_eq!(outcome.saved.unwrap().bytes, source.source.bytes);
    let source = call(&root, &config, SavedAction::Load("Known".into()))
        .loaded
        .unwrap()
        .source;
    let reused = call(&root, &config, SavedAction::Reuse(source.clone()))
        .reused
        .unwrap();
    assert_eq!(reused.source, source);
    assert_eq!(fs::read(root.join("Known.toml")).unwrap(), source.bytes);
    assert!(!config.exists());
}
#[test]
fn stale_and_foreign_storage_are_refused_before_mutation() {
    let (_temp, root, config) = fixture();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Source.toml"), "package_workflow='none'\n").unwrap();
    let source = checked::capture(&root, "Source").unwrap();
    let review = checked::prepare(&source, SetupAction::Delete).unwrap();
    let other = tempfile::tempdir().unwrap();
    let report = call(
        other.path(),
        &config,
        SavedAction::Execute(Box::new(review.clone())),
    );
    assert_eq!(report.mutation, MutationState::NotStarted);
    fs::write(root.join("Source.toml"), "changed=true").unwrap();
    let report = call(&root, &config, SavedAction::Execute(Box::new(review)));
    assert_eq!(report.mutation, MutationState::NotStarted);
    assert!(root.join("Source.toml").exists());
}
#[test]
fn oversized_documents_are_visible_but_not_loaded_or_mutated() {
    let (_temp, root, config) = fixture();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Huge.toml"), vec![b' '; IPC_LIMIT as usize + 1]).unwrap();
    let listing = call(&root, &config, SavedAction::Browse).listing.unwrap();
    assert!(
        listing.entries[0]
            .warning
            .as_ref()
            .unwrap()
            .contains("2 MiB")
    );
    let report = call(&root, &config, SavedAction::Load("Huge".into()));
    assert_eq!(report.status, Some(ManagementStatus::Failed));
    assert!(report.loaded.is_none());
    assert_eq!(
        fs::metadata(root.join("Huge.toml")).unwrap().len(),
        IPC_LIMIT + 1
    );
}

#[cfg(windows)]
mod owned;

#[test]
#[cfg(windows)]
fn watch_ownership_does_not_block_or_change_during_setup_management() {
    let (_temp, root, config) = fixture();
    fs::create_dir(&root).unwrap();
    fs::write(root.join("Example.toml"), "package_workflow='none'\n").unwrap();
    let source = checked::capture(&root, "Example").unwrap();
    let review = checked::prepare(&source, SetupAction::Duplicate("Copy".into())).unwrap();
    let project = tempfile::tempdir().unwrap();
    let project = project.path().canonicalize().unwrap();
    crate::background_watch::with_unresponsive_fixture(&project, || {
        let before = crate::background_watch::structured_status().unwrap();
        let report = call(&root, &config, SavedAction::Execute(Box::new(review)));
        assert_eq!(report.status, Some(ManagementStatus::Succeeded));
        let after = crate::background_watch::structured_status().unwrap();
        assert_eq!(
            serde_json::to_value(after).unwrap(),
            serde_json::to_value(before).unwrap()
        );
    });
}
