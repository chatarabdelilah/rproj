use super::model::{Draft, Effect, Modal, Step, validate_name};
use crate::{config::PackageWorkflow, graph::ProjectGraph, tui::InputState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};

fn draft() -> Draft {
    let mut draft = Draft::new("Example", vec!["saved".into()], vec![], vec![]);
    select(&mut draft, "source");
    draft
}

#[test]
fn new_project_starts_on_review_and_keeps_start_point_choices_available() {
    let mut draft = Draft::new("Example", vec!["saved".into()], vec![], vec![]);
    assert_eq!(draft.step, Step::Review);
    assert!(draft.picker.items.iter().any(|item| item.value == "source"));
    select(&mut draft, "source");
    assert_eq!(draft.step, Step::Source);
    assert!(draft.picker.items.iter().any(|item| item.value == "guided"));
    assert!(draft.picker.items.iter().any(|item| item.value == "expert"));
    assert!(
        draft
            .picker
            .items
            .iter()
            .any(|item| item.value == "saved:saved")
    );
}

#[test]
fn fresh_project_choices_are_empty_and_revision_keeps_explicit_choices() {
    for mode in ["guided", "expert"] {
        for strategy in ["wally", "git", "none"] {
            let mut draft = draft();
            select(&mut draft, mode);
            select(&mut draft, strategy);
            while matches!(draft.step, Step::Packages(_)) {
                assert!(draft.checked.is_empty());
                key(&mut draft, KeyCode::Enter);
            }
            assert_eq!(draft.step, Step::Capabilities);
            assert!(draft.checked.is_empty());
            key(&mut draft, KeyCode::Enter);
            assert_eq!(draft.step, Step::Review);
            assert!(draft.graph.packages.is_empty());
            assert!(draft.graph.capabilities.is_empty());
            select(&mut draft, "capabilities");
            draft.checked.insert("lint".into());
            key(&mut draft, KeyCode::Enter);
            select(&mut draft, "capabilities");
            assert_eq!(draft.checked, ["lint".into()].into());
        }
    }
}

#[test]
fn jest_execution_choice_is_explicit_and_saved_with_the_composition() {
    for backend in ["jest-roblox", "jest-roblox-open-cloud"] {
        let mut draft = draft();
        select(&mut draft, "expert");
        select(&mut draft, "wally");
        key(&mut draft, KeyCode::Enter);
        draft.checked = ["test".into()].into();
        key(&mut draft, KeyCode::Enter);
        select(&mut draft, "jest-roblox");
        assert_eq!(draft.step, Step::JestBackend);
        select(&mut draft, backend);
        assert_eq!(draft.step, Step::Review);
        assert_eq!(draft.graph.capabilities["test"], backend);
        let saved = toml::to_string(&draft.graph).unwrap();
        let restored: ProjectGraph = toml::from_str(&saved).unwrap();
        assert_eq!(restored.jest_backend(), draft.graph.jest_backend());
        let mut editor = Draft::edit_setup("example", restored);
        select(&mut editor, "capabilities");
        key(&mut editor, KeyCode::Enter);
        select(&mut editor, "jest-roblox");
        assert_eq!(
            editor.picker.selected_value().map(String::as_str),
            Some(backend)
        );
    }
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
            assert_eq!(draft.step, Step::JestBackend);
            select(&mut draft, "jest-roblox");
            assert_eq!(draft.graph.capabilities["test"], "jest-roblox");
        } else {
            assert_eq!(draft.graph.capabilities["test"], "testez");
        }
        assert_eq!(draft.step, Step::Review);
    }
}

