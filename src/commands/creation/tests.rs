use super::model::{Draft, Effect, Modal, Step, validate_name};
use crate::{config::PackageWorkflow, graph::ProjectGraph, tui::InputState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

fn draft() -> Draft {
    Draft::new("Example", vec!["saved".into()], vec![], vec![])
}
fn key(draft: &mut Draft, code: KeyCode) -> Effect {
    draft.key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn select(draft: &mut Draft, value: &str) -> Effect {
    draft.picker.query = InputState::new("");
    draft.picker.selected = draft
        .picker
        .items
        .iter()
        .position(|p| p.value == value)
        .unwrap_or_else(|| panic!("missing {value} in {:?}", draft.step));
    key(draft, KeyCode::Enter)
}
fn graph_value(draft: &Draft) -> serde_json::Value {
    serde_json::to_value(&draft.graph).unwrap()
}
fn review_graph(graph: ProjectGraph) -> Draft {
    let mut draft = draft();
    draft.loaded("saved", graph, vec![]);
    draft
}

#[test]
fn expert_filter_retains_checked_packages_and_matches_existing_graph() {
    for (workflow, strategy) in [
        (PackageWorkflow::Wally, "wally"),
        (PackageWorkflow::GitSubmodules, "git"),
    ] {
        let mut draft = draft();
        select(&mut draft, "expert");
        select(&mut draft, strategy);
        draft.paste("janitor");
        key(&mut draft, KeyCode::Char(' '));
        draft.picker.query = InputState::new("nothing-matches");
        key(&mut draft, KeyCode::Enter);
        assert_eq!(draft.step, Step::Capabilities);
        draft.checked.clear();
        key(&mut draft, KeyCode::Enter);
        assert_eq!(draft.step, Step::Review);
        let expected = ProjectGraph {
            mode: "expert".into(),
            package_workflow: workflow,
            packages: vec!["janitor".into()],
            ..Default::default()
        };
        assert_eq!(
            graph_value(&draft),
            serde_json::to_value(&expected).unwrap()
        );
        assert_eq!(draft.graph.plan(&[], &[]), expected.plan(&[], &[]));
    }
}

#[test]
fn guided_companions_match_the_direct_composition_and_can_be_removed() {
    let mut draft = draft();
    select(&mut draft, "guided");
    select(&mut draft, "wally");
    while matches!(draft.step, Step::Packages(_)) {
        if draft.picker.items.iter().any(|p| p.value == "react") {
            select(&mut draft, "react");
        } else {
            key(&mut draft, KeyCode::Enter);
        }
    }
    assert!(draft.graph.packages.iter().any(|p| p == "reactRoblox"));
    draft.checked.clear();
    key(&mut draft, KeyCode::Enter);
    select(&mut draft, "packages");
    while matches!(draft.step, Step::Packages(_)) {
        if draft.picker.items.iter().any(|p| p.value.is_empty()) {
            select(&mut draft, "");
        } else {
            draft.checked.clear();
            key(&mut draft, KeyCode::Enter);
        }
    }
    assert!(draft.graph.packages.is_empty());
    assert_eq!(draft.step, Step::Review);
}

#[test]
fn no_dependencies_skips_packages_and_testing_stays_optional() {
    let mut draft = draft();
    select(&mut draft, "guided");
    select(&mut draft, "none");
    assert_eq!(draft.step, Step::Capabilities);
    draft.checked.clear();
    key(&mut draft, KeyCode::Enter);
    assert!(draft.graph.test_runner().is_none());
    assert_eq!(draft.graph.mode, "none");
    assert!(draft.graph.packages.is_empty());
}

#[test]
fn testing_implementations_are_sorted_and_explicit_with_broad_testez_fallback() {
    for strategy in ["wally", "git", "none"] {
        let mut draft = draft();
        select(&mut draft, "expert");
        select(&mut draft, strategy);
        if strategy != "none" {
            key(&mut draft, KeyCode::Enter);
        }
        draft.checked = ["test".into()].into();
        key(&mut draft, KeyCode::Enter);
        if strategy == "wally" {
            assert_eq!(draft.step, Step::Implementation("test"));
            assert_eq!(
                draft
                    .picker
                    .items
                    .iter()
                    .map(|p| p.label.as_str())
                    .collect::<Vec<_>>(),
                vec!["Jest Roblox", "TestEZ"]
            );
            select(&mut draft, "jest-roblox");
            assert_eq!(draft.graph.capabilities["test"], "jest-roblox");
        } else {
            assert_eq!(draft.graph.capabilities["test"], "testez");
        }
        assert_eq!(draft.step, Step::Review);
    }
}

#[test]
fn saved_setups_preserve_concrete_choices_and_dropped_files() {
    for workflow in [
        PackageWorkflow::Wally,
        PackageWorkflow::GitSubmodules,
        PackageWorkflow::None,
    ] {
        let mut graph = ProjectGraph {
            package_workflow: workflow,
            dropped: vec!["test-examples".into()],
            ..Default::default()
        };
        graph.choose(
            "test",
            Some(if workflow == PackageWorkflow::Wally {
                "jest-roblox"
            } else {
                "testez"
            }),
        );
        super::super::new::apply_derived_packages(&mut graph);
        let draft = review_graph(graph.clone());
        graph.mode = "like:saved".into();
        assert_eq!(graph_value(&draft), serde_json::to_value(graph).unwrap());
        assert_eq!(draft.step, Step::Review);
    }
}

#[test]
fn backing_out_of_an_unanswered_runner_cannot_skip_capability_review() {
    let mut draft = draft();
    select(&mut draft, "expert");
    select(&mut draft, "wally");
    key(&mut draft, KeyCode::Enter);
    draft.checked = ["test".into()].into();
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.step, Step::Implementation("test"));
    key(&mut draft, KeyCode::Esc);
    assert!(draft.checked.contains("test"));
    key(&mut draft, KeyCode::Esc);
    select(&mut draft, "none");
    assert_eq!(draft.step, Step::Capabilities);
    assert!(draft.checked.contains("test"));

    let mut draft = super::model::Draft::new("Example", vec![], vec![], vec![]);
    select(&mut draft, "expert");
    select(&mut draft, "wally");
    key(&mut draft, KeyCode::Enter);
    draft.checked = ["test".into(), "asset-pipeline".into()].into();
    key(&mut draft, KeyCode::Enter);
    select(&mut draft, "jest-roblox");
    assert_eq!(draft.step, Step::Implementation("asset-pipeline"));
    key(&mut draft, KeyCode::Esc);
    key(&mut draft, KeyCode::Esc);
    select(&mut draft, "none");
    assert_eq!(draft.step, Step::RepairTesting);
    select(&mut draft, "testez");
    assert_eq!(draft.step, Step::Capabilities);
}

#[test]
fn strategy_revision_invalidates_only_dependencies_and_requests_testing_repair() {
    for runner in ["testez", "off"] {
        let mut graph = ProjectGraph {
            packages: vec!["signal".into()],
            dropped: vec!["test-examples".into()],
            ..Default::default()
        };
        graph.choose("test", Some("jest-roblox"));
        graph.choose("lint", None);
        let mut draft = review_graph(graph);
        select(&mut draft, "strategy");
        select(&mut draft, "none");
        assert_eq!(draft.step, Step::RepairTesting);
        assert!(draft.status.contains("requires Wally"));
        assert_eq!(draft.graph.capabilities["lint"], "selene");
        select(&mut draft, runner);
        assert_eq!(draft.step, Step::Review);
        assert!(draft.graph.testing_is_compatible());
        assert!(draft.graph.dropped.is_empty());
        assert!(!draft.graph.packages.iter().any(|p| p.starts_with("jest")));
        assert_eq!(
            draft.graph.capabilities.contains_key("test"),
            runner == "testez"
        );
    }
}

#[test]
fn cancelling_a_revision_restores_the_whole_reviewed_graph() {
    let mut graph = ProjectGraph {
        packages: vec!["signal".into()],
        dropped: vec![".gitignore".into()],
        ..Default::default()
    };
    graph.choose("test", Some("jest-roblox"));
    let mut draft = review_graph(graph);
    let before = graph_value(&draft);
    select(&mut draft, "strategy");
    select(&mut draft, "none");
    key(&mut draft, KeyCode::Esc);
    assert_eq!(graph_value(&draft), before);
    assert_eq!(draft.step, Step::Review);
}

#[test]
fn capability_requirements_and_unvendorable_packages_are_explicit_errors() {
    let mut draft = draft();
    select(&mut draft, "expert");
    select(&mut draft, "git");
    draft.checked.insert("reactReflex".into());
    key(&mut draft, KeyCode::Enter);
    assert!(matches!(draft.step, Step::Packages(_)));
    assert!(draft.status.contains("Cannot vendor"));
    draft.checked.clear();
    key(&mut draft, KeyCode::Enter);
    draft.checked = ["ci".into()].into();
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.step, Step::Capabilities);
    assert!(draft.status.contains("requires gate"));
}

