use super::CreationSession;
use rproj_core::creation::{CreationDraft, CreationEnvironment, PackageWorkflow, protocol::*};
use std::path::PathBuf;

fn context() -> CreationContext {
    CreationContext {
        protocol: CREATION_PROTOCOL,
        environment: CreationEnvironment {
            projects_root: PathBuf::from("C:/fixtures"),
            apps: vec![],
            extensions: vec![],
        },
        machine_ready: true,
        template: None,
        template_problem: None,
        config_source: SourceSnapshot {
            bytes: Some(vec![1]),
        },
        template_source: SourceSnapshot { bytes: None },
        setups: vec![],
        setups_problem: None,
    }
}
#[test]
fn review_revisions_and_sources_cannot_follow_edits_or_recheck() {
    let mut state = CreationSession {
        draft: Some(CreationDraft::new("Example".into())),
        context: Some(context()),
        ..Default::default()
    };
    let request = state.request().unwrap();
    let prepared = PreparedCreation {
        plan: request
            .draft
            .plan(&state.context.as_ref().unwrap().environment)
            .unwrap(),
        sources: request.sources,
        template: None,
    };
    state.prepared = Some((state.revision, prepared));
    assert!(state.fresh_plan().is_some());
    state.context.as_mut().unwrap().config_source.bytes = Some(vec![2]);
    assert!(state.fresh_plan().is_none());
    state.changed();
    assert!(state.prepared.is_none());
    assert!(state.dirty);
}
#[test]
fn unsupported_saved_choices_are_visible_and_never_normalized() {
    let mut state = CreationSession {
        draft: Some(CreationDraft::new("Example".into())),
        context: Some(context()),
        ..Default::default()
    };
    let original = state.draft.clone();
    let mut graph = CreationDraft::new("irrelevant".into()).graph;
    graph.packages.push("future-package".into());
    state
        .context
        .as_mut()
        .unwrap()
        .setups
        .push(SavedComposition {
            name: "Future".into(),
            graph: Some(graph),
            problem: None,
            source: SourceSnapshot {
                bytes: Some(vec![3]),
            },
        });
    assert!(
        state
            .load_setup("Future")
            .unwrap_err()
            .contains("Unsupported package")
    );
    assert_eq!(state.draft, original);
    assert!(state.selected_setup.is_none());
    assert!(state.request().unwrap_err().contains("Unsupported package"));
}
#[test]
fn saved_source_changes_require_explicit_reload_and_preserve_concrete_choices() {
    let mut state = CreationSession {
        draft: Some(CreationDraft::new("Example".into())),
        context: Some(context()),
        ..Default::default()
    };
    let mut graph = CreationDraft::new("unused".into()).graph;
    graph
        .capabilities
        .insert("test".into(), "jest-roblox-open-cloud".into());
    state
        .context
        .as_mut()
        .unwrap()
        .setups
        .push(SavedComposition {
            name: "Saved".into(),
            graph: Some(graph),
            problem: None,
            source: SourceSnapshot {
                bytes: Some(vec![1]),
            },
        });
    state.load_setup("Saved").unwrap();
    assert_eq!(
        state.draft.as_ref().unwrap().graph.capabilities["test"],
        "jest-roblox-open-cloud"
    );
    state.context.as_mut().unwrap().setups[0].source.bytes = Some(vec![2]);
    assert!(state.request().unwrap_err().contains("changed"));
    state.load_setup("Saved").unwrap();
    assert!(state.request().is_ok());
}
#[test]
fn incompatible_testing_requires_a_decision_and_missing_setup_blocks_review() {
    let mut state = CreationSession {
        draft: Some(CreationDraft::new("Example".into())),
        context: Some(context()),
        ..Default::default()
    };
    let draft = state.draft.as_mut().unwrap();
    draft.set_capability("test", Some("jest-roblox")).unwrap();
    draft.set_workflow(PackageWorkflow::None);
    assert!(
        state.project_ready().is_ok(),
        "Back to Project must not trap a repairable composition"
    );
    assert!(state.request().unwrap_err().contains("compatible"));
    state
        .draft
        .as_mut()
        .unwrap()
        .set_capability("test", Some("testez"))
        .unwrap();
    assert!(state.request().is_ok());
    state.context.as_mut().unwrap().machine_ready = false;
    assert!(state.request().unwrap_err().contains("rproj setup"));
}

#[test]
fn guided_conversion_protects_primary_choices_and_uses_shared_companions() {
    let mut state = CreationSession {
        draft: Some(CreationDraft::new("Example".into())),
        context: Some(context()),
        ..Default::default()
    };
    state.select_mode("expert").unwrap();
    state
        .draft
        .as_mut()
        .unwrap()
        .toggle_package("react", false)
        .unwrap();
    state
        .draft
        .as_mut()
        .unwrap()
        .toggle_package("vide", false)
        .unwrap();
    let before = state.draft.clone();
    assert!(
        state
            .select_mode("guided")
            .unwrap_err()
            .contains("Remove the extra choices")
    );
    assert_eq!(state.draft, before);
    state
        .draft
        .as_mut()
        .unwrap()
        .toggle_package("vide", false)
        .unwrap();
    state
        .draft
        .as_mut()
        .unwrap()
        .graph
        .dropped
        .push(".gitignore".into());
    state.select_mode("guided").unwrap();
    let draft = state.draft.as_ref().unwrap();
    assert!(draft.graph.packages.iter().any(|p| p == "reactRoblox"));
    assert!(draft.graph.dropped.is_empty());
    state
        .draft
        .as_mut()
        .unwrap()
        .graph
        .packages
        .push("future-package".into());
    let before = state.draft.clone();
    assert!(
        state
            .select_mode("guided")
            .unwrap_err()
            .contains("Unsupported package")
    );
    assert_eq!(state.draft, before);
}