#[test]
fn creation_screens_match_shared_choices_and_graph_results() {
    use crate::catalog::capabilities;

    for workflow in PackageWorkflow::ALL {
        for capability in capabilities::CAPABILITIES {
            for implementation in capability.implementation_choices(*workflow) {
                let selections = if capability.needs_jest_backend(implementation.key) {
                    vec!["jest-roblox", "jest-roblox-open-cloud"]
                } else {
                    vec![implementation.key]
                };
                for selected in selections {
                    let mut draft = Draft::new("Example", vec![], vec![], vec![]);
                    draft.graph.package_workflow = *workflow;
                    select(&mut draft, "capabilities");
                    draft.checked.insert(capability.key.into());
                    draft
                        .checked
                        .extend(capability.requires.iter().map(|key| (*key).into()));
                    key(&mut draft, KeyCode::Enter);
                    if capability.needs_an_implementation_prompt(*workflow) {
                        assert_eq!(draft.step, Step::Implementation(capability.key));
                        assert_eq!(
                            draft
                                .picker
                                .items
                                .iter()
                                .map(|item| item.value.as_str())
                                .collect::<Vec<_>>(),
                            capability
                                .implementation_choices(*workflow)
                                .iter()
                                .map(|i| i.key)
                                .collect::<Vec<_>>()
                        );
                        select(&mut draft, implementation.key);
                    }
                    if capability.needs_jest_backend(implementation.key) {
                        assert_eq!(draft.step, Step::JestBackend);
                        select(&mut draft, selected);
                    }
                    assert_eq!(draft.step, Step::Review);
                    let mut expected = ProjectGraph {
                        package_workflow: *workflow,
                        ..Default::default()
                    };
                    for required in capability.requires {
                        expected.choose(required, None);
                    }
                    expected.choose(capability.key, Some(selected));
                    super::super::new::apply_derived_packages(&mut expected);
                    assert_eq!(draft.graph.capabilities, expected.capabilities);
                    assert_eq!(draft.graph.packages, expected.packages);
                    assert_eq!(draft.graph.derived(), expected.derived());
                }
            }
        }
    }
}

#[test]
fn saved_unknown_choices_survive_inspection_and_unrelated_choices_are_preserved() {
    let mut graph = ProjectGraph::default();
    graph
        .capabilities
        .insert("future-capability".into(), "future-tool".into());
    graph
        .capabilities
        .insert("test".into(), "future-runner".into());
    let mut editor = Draft::edit_setup("example", graph.clone());
    select(&mut editor, "capabilities");
    assert_eq!(editor.graph.capabilities, graph.capabilities);
    key(&mut editor, KeyCode::Esc);
    assert_eq!(editor.graph.capabilities, graph.capabilities);
    select(&mut editor, "capabilities");
    key(&mut editor, KeyCode::Enter);
    select(&mut editor, "testez");
    assert_eq!(editor.graph.capabilities["test"], "testez");
    assert_eq!(
        editor.graph.capabilities["future-capability"],
        "future-tool"
    );
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
fn manager_open_cancel_and_review_preserve_recorded_choices() {
    let graph = ProjectGraph {
        mode: "like:original".into(),
        packages: vec!["signal".into()],
        dropped: vec!["future-file".into(), ".gitignore".into()],
        ..Default::default()
    };
    let mut draft = Draft::edit_setup("saved", graph.clone());
    assert_eq!(draft.step, Step::Review);
    assert_eq!(graph_value(&draft), serde_json::to_value(&graph).unwrap());
    assert!(
        !draft
            .picker
            .items
            .iter()
            .any(|item| ["create", "name", "setup"].contains(&item.value.as_str()))
    );
    select(&mut draft, "strategy");
    select(&mut draft, "none");
    assert_eq!(draft.graph.mode, graph.mode);
    assert_eq!(draft.graph.dropped, graph.dropped);
    select(&mut draft, "packages");
    key(&mut draft, KeyCode::Esc);
    assert_eq!(draft.graph.mode, graph.mode);
}

#[test]
fn manager_jest_repair_and_optional_files_keep_unknown_exclusions() {
    let mut graph = ProjectGraph {
        mode: "expert".into(),
        dropped: vec!["future-file".into()],
        ..Default::default()
    };
    graph.choose("test", Some("jest-roblox"));
    let mut draft = Draft::edit_setup("saved", graph);
    select(&mut draft, "strategy");
    select(&mut draft, "none");
    assert_eq!(draft.step, Step::RepairTesting);
    select(&mut draft, "testez");
    assert_eq!(draft.step, Step::Review);
    assert_eq!(draft.graph.capabilities["test"], "testez");
    select(&mut draft, "files");
    key(&mut draft, KeyCode::Enter);
    assert!(draft.graph.dropped.contains(&"future-file".into()));
    assert_eq!(draft.graph.mode, "expert");
    assert_eq!(
        draft.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL)),
        Effect::Save
    );
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
    assert_eq!(draft.step, Step::Review);
    assert!(draft.graph.test_runner().is_none());
}

