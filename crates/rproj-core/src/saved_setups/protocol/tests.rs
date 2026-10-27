use super::*;

fn source() -> CapturedSetup {
    CapturedSetup {
        root: std::env::current_dir().unwrap(),
        name: "Example".into(),
        storage_identity: "root".into(),
        source_identity: "file".into(),
        bytes: b"# custom\npackage_workflow='none'\nteam='example'\n".to_vec(),
    }
}
fn request(action: SavedAction) -> SavedRequest {
    SavedRequest {
        protocol: SAVED_SETUPS_PROTOCOL,
        id: "test-operation".into(),
        action,
    }
}

#[test]
fn baseline_and_reuse_require_matching_document_bytes() {
    let loaded = LoadedSetup::new(source());
    loaded.validate().unwrap();
    assert!(loaded.reuse_problem.is_none());
    let mut tampered = loaded.clone();
    tampered.graph.as_mut().unwrap().mode = "forged".into();
    assert!(tampered.validate().is_err());
    let req = request(SavedAction::Reuse(source()));
    let mut report = SavedReport::initial(&req);
    report.status = Some(ManagementStatus::Succeeded);
    report.reused = Some(ReusedSetup {
        source: source(),
        graph: reusable_graph(&source().bytes).unwrap(),
    });
    report.check(&req).unwrap();
    report.reused.as_mut().unwrap().source.bytes.push(b' ');
    assert!(report.check(&req).is_err());
}

#[test]
fn mutation_reports_verify_identity_status_exact_bytes_and_noop() {
    let source = source();
    let review = ManagementReview {
        source: source.clone(),
        action: SetupAction::Duplicate("Copy".into()),
        document: None,
        destination: Some(source.root.join("Copy.toml")),
    };
    let req = request(SavedAction::Execute(Box::new(review.clone())));
    let mut report = SavedReport::initial(&req);
    let mut saved = source.clone();
    saved.name = "Copy".into();
    saved.source_identity = "copy".into();
    report.status = Some(ManagementStatus::Succeeded);
    report.mutation = MutationState::Completed;
    report.outcome = Some(ManagementOutcome {
        status: ManagementStatus::Succeeded,
        mutation: MutationState::Completed,
        source: source.root.join("Example.toml"),
        destination: review.destination.clone(),
        saved: Some(saved),
        message: None,
    });
    report.check(&req).unwrap();
    let original = report.clone();
    report
        .outcome
        .as_mut()
        .unwrap()
        .saved
        .as_mut()
        .unwrap()
        .bytes
        .push(b'!');
    assert!(report.check(&req).is_err());
    report = original.clone();
    report.id.push('x');
    assert!(report.check(&req).is_err());
    report = original.clone();
    report.mutation = MutationState::NotStarted;
    assert!(report.check(&req).is_err());
    report = original;
    report.status = Some(ManagementStatus::Failed);
    assert!(report.check(&req).is_err());
}

#[test]
fn diff_and_readonly_results_cannot_be_forged() {
    let source = source();
    let mut graph = reusable_graph(&source.bytes).unwrap();
    graph.capabilities.insert("format".into(), "stylua".into());
    let review = ManagementReview {
        document: Some(DocumentReview::prepare(&source.bytes, &graph).unwrap()),
        source: source.clone(),
        action: SetupAction::Save(graph.clone()),
        destination: None,
    };
    let mut prepared = PreparedSetup::new(review).unwrap();
    assert!(!prepared.rows().unwrap().is_empty());
    prepared.review.document.as_mut().unwrap().replacement = Some("forged".into());
    assert!(prepared.validate().is_err());
    let req = request(SavedAction::Load(source.name.clone()));
    let mut report = SavedReport::initial(&req);
    report.loaded = Some(LoadedSetup::new(source));
    report.status = Some(ManagementStatus::Succeeded);
    report.check(&req).unwrap();
    report.mutation = MutationState::MayHaveBegun;
    assert!(report.check(&req).is_err());
}
