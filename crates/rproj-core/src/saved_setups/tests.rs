use super::*;

const SOURCE: &str = "# preserved until edited\r\nmode = 'like:original'\r\npackage_workflow = 'wally'\r\npackages = ['react', 'reactRoblox']\r\ndropped = ['future-file', '.gitignore']\r\nteam = '日本語'\r\n[capabilities]\r\ntest = 'testez'\r\nformat = 'stylua'\r\n[future]\r\ndate = 2026-10-27\r\ncount = 9223372036854775807";

#[test]
fn parse_diagnostics_do_not_expose_unrelated_document_values() {
    for bytes in [
        b"private-value='unterminated".as_slice(),
        b"packages = { secret = 'private-value' }",
    ] {
        let decoded = decode(bytes);
        assert!(!decoded.problem.unwrap().contains("private-value"));
    }
}

#[test]
fn no_op_and_revert_keep_original_bytes_with_unknown_metadata() {
    let graph = decode(SOURCE.as_bytes()).graph.unwrap();
    let mut draft = SavedSetupDraft::new(graph.clone()).unwrap();
    let review = DocumentReview::prepare(SOURCE.as_bytes(), &draft.graph).unwrap();
    assert!(review.replacement.is_none());
    assert_eq!(review.original, SOURCE.as_bytes());
    draft.set_capability("format", None).unwrap();
    assert_eq!(draft.graph.packages, graph.packages);
    assert_eq!(draft.graph.dropped, graph.dropped);
    draft.set_capability("format", Some("stylua")).unwrap();
    assert!(!draft.dirty());
    assert!(
        DocumentReview::prepare(SOURCE.as_bytes(), &draft.graph)
            .unwrap()
            .replacement
            .is_none()
    );
}

#[test]
fn changed_review_preserves_metadata_types_and_exposes_complete_diff() {
    let mut graph = decode(SOURCE.as_bytes()).graph.unwrap();
    graph.capabilities.remove("format");
    let review = DocumentReview::prepare(SOURCE.as_bytes(), &graph).unwrap();
    let saved = decode(review.replacement.as_ref().unwrap().as_bytes());
    let original = decode(SOURCE.as_bytes()).parsed.unwrap();
    for key in ["mode", "dropped", "future", "team", "packages"] {
        assert_eq!(saved.parsed.as_ref().unwrap()[key], original[key]);
    }
    assert_eq!(
        saved.parsed.as_ref().unwrap()["future"]["date"].type_str(),
        "datetime"
    );
    let rows = review.diff("sample.toml").unwrap();
    assert!(rows.iter().any(|r| r.text.contains("[CRLF]")));
    assert!(rows.iter().any(|r| r.text.contains("[no newline]")));
    assert!(rows.iter().any(|r| r.kind == diff::Kind::Added));
    let mut forged = review;
    forged.replacement = Some("arbitrary = true".into());
    assert!(forged.validate().is_err());
}

#[test]
fn reuse_allows_metadata_but_rejects_unknown_composition_or_exclusions() {
    assert!(reusable_graph(SOURCE.as_bytes()).is_err());
    let supported = SOURCE.replace("'future-file', ", "");
    assert!(reusable_graph(supported.as_bytes()).is_ok());
    for replacement in [
        supported.replace("'react'", "'future-package'"),
        supported.replace("testez", "future-test"),
        supported.replace("'wally'", "'unknown-workflow'"),
        supported.replace("'.gitignore'", "'src'"),
    ] {
        assert!(reusable_graph(replacement.as_bytes()).is_err());
    }
    assert!(SavedSetupDraft::new(decode(SOURCE.as_bytes()).graph.unwrap()).is_ok());
}

#[test]
fn malformed_bytes_and_unknown_choices_remain_visible_without_normalization() {
    let bad = decode(b"not [ toml\xff");
    assert!(bad.graph.is_none() && bad.problem.is_some());
    let unknown = SOURCE.replace("'react'", "'future-package'");
    let graph = decode(unknown.as_bytes()).graph.unwrap();
    assert!(unsupported(&graph));
    assert!(SavedSetupDraft::new(graph.clone()).is_err());
    assert!(
        DocumentReview::prepare(unknown.as_bytes(), &graph)
            .unwrap()
            .replacement
            .is_none()
    );
    let mut changed = graph;
    changed.packages.clear();
    assert!(DocumentReview::prepare(unknown.as_bytes(), &changed).is_err());
}

#[test]
fn workflow_testing_companions_and_required_files_have_explicit_rules() {
    let mut graph = decode(SOURCE.as_bytes()).graph.unwrap();
    graph
        .capabilities
        .insert("test".into(), "jest-roblox-open-cloud".into());
    let mut draft = SavedSetupDraft::new(graph.clone()).unwrap();
    draft.set_workflow(PackageWorkflow::None).unwrap();
    assert!(draft.validate().is_err());
    assert_eq!(draft.graph.capabilities["test"], "jest-roblox-open-cloud");
    assert_eq!(draft.graph.dropped, graph.dropped);
    draft.set_capability("test", Some("testez")).unwrap();
    draft.validate().unwrap();
    draft.set_workflow(PackageWorkflow::Wally).unwrap();
    draft.toggle_package("react", true).unwrap();
    assert!(draft.graph.packages.contains(&"reactRoblox".into()));
    let env = CreationEnvironment {
        projects_root: PathBuf::new(),
        apps: vec![],
        extensions: vec![],
    };
    assert!(draft.toggle_file("src", &env).is_err());
    assert!(draft.toggle_file("tests", &env).is_err());
    draft.toggle_file("rproj.toml", &env).unwrap();
    assert!(draft.graph.dropped.contains(&"future-file".into()));
    draft.validate().unwrap();
}

#[test]
fn unknown_new_choices_and_provenance_changes_cannot_be_saved() {
    let original = decode(SOURCE.as_bytes()).graph.unwrap();
    for change in [0, 1, 2] {
        let mut graph = original.clone();
        match change {
            0 => graph.mode = "expert".into(),
            1 => graph.packages.push("future".into()),
            _ => graph.dropped.push("src".into()),
        }
        assert!(validate_changes(&original, &graph).is_err());
    }
}