#[test]
fn project_editor_adds_choices_without_removing_existing_ones() {
    let mut graph = ProjectGraph {
        mode: "expert".into(),
        packages: vec!["janitor".into()],
        ..Default::default()
    };
    graph.choose("lint", Some("selene"));
    let mut draft = Draft::edit_project("Example", graph, vec![], vec![]);
    assert!(draft.project_mode);
    assert_eq!(draft.step, Step::Review);
    assert!(
        !draft
            .picker
            .items
            .iter()
            .any(|item| item.value == "strategy")
    );

    select(&mut draft, "packages");
    draft.picker.selected = draft
        .picker
        .items
        .iter()
        .position(|item| item.value == "janitor")
        .unwrap();
    key(&mut draft, KeyCode::Char(' '));
    assert!(draft.checked.contains("janitor"));
    draft.checked.insert("promise".into());
    key(&mut draft, KeyCode::Enter);
    assert!(draft.graph.packages.contains(&"janitor".into()));
    assert!(draft.graph.packages.contains(&"promise".into()));

    select(&mut draft, "capabilities");
    draft.picker.selected = draft
        .picker
        .items
        .iter()
        .position(|item| item.value == "lint")
        .unwrap();
    key(&mut draft, KeyCode::Char(' '));
    assert!(draft.checked.contains("lint"));
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
    select(&mut draft, "name");
    let name_row = draft.picker.selected;
    draft.modal = Some(Modal::Name(InputState::new("")));
    draft.paste("世界\nStudio");
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.name, "世界Studio");
    assert_eq!(draft.picker.selected, name_row);
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
    let setup_row = draft.picker.selected;
    draft.paste("saved");
    key(&mut draft, KeyCode::Enter);
    assert!(matches!(draft.modal,Some(Modal::Setup(ref input)) if input.error.is_some()));
    assert!(draft.save_setup.is_none());
    assert_eq!(draft.picker.selected, setup_row);
}

