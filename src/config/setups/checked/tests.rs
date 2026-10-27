use super::*;

fn fixture() -> (tempfile::TempDir, CapturedSetup) {
    let directory = tempfile::tempdir().unwrap();
    let root = directory
        .path()
        .canonicalize()
        .unwrap()
        .join("saved 日本語 setups");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("sample.toml"),
        include_bytes!("../fixtures/metadata.toml"),
    )
    .unwrap();
    let captured = capture(&root, "sample").unwrap();
    (directory, captured)
}

fn save(source: &CapturedSetup) -> ManagementReview {
    let mut graph = decode(&source.bytes).graph.unwrap();
    graph.capabilities.remove("format");
    prepare(source, SetupAction::Save(graph)).unwrap()
}

#[test]
fn preparation_is_read_only_and_success_returns_exact_saved_baseline() {
    let (_directory, source) = fixture();
    let review = save(&source);
    assert_eq!(fs::read_dir(&source.root).unwrap().count(), 1);
    assert_eq!(capture(&source.root, "sample").unwrap(), source);
    let mut states = vec![];
    let result = execute(&review, &mut |s| {
        states.push(s);
        Ok(())
    });
    assert_eq!(result.status, ManagementStatus::Succeeded);
    assert_eq!(states, [MutationState::MayHaveBegun]);
    let saved = result.saved.unwrap();
    assert_eq!(
        saved.bytes,
        review.document.unwrap().replacement.unwrap().as_bytes()
    );
    assert_eq!(capture(&source.root, "sample").unwrap(), saved);
    assert_eq!(
        decode(&saved.bytes).parsed.unwrap()["future"],
        decode(&source.bytes).parsed.unwrap()["future"]
    );
}

#[test]
fn stale_source_and_forged_destination_or_contents_fail_before_staging() {
    let (_directory, source) = fixture();
    let mut forged = save(&source);
    forged.document.as_mut().unwrap().replacement = Some("tampered = true".into());
    let result = execute(&forged, &mut |_| panic!("no mutation"));
    assert_eq!(result.status, ManagementStatus::Failed);
    assert_eq!(result.mutation, MutationState::NotStarted);
    let mut forged = prepare(&source, SetupAction::Duplicate("copy".into())).unwrap();
    forged.destination = Some(source.root.join("wrong.toml"));
    assert_eq!(
        execute(&forged, &mut |_| panic!("no mutation")).status,
        ManagementStatus::Failed
    );
    let review = save(&source);
    fs::write(source.root.join("sample.toml"), "external = true").unwrap();
    assert_eq!(
        execute(&review, &mut |_| panic!("no mutation")).status,
        ManagementStatus::Failed
    );
    assert_eq!(fs::read_dir(&source.root).unwrap().count(), 1);
}

#[test]
fn identical_bytes_in_replaced_source_or_storage_require_reload() {
    let (_directory, source) = fixture();
    let review = save(&source);
    fs::rename(
        source.root.join("sample.toml"),
        source.root.join("old.toml"),
    )
    .unwrap();
    fs::write(source.root.join("sample.toml"), &source.bytes).unwrap();
    assert_eq!(
        execute(&review, &mut |_| panic!("no mutation")).status,
        ManagementStatus::Failed
    );
    let fresh = capture(&source.root, "sample").unwrap();
    let review = save(&fresh);
    fs::rename(&source.root, source.root.with_file_name("old storage")).unwrap();
    fs::create_dir(&source.root).unwrap();
    fs::write(source.root.join("sample.toml"), &source.bytes).unwrap();
    assert_eq!(
        execute(&review, &mut |_| panic!("no mutation")).status,
        ManagementStatus::Failed
    );
}

#[test]
fn destination_collision_after_review_never_overwrites() {
    let (_directory, source) = fixture();
    let review = prepare(&source, SetupAction::Duplicate("copy".into())).unwrap();
    let result = execute(&review, &mut |_| {
        fs::write(source.root.join("COPY.toml"), b"other writer")?;
        Ok(())
    });
    assert_ne!(result.status, ManagementStatus::Succeeded);
    assert_eq!(
        fs::read(source.root.join("COPY.toml")).unwrap(),
        b"other writer"
    );
    assert_eq!(
        fs::read(source.root.join("sample.toml")).unwrap(),
        source.bytes
    );
}

