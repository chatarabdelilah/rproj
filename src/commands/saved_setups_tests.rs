use super::*;
use ratatui::{Terminal, backend::TestBackend};

const SOURCE: &str = "# keep me\nmode = 'like:original'\npackage_workflow = 'wally'\npackages = []\ndropped = ['future-file']\n[capabilities]\ntest = 'testez'\n";

#[test]
fn setup_pty_driver() {
    let Some(root) = std::env::var_os("RPROJ_SETUP_TEST_ROOT") else {
        return;
    };
    let mut app = SavedSetupsApp::at(Ok(root.into()));
    app.open();
    let mut terminal = tui::TerminalSession::enter().unwrap();
    loop {
        terminal.draw(|frame| app.render(frame)).unwrap();
        match terminal.read_event().unwrap() {
            crossterm::event::Event::Key(key)
                if key.kind != crossterm::event::KeyEventKind::Release =>
            {
                if app.handle_key(key) {
                    break;
                }
            }
            crossterm::event::Event::Paste(text) => app.paste(&text),
            _ => {}
        }
    }
    drop(terminal);
    println!("Manager returned");
}

#[test]
fn pty_repeated_save_exit_and_default_no_delete_restore_terminal() {
    use super::common;
    let (root, _app) = app();
    let mut session = common::Session::start_program(
        &std::env::current_exe().unwrap(),
        root.path(),
        &[
            "commands::saved_setups::tests::setup_pty_driver",
            "--exact",
            "--nocapture",
        ],
        &[("RPROJ_SETUP_TEST_ROOT", root.path().to_str().unwrap())],
    );
    session.wait_for("Filter:");
    session.send(common::ENTER);
    session.wait_for("Actions");
    session.send(common::ENTER);
    session.wait_for("Edit Saved Setup");
    session.send("\x13");
    session.wait_for("No changes to save.");
    session.send("\x13");
    session.send(common::ESC);
    session.wait_for("Actions");
    for _ in 0..3 {
        session.send(common::DOWN);
    }
    session.send(common::ENTER);
    session.wait_for("[No]");
    session.send(common::ENTER);
    session.wait_for("Actions");
    session.send("\x03");
    session.wait_for("Manager returned");
    assert_eq!(session.finish().code, 0);
    assert_eq!(
        std::fs::read_to_string(root.path().join("sample.toml")).unwrap(),
        SOURCE
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

fn app() -> (tempfile::TempDir, SavedSetupsApp) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("sample.toml"), SOURCE).unwrap();
    let mut app = SavedSetupsApp::at(Ok(root.path().into()));
    app.open();
    (root, app)
}

fn press(app: &mut SavedSetupsApp, code: KeyCode) -> bool {
    app.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn save(app: &mut SavedSetupsApp) {
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
}

fn edit(app: &mut SavedSetupsApp) {
    press(app, KeyCode::Enter);
    press(app, KeyCode::Enter);
    assert!(matches!(app.view, View::Editor(_)));
}

fn rendered(app: &mut SavedSetupsApp, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn repeated_save_updates_baseline_without_normalizing_open() {
    let (root, mut app) = app();
    edit(&mut app);
    assert!(!app.dirty());
    save(&mut app);
    assert_eq!(
        std::fs::read_to_string(root.path().join("sample.toml")).unwrap(),
        SOURCE
    );
    if let View::Editor(draft) = &mut app.view {
        assert_eq!(draft.graph.mode, "like:original");
        draft.graph.capabilities.clear();
    }
    assert!(app.dirty());
    save(&mut app);
    assert!(!app.dirty());
    assert!(matches!(app.view, View::Editor(_)));
    let bytes = std::fs::read(root.path().join("sample.toml")).unwrap();
    save(&mut app);
    assert_eq!(
        std::fs::read(root.path().join("sample.toml")).unwrap(),
        bytes
    );
    assert!(!press(&mut app, KeyCode::Esc));
    assert!(matches!(app.view, View::Actions));
}

#[test]
fn dirty_exit_defaults_no_and_confirmed_discard_does_not_write() {
    let (root, mut app) = app();
    edit(&mut app);
    if let View::Editor(draft) = &mut app.view {
        draft.graph.capabilities.clear();
    }
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Enter);
    assert!(app.dirty());
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.view, View::Actions));
    assert_eq!(
        std::fs::read_to_string(root.path().join("sample.toml")).unwrap(),
        SOURCE
    );
}

#[test]
fn save_conflict_retains_draft_and_last_loaded_baseline() {
    let (root, mut app) = app();
    edit(&mut app);
    if let View::Editor(draft) = &mut app.view {
        draft.graph.capabilities.clear();
    }
    std::fs::write(root.path().join("sample.toml"), "external = true").unwrap();
    save(&mut app);
    assert!(app.dirty());
    if let View::Editor(draft) = &app.view {
        assert!(draft.status.contains("refresh"));
    }
    assert_eq!(
        std::fs::read_to_string(root.path().join("sample.toml")).unwrap(),
        "external = true"
    );
}

#[test]
fn malformed_setup_and_default_no_delete() {
    let (root, mut app) = app();
    std::fs::write(root.path().join("sample.toml"), "broken [").unwrap();
    app.refresh(None);
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.view, View::Actions));
    assert!(app.status.contains("disabled"));
    app.action = 3;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(root.path().join("sample.toml").exists());
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Right);
    press(&mut app, KeyCode::Enter);
    assert!(!root.path().join("sample.toml").exists());
    assert!(matches!(app.view, View::Browser));
}

#[test]
fn browser_back_preserves_filter_selection_and_details_scroll() {
    let (_root, mut app) = app();
    app.paste("sam");
    app.scroll = 4;
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.picker.query.text(), "sam");
    assert_eq!(
        app.picker.selected_value().map(String::as_str),
        Some("sample")
    );
    assert_eq!(app.scroll, 4);
}

#[test]
fn wide_narrow_tiny_help_error_and_editor_render() {
    let (_root, mut app) = app();
    for (width, height) in [(120, 30), (280, 70), (80, 24), (40, 10)] {
        assert!(rendered(&mut app, width, height).contains("Saved Setups"));
        press(&mut app, KeyCode::Char('?'));
        assert!(rendered(&mut app, width, height).contains("Help"));
        press(&mut app, KeyCode::Esc);
    }
    app.status = "Cannot read storage".into();
    assert!(rendered(&mut app, 120, 30).contains("Cannot read storage"));
    edit(&mut app);
    assert!(rendered(&mut app, 120, 30).contains("Edit Saved Setup"));
    press(&mut app, KeyCode::Char('?'));
    let help = rendered(&mut app, 120, 30);
    assert!(help.contains("Ctrl+S"));
    assert!(!help.contains("New Project"));
}