#[test]
fn new_project_resize_preserves_filtered_checked_package_revision() {
    let mut draft = draft();
    select(&mut draft, "expert");
    select(&mut draft, "wally");
    key(&mut draft, KeyCode::Enter);
    key(&mut draft, KeyCode::Enter);
    assert_eq!(draft.step, Step::Review);
    let reviewed = graph_value(&draft);

    select(&mut draft, "packages");
    draft.paste("janitor");
    key(&mut draft, KeyCode::Char(' '));
    key(&mut draft, KeyCode::Tab);
    key(&mut draft, KeyCode::Down);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    let draw = |terminal: &mut Terminal<TestBackend>, draft: &Draft| {
        terminal
            .draw(|frame| super::render::draw(frame, draft, "C:\\Projects\\Example"))
            .unwrap();
    };
    draw(&mut terminal, &draft);
    let baseline = terminal.backend().buffer().clone();
    for (width, height) in [(120, 30), (80, 24), (60, 16), (40, 10), (120, 30)] {
        terminal.backend_mut().resize(width, height);
        draw(&mut terminal, &draft);
        let before_help = terminal.backend().buffer().clone();
        assert_eq!(draft.step, Step::Packages(None));
        assert_eq!(draft.picker.query.text(), "janitor");
        assert_eq!(
            draft.picker.selected_value().map(String::as_str),
            Some("janitor")
        );
        assert_eq!(draft.checked, ["janitor".into()].into());
        assert_eq!(graph_value(&draft), reviewed);
        assert!(draft.details_focus);
        assert_eq!(draft.scroll, 1);
        let text: String = before_help.content().iter().map(|c| c.symbol()).collect();
        if width < 60 {
            assert!(text.contains("Resize to at least 60x16"));
        } else {
            assert!(text.contains("New Project"));
            assert!(text.contains("Filter: janitor"));
            assert!(text.contains("[x] janitor"));
        }
        assert_eq!(key(&mut draft, KeyCode::Char('?')), Effect::None);
        draw(&mut terminal, &draft);
        let help: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(help.contains("Help"));
        assert_eq!(key(&mut draft, KeyCode::Esc), Effect::None);
        assert!(draft.modal.is_none());
        draw(&mut terminal, &draft);
        assert_eq!(terminal.backend().buffer(), &before_help);
    }
    assert_eq!(terminal.backend().buffer(), &baseline);
    key(&mut draft, KeyCode::Up);
    assert_eq!(draft.scroll, 0);
    draw(&mut terminal, &draft);
    assert_ne!(terminal.backend().buffer(), &baseline);
    key(&mut draft, KeyCode::Tab);
    assert_eq!(key(&mut draft, KeyCode::Enter), Effect::None);
    assert_eq!(draft.step, Step::Review);
    let mut expected = reviewed;
    expected["packages"] = serde_json::json!(["janitor"]);
    assert_eq!(graph_value(&draft), expected);
    select(&mut draft, "packages");
    assert_eq!(draft.checked, ["janitor".into()].into());
    key(&mut draft, KeyCode::Esc);
    assert_eq!(draft.step, Step::Review);
    assert_eq!(graph_value(&draft), expected);
}

#[test]
fn creation_pty_driver() {
    let Some(root) = std::env::var_os("RPROJ_CREATION_TEST_ROOT") else {
        return;
    };
    let mut draft = draft();
    select(&mut draft, "expert");
    select(&mut draft, "wally");
    key(&mut draft, KeyCode::Enter);
    key(&mut draft, KeyCode::Enter);
    let mut expected = graph_value(&draft);
    expected["packages"] = serde_json::json!(["janitor"]);
    select(&mut draft, "packages");
    let destination = std::path::PathBuf::from(root).join("Example");
    let mut terminal = crate::tui::TerminalSession::enter().unwrap();
    loop {
        match super::next_effect(
            &mut terminal,
            &mut draft,
            &destination.display().to_string(),
        )
        .unwrap()
        {
            Effect::None => {}
            Effect::Cancel => break,
            effect => panic!("unexpected execution effect: {effect:?}"),
        }
    }
    drop(terminal);
    assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
    assert_eq!(draft.step, Step::Review);
    assert_eq!(graph_value(&draft), expected);
    assert!(draft.save_setup.is_none());
    println!("Creation cancelled; reviewed packages retained; terminal restored");
}

