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
    let (root, mut app) = app();
    let source = format!(
        "{SOURCE}{}",
        (0..40)
            .map(|n| format!("# Recovery detail {n:02}\n"))
            .collect::<String>()
    );
    for n in 0..30 {
        std::fs::write(root.path().join(format!("sample{n:02}.toml")), &source).unwrap();
    }
    std::fs::write(root.path().join("other.toml"), SOURCE).unwrap();
    app.refresh(None);
    for ch in "sample".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    assert_eq!(app.picker.filtered().len(), 31);
    press(&mut app, KeyCode::End);
    assert_eq!(app.picker.selected, 30);
    assert_eq!(app.picker.selected_value().unwrap(), "sample29");
    press(&mut app, KeyCode::Tab);
    for _ in 0..3 {
        press(&mut app, KeyCode::PageDown);
    }
    let scroll = app.scroll;
    assert!(scroll > 0);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let baseline = terminal.backend().buffer().clone();
    assert!(app.list_state.offset() > 0);
    for (width, height) in [
        (120, 30),
        (280, 70),
        (80, 24),
        (60, 16),
        (40, 10),
        (120, 30),
    ] {
        terminal.backend_mut().resize(width, height);
        terminal.draw(|frame| app.render(frame)).unwrap();
        let before_help = terminal.backend().buffer().clone();
        let output: String = before_help
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(output.contains("Saved Setups"));
        assert_eq!(app.picker.query.text(), "sample");
        assert_eq!(app.picker.selected, 30);
        assert_eq!(app.picker.selected_value().unwrap(), "sample29");
        assert_eq!(app.document.as_ref().unwrap().name, "sample29");
        assert_eq!(app.scroll, scroll);
        assert!(app.details_focus);
        assert!(matches!(app.view, View::Browser));
        if width < 60 {
            assert!(output.contains("Resize to at least 60x16"));
        } else {
            let list =
                tui::responsive_panes(ratatui::layout::Rect::new(0, 2, width, height - 5), 40)[0];
            let row = (list.y + 1..list.bottom() - 1)
                .find(|&y| {
                    let text: String = (list.x + 1..list.right() - 1)
                        .map(|x| before_help[(x, y)].symbol())
                        .collect();
                    text.starts_with("sample29")
                })
                .expect("selected setup must remain visible in the list pane");
            for x in list.x + 1..list.x + 1 + "sample29".len() as u16 {
                assert_eq!(before_help[(x, row)].fg, ratatui::style::Color::Black);
                assert_eq!(before_help[(x, row)].bg, ratatui::style::Color::DarkGray);
            }
            if height <= 30 {
                assert!(app.list_state.offset() > 0);
            }
            assert!(output.contains("Recovery detail"));
            assert!(!output.contains("other"));
        }
        press(&mut app, KeyCode::Char('?'));
        terminal.draw(|frame| app.render(frame)).unwrap();
        let help: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(help.contains("Help"));
        assert!(!press(&mut app, KeyCode::Esc));
        assert!(app.modal.is_none());
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(terminal.backend().buffer(), &before_help);
    }
    // Resizing can move the list offset while keeping its selected row visible.
    let details = tui::responsive_panes(ratatui::layout::Rect::new(0, 2, 120, 25), 40)[1];
    for y in details.y..details.bottom() {
        for x in details.x..details.right() {
            assert_eq!(terminal.backend().buffer()[(x, y)], baseline[(x, y)]);
        }
    }
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.scroll, scroll - 10);
    assert_ne!(
        rendered(&mut app, 120, 30),
        baseline
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    );
    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.scroll, scroll);
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    assert!(matches!(app.view, View::Actions));
    assert!(!press(&mut app, KeyCode::Esc));
    assert!(matches!(app.view, View::Browser));
    assert_eq!(app.picker.query.text(), "sample");
    assert_eq!(app.picker.selected_value().unwrap(), "sample29");
    assert_eq!(app.scroll, scroll);
    app.status = "Cannot read storage".into();
    assert!(rendered(&mut app, 120, 30).contains("Cannot read storage"));
    edit(&mut app);
    assert!(rendered(&mut app, 120, 30).contains("Edit Saved Setup"));
    press(&mut app, KeyCode::Char('?'));
    let help = rendered(&mut app, 120, 30);
    assert!(help.contains("Ctrl+S"));
    assert!(!help.contains("New Project"));
}