#[test]
fn files_never_offer_managed_runner_wiring_and_filtering_keeps_selections() {
    let mut graph = ProjectGraph::default();
    graph.choose("test", Some("jest-roblox"));
    let mut draft = review_graph(graph);
    select(&mut draft, "files");
    for key in [
        "src",
        "default.project.json",
        "tests",
        "jest.project.json",
        "jest.config.json",
    ] {
        assert!(!draft.picker.items.iter().any(|p| p.value == key));
    }
    draft.checked.remove("test-examples");
    draft.picker.query = InputState::new("nonexistent");
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.graph.dropped, vec!["test-examples"]);
}

#[test]
fn create_needs_confirmation_and_ctrl_c_always_cancels() {
    let mut draft = review_graph(ProjectGraph::default());
    assert_eq!(select(&mut draft, "create"), Effect::None);
    assert!(matches!(draft.modal, Some(Modal::Create(_))));
    key(&mut draft, KeyCode::Esc);
    select(&mut draft, "create");
    assert_eq!(key(&mut draft, KeyCode::Enter), Effect::Create);
    draft.modal = Some(Modal::Name(InputState::new("unsaved")));
    assert_eq!(
        draft.key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Effect::Cancel
    );
}

#[test]
fn unicode_paste_name_validation_and_setup_refusal_preserve_existing_data() {
    let mut draft = review_graph(ProjectGraph::default());
    draft.modal = Some(Modal::Name(InputState::new("")));
    draft.paste("世界\nStudio");
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.name, "世界Studio");
    for name in [
        "",
        "..",
        "../project",
        "C:\\project",
        "NUL.txt",
        "COM1",
        "bad.",
        "bad?",
    ] {
        assert!(validate_name(name).is_err(), "{name}");
    }
    select(&mut draft, "setup");
    draft.paste("saved");
    key(&mut draft, KeyCode::Enter);
    assert!(matches!(draft.modal,Some(Modal::Setup(ref input)) if input.error.is_some()));
    assert!(draft.save_setup.is_none());
}

#[test]
fn wide_narrow_help_errors_input_and_minimum_render() {
    let mut draft = review_graph(ProjectGraph::default());
    for (width, height) in [(120, 30), (80, 24), (60, 16), (40, 10)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| super::render::draw(frame, &draft, "C:\\Projects\\Example"))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            text.contains(if width < 60 { "Resize" } else { "Review" }),
            "{text}"
        );
    }
    for modal in [Modal::Help, Modal::Name(InputState::new("世界"))] {
        draft.modal = Some(modal);
        draft.status = "Validation failed".into();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| super::render::draw(frame, &draft, "destination"))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            text.contains("Help") || (text.contains('世') && text.contains('界')),
            "{text}"
        );
    }
}