fn expected_pty_screen(draft: &Draft, destination: &str) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| super::render::draw(frame, draft, destination))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(120)
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .trim_end_matches(' ')
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[track_caller]
fn wait_pty_screen(session: &crate::test_common::Session, ready: impl Fn(&str) -> bool) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let screen = session.screen();
        // Physical rows ignore ConPTY soft-wrap metadata while preserving layout.
        let text = screen
            .rows(0, screen.size().1)
            .map(|row| row.trim_end_matches(' ').to_owned())
            .collect::<Vec<_>>()
            .join("\n");
        if ready(&text) {
            return text;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "screen did not settle:\n{text}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn pty_new_project_resize_recovers_checked_package_revision_without_creating() {
    use crate::test_common as common;
    let root = tempfile::tempdir().unwrap();
    let sentinel = root.path().join("sentinel.txt");
    std::fs::write(&sentinel, b"preserve fixture bytes\n").unwrap();
    let mut expected_draft = draft();
    select(&mut expected_draft, "expert");
    select(&mut expected_draft, "wally");
    key(&mut expected_draft, KeyCode::Enter);
    key(&mut expected_draft, KeyCode::Enter);
    select(&mut expected_draft, "packages");
    expected_draft.paste("janitor");
    key(&mut expected_draft, KeyCode::Char(' '));
    key(&mut expected_draft, KeyCode::Tab);
    key(&mut expected_draft, KeyCode::Down);
    let destination = root.path().join("Example").display().to_string();
    let baseline = expected_pty_screen(&expected_draft, &destination);
    let mut session = common::Session::start_program(
        &std::env::current_exe().unwrap(),
        root.path(),
        &[
            "--exact",
            "commands::creation::tests::creation_pty_driver",
            "--nocapture",
            "--test-threads=1",
        ],
        &[
            ("RPROJ_CREATION_TEST_ROOT", root.path().to_str().unwrap()),
            ("RPROJ_NO_LOG", "1"),
        ],
    );
    session.wait_for(" New Project:");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, " Packages ");
    session.send("janitor");
    session.wait_for("Filter: janitor");
    session.send(" ");
    session.wait_for("[x] janitor");
    session.send("\t");
    session.send(common::DOWN);
    let checkpoint = session.output_checkpoint();
    session.send("?");
    session.wait_for_output_since(checkpoint, "Esc or ? closes Help.");
    session.send(common::ESC);
    wait_pty_screen(&session, |text| text == baseline);
    for (rows, cols) in [(24, 80), (16, 60), (10, 40)] {
        session.resize(rows, cols);
        let expected = if cols < 60 {
            "Resize to at least 60x16"
        } else {
            "Filter: janitor"
        };
        wait_pty_screen(&session, |text| {
            text.contains(expected) && !text.contains("┌ Help ")
        });
        session.send("?");
        session.wait_for("┌ Help ");
        session.send(common::ESC);
        wait_pty_screen(&session, |text| {
            text.contains(expected) && !text.contains("┌ Help ")
        });
        if cols < 60 {
            session.send("\t \rblocked\x1b[200~pasted\x1b[201~\x13");
            // Help is an input barrier before resizing so blocked keys cannot run later.
            session.send("?");
            session.wait_for("┌ Help ");
            session.send(common::ESC);
            wait_pty_screen(&session, |text| {
                text.contains(expected) && !text.contains("┌ Help ")
            });
        }
        assert!(!root.path().join("Example").exists());
        assert_eq!(
            std::fs::read(&sentinel).unwrap(),
            b"preserve fixture bytes\n"
        );
    }
    session.resize(30, 120);
    wait_pty_screen(&session, |text| text == baseline);
    session.send("\x1b[A");
    let checkpoint = session.output_checkpoint();
    session.send("?");
    session.wait_for_output_since(checkpoint, "Esc or ? closes Help.");
    session.send(common::ESC);
    key(&mut expected_draft, KeyCode::Up);
    let unscrolled = expected_pty_screen(&expected_draft, &destination);
    wait_pty_screen(&session, |text| text == unscrolled);
    assert_ne!(unscrolled, baseline);
    session.send("\t");
    session.send(common::ENTER);
    session.wait_for(" Review ");
    session.send("packages");
    session.send(common::ENTER);
    session.wait_for(" Packages ");
    session.send("janitor");
    session.wait_for("[x] janitor");
    session.send(common::ESC);
    session.wait_for("Revision cancelled.");
    session.send("\x03");
    session.wait_for("Creation cancelled; reviewed packages retained; terminal restored");
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(!root.path().join("Example").exists());
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"preserve fixture bytes\n"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
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