#[test]
fn rename_reports_both_paths_when_source_changes_after_copy() {
    let (_directory, source) = fixture();
    let review = prepare(&source, SetupAction::Rename("renamed 日本語".into())).unwrap();
    let result = execute(&review, &mut |state| {
        if state == MutationState::Copied {
            fs::write(source.root.join("sample.toml"), b"external = true")?;
        }
        Ok(())
    });
    assert_eq!(result.status, ManagementStatus::Partial);
    assert_eq!(result.mutation, MutationState::Copied);
    assert!(result.message.unwrap().contains("Both copies"));
    assert_eq!(fs::read(result.destination.unwrap()).unwrap(), source.bytes);
    assert_eq!(fs::read(result.source).unwrap(), b"external = true");
}

#[test]
fn report_callback_failure_stops_before_writes_or_source_removal() {
    let (_directory, source) = fixture();
    let review = save(&source);
    let result = execute(&review, &mut |_| anyhow::bail!("report unavailable"));
    assert_ne!(result.status, ManagementStatus::Succeeded);
    assert_eq!(
        fs::read(source.root.join("sample.toml")).unwrap(),
        source.bytes
    );
    let review = prepare(&source, SetupAction::Rename("copy".into())).unwrap();
    let result = execute(&review, &mut |state| {
        ensure!(state != MutationState::Copied, "second report unavailable");
        Ok(())
    });
    assert_eq!(result.status, ManagementStatus::Partial);
    assert_eq!(
        fs::read(source.root.join("sample.toml")).unwrap(),
        source.bytes
    );
    assert_eq!(
        fs::read(source.root.join("copy.toml")).unwrap(),
        source.bytes
    );
}

#[test]
fn malformed_documents_support_checked_management_but_not_save_or_reuse() {
    let (_directory, source) = fixture();
    fs::write(source.root.join("sample.toml"), b"bad [ toml\xff").unwrap();
    let source = capture(&source.root, "sample").unwrap();
    assert!(reusable_graph(&source.bytes).is_err());
    assert!(prepare(&source, SetupAction::Save(Default::default())).is_err());
    let copied = execute(
        &prepare(&source, SetupAction::Duplicate("copy".into())).unwrap(),
        &mut |_| Ok(()),
    );
    assert_eq!(copied.status, ManagementStatus::Succeeded);
    assert_eq!(copied.saved.unwrap().bytes, source.bytes);
    let renamed = execute(
        &prepare(&source, SetupAction::Rename("renamed".into())).unwrap(),
        &mut |_| Ok(()),
    );
    assert_eq!(renamed.status, ManagementStatus::Succeeded);
    assert!(!source.root.join("sample.toml").exists());
    let renamed = renamed.saved.unwrap();
    let deleted = execute(
        &prepare(&renamed, SetupAction::Delete).unwrap(),
        &mut |_| Ok(()),
    );
    assert_eq!(deleted.status, ManagementStatus::Succeeded);
    assert!(!renamed.root.join("renamed.toml").exists());
}

#[test]
fn no_op_save_and_rename_do_not_report_mutation() {
    let (_directory, source) = fixture();
    let graph = decode(&source.bytes).graph.unwrap();
    for action in [
        SetupAction::Save(graph),
        SetupAction::Rename("sample".into()),
    ] {
        let result = execute(&prepare(&source, action).unwrap(), &mut |_| {
            panic!("no mutation")
        });
        assert_eq!(result.status, ManagementStatus::Succeeded);
        assert_eq!(result.mutation, MutationState::NotStarted);
        assert_eq!(result.saved.unwrap(), source);
    }
    assert_eq!(fs::read_dir(&source.root).unwrap().count(), 1);
}

#[test]
fn saving_does_not_bypass_readonly_source_permissions() {
    let (_directory, source) = fixture();
    let review = save(&source);
    let path = source.root.join("sample.toml");
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original_permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let result = execute(&review, &mut |_| Ok(()));
    fs::set_permissions(&path, original_permissions).unwrap();
    assert_ne!(result.status, ManagementStatus::Succeeded);
    assert_eq!(fs::read(path).unwrap(), source.bytes);
    assert_eq!(fs::read_dir(&source.root).unwrap().count(), 1);
}
