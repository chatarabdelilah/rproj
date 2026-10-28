use super::{Prepared, SavedSession, Task, path};
use rproj_core::{
    creation::PackageWorkflow,
    saved_setups::{protocol::*, *},
};
use std::{path::PathBuf, sync::Arc};

pub(super) fn loaded(root: PathBuf, name: &str) -> LoadedSetup {
    LoadedSetup::new(CapturedSetup { root, name:name.into(),storage_identity:"storage".into(),source_identity:format!("file-{name}"),bytes:b"# preserved\nmode='like:team'\npackage_workflow='wally'\npackages=['react']\ndropped=['future-file']\n[metadata]\nowner='team'\n".to_vec() })
}
pub(super) fn session(root: PathBuf) -> SavedSession {
    let loaded = loaded(root.clone(), "Example");
    SavedSession {
        visible: true,
        support: Some(Ok(())),
        listing: Some(SetupListing {
            root: root.clone(),
            entries: vec![SetupEntry {
                name: "Example".into(),
                path: path(&loaded.source),
                summary: "Wally composition".into(),
                warning: loaded.reuse_problem.clone(),
            }],
            environment: None,
            environment_problem: Some(
                "Machine context is missing; composition-only file suggestions".into(),
            ),
        }),
        selected: Some(loaded),
        ..Default::default()
    }
}
pub(super) fn prepare(session: &mut SavedSession) -> ManagementReview {
    let draft = session.draft.as_ref().unwrap();
    let source = draft.loaded.source.clone();
    let review = ManagementReview {
        document: Some(DocumentReview::prepare(&source.bytes, &draft.draft.graph).unwrap()),
        source,
        action: SetupAction::Save(draft.draft.graph.clone()),
        destination: None,
    };
    let prepared = PreparedSetup::new(review.clone()).unwrap();
    session.prepared = Some(Prepared {
        rows: Arc::new(prepared.rows().unwrap()),
        review: review.clone(),
    });
    review
}

#[test]
fn editing_preserves_ownership_metadata_exclusions_and_noop_bytes() {
    let mut s = session(std::env::current_dir().unwrap());
    let original = s.selected.clone().unwrap();
    s.begin_edit(original.clone()).unwrap();
    let review = prepare(&mut s);
    assert!(review.document.unwrap().replacement.is_none());
    assert!(s.ready().is_none());
    s.edit(|d| d.set_capability("format", Some("stylua")));
    assert!(s.dirty());
    s.selected = Some(loaded(original.source.root.join("other"), "Other"));
    assert_eq!(s.name(), "Example");
    let review = prepare(&mut s);
    assert_eq!(review.source, original.source);
    assert!(
        review
            .document
            .unwrap()
            .replacement
            .unwrap()
            .contains("[metadata]")
    );
    s.edit(|d| d.set_capability("format", None));
    assert!(!s.dirty());
    assert!(prepare(&mut s).document.unwrap().replacement.is_none());
}
#[test]
fn required_files_and_explicit_testing_repair_use_shared_rules() {
    let mut s = session(std::env::current_dir().unwrap());
    s.begin_edit(s.selected.clone().unwrap()).unwrap();
    let env = s.environment();
    s.edit(|d| d.toggle_file("src", &env));
    assert!(s.error.is_some());
    assert!(!s.dirty());
    s.edit(|d| d.set_capability("test", Some("jest-roblox")));
    s.edit(|d| d.set_workflow(PackageWorkflow::None));
    assert!(s.draft.as_ref().unwrap().draft.validate().is_err());
    assert_eq!(
        s.draft.as_ref().unwrap().draft.graph.capabilities["test"],
        "jest-roblox"
    );
    s.edit(|d| d.set_capability("test", Some("testez")));
    assert!(s.draft.as_ref().unwrap().draft.validate().is_ok());
    assert!(
        s.draft
            .as_ref()
            .unwrap()
            .draft
            .graph
            .dropped
            .contains(&"future-file".into())
    );
}
#[test]
fn companion_addition_is_explicit_and_preserves_recorded_packages() {
    let mut s = session(std::env::current_dir().unwrap());
    s.begin_edit(s.selected.clone().unwrap()).unwrap();
    s.add_companions();
    let graph = &s.draft.as_ref().unwrap().draft.graph;
    assert!(graph.packages.contains(&"reactRoblox".into()));
    assert!(graph.packages.contains(&"react".into()));
    assert_eq!(graph.mode, "like:team");
    assert_eq!(graph.dropped, ["future-file"]);
}
#[test]
fn resuming_editor_cannot_apply_another_documents_management_review() {
    let mut s = session(std::env::current_dir().unwrap());
    s.begin_edit(s.selected.clone().unwrap()).unwrap();
    s.edit(|d| d.set_capability("format", Some("stylua")));
    prepare(&mut s);
    assert!(s.ready().is_some());
    let source = loaded(std::env::current_dir().unwrap(), "Other").source;
    s.prepared = Some(Prepared {
        review: ManagementReview {
            source,
            action: SetupAction::Delete,
            document: None,
            destination: None,
        },
        rows: Arc::new(vec![]),
    });
    assert!(s.current_prepared().is_none());
    assert!(s.ready().is_none());
    assert_eq!(s.name(), "Example");
    s.editing = false;
    assert!(s.ready().is_some());
}
#[test]
fn pending_and_stale_operations_freeze_edits_and_require_reload() {
    let mut s = session(std::env::current_dir().unwrap());
    s.begin_edit(s.selected.clone().unwrap()).unwrap();
    s.edit(|d| d.set_capability("format", Some("stylua")));
    let review = prepare(&mut s);
    let graph = s.draft.as_ref().unwrap().draft.graph.clone();
    s.pending = Some(Task::Manage(Box::new(review.clone())));
    s.edit(|d| d.set_capability("format", None));
    assert_eq!(s.draft.as_ref().unwrap().draft.graph, graph);
    assert!(s.ready().is_none());
    s.pending = None;
    s.reload_sources.insert(path(&review.source));
    assert!(!s.editable());
    assert!(s.ready().is_none());
    s.reload_sources.remove(&path(&review.source));
    s.teardown_uncertain = true;
    assert!(!s.editable());
}
#[test]
fn unsupported_documents_disable_editing_but_keep_management_identity() {
    let mut loaded = loaded(std::env::current_dir().unwrap(), "Unknown");
    loaded.source.bytes = b"packages=['future']".to_vec();
    let loaded = LoadedSetup::new(loaded.source);
    assert!(loaded.edit_problem.is_some());
    assert!(loaded.reuse_problem.is_some());
    let mut s = SavedSession::default();
    assert!(s.begin_edit(loaded.clone()).is_err());
    let review = ManagementReview {
        source: loaded.source,
        action: SetupAction::Delete,
        document: None,
        destination: None,
    };
    review.validate().unwrap();
}
