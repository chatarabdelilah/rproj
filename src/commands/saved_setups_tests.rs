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
        let mut small = false;
        terminal
            .draw(|frame| {
                small = tui::is_too_small(frame.area());
                app.render(frame);
            })
            .unwrap();
        match terminal.read_event().unwrap() {
            crossterm::event::Event::Key(key)
                if key.kind != crossterm::event::KeyEventKind::Release =>
            {
                if routed_key(&mut app, small, key) {
                    break;
                }
            }
            crossterm::event::Event::Paste(text) if !small => app.paste(&text),
            _ => {}
        }
    }
    drop(terminal);
    assert!(!crossterm::terminal::is_raw_mode_enabled().unwrap());
    println!("Manager returned");
}

// Match Home's input gate: an undersized editor only accepts help and exit keys,
// plus discard-confirmation choices. Storage and terminal state stay disposable.
fn routed_key(app: &mut SavedSetupsApp, small: bool, key: KeyEvent) -> bool {
    if !small
        || app.exit_confirmation_key(key)
        || matches!(key.code, KeyCode::Esc | KeyCode::Char('?'))
        || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c'))
    {
        app.handle_key(key)
    } else {
        false
    }
}

fn screen_rows(screen: &vt100::Screen) -> String {
    // ConPTY redraws can change soft-wrap flags without changing the visible rows.
    screen
        .rows(0, screen.size().1)
        .map(|row| row.trim_end_matches(' ').to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn expected_screen(app: &mut SavedSetupsApp) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
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

#[test]
fn screen_rows_ignore_soft_wrap_metadata_but_preserve_layout() {
    let mut wrapped = vt100::Parser::new(2, 4, 0);
    wrapped.process(b"abcde");
    let mut positioned = vt100::Parser::new(2, 4, 0);
    positioned.process(b"abcd\x1b[2;1He");
    assert_ne!(wrapped.screen().contents(), positioned.screen().contents());
    assert_eq!(screen_rows(wrapped.screen()), "abcd\ne");
    assert_eq!(
        screen_rows(wrapped.screen()),
        screen_rows(positioned.screen())
    );
    positioned.process(b"\x1b[2;4H ");
    assert_eq!(
        screen_rows(wrapped.screen()),
        screen_rows(positioned.screen())
    );

    let mut different_layout = vt100::Parser::new(2, 4, 0);
    different_layout.process(b"abc\x1b[2;1Hde");
    assert_eq!(
        wrapped.screen().contents(),
        different_layout.screen().contents().replace('\n', "")
    );
    assert_ne!(
        screen_rows(wrapped.screen()),
        screen_rows(different_layout.screen())
    );
}

#[track_caller]
fn wait_screen(session: &super::common::Session, ready: impl Fn(&str) -> bool) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let screen = screen_rows(&session.screen());
        if ready(&screen) {
            return screen;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "screen did not settle:\n{screen}"
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[test]
fn editor_resize_preserves_revision_help_scroll_and_saved_discard_baseline() {
    let (root, mut app) = app();
    edit(&mut app);
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    app.paste("test");
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Down);
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let baseline = terminal.backend().buffer().clone();
    for (width, height) in [(80, 24), (60, 16), (40, 10), (120, 30)] {
        terminal.backend_mut().resize(width, height);
        terminal.draw(|frame| app.render(frame)).unwrap();
        let before_help = terminal.backend().buffer().clone();
        let View::Editor(draft) = &app.view else {
            panic!("resize lost the editor");
        };
        assert_eq!(
            draft.step,
            crate::commands::creation::model::Step::Capabilities
        );
        assert_eq!(draft.picker.query.text(), "test");
        assert_eq!(
            draft.picker.selected_value().map(String::as_str),
            Some("test")
        );
        assert!(!draft.checked.contains("test"));
        assert_eq!(draft.graph.capabilities["test"], "testez");
        assert!(draft.details_focus);
        assert_eq!(draft.scroll, 1);
        press(&mut app, KeyCode::Char('?'));
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert!(rendered(&mut app, width, height).contains("Help"));
        assert!(!press(&mut app, KeyCode::Esc));
        terminal.draw(|frame| app.render(frame)).unwrap();
        assert_eq!(terminal.backend().buffer(), &before_help);
    }
    assert_eq!(terminal.backend().buffer(), &baseline);
    press(&mut app, KeyCode::Up);
    let View::Editor(draft) = &app.view else {
        unreachable!()
    };
    assert_eq!(draft.scroll, 0);
    assert_ne!(
        rendered(&mut app, 120, 30),
        baseline
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    );
    press(&mut app, KeyCode::Tab);
    press(&mut app, KeyCode::Enter);
    assert!(app.dirty());
    let original = std::fs::read(root.path().join("sample.toml")).unwrap();
    assert_eq!(original, SOURCE.as_bytes());
    save(&mut app);
    assert!(!app.dirty());
    let saved = std::fs::read(root.path().join("sample.toml")).unwrap();
    assert_ne!(saved, original);
    let graph: crate::graph::ProjectGraph =
        toml::from_str(std::str::from_utf8(&saved).unwrap()).unwrap();
    assert!(!graph.capabilities.contains_key("test"));
    // A second revision is dirty relative to the successful save, not the open bytes.
    for _ in 0..3 {
        press(&mut app, KeyCode::Down);
    }
    press(&mut app, KeyCode::Enter);
    app.paste("test");
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    assert!(app.dirty());
    terminal.backend_mut().resize(40, 10);
    terminal.draw(|frame| app.render(frame)).unwrap();
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    assert!(!routed_key(&mut app, true, key(KeyCode::Esc)));
    assert!(rendered(&mut app, 40, 10).contains("[No]"));
    assert!(!routed_key(&mut app, true, key(KeyCode::Enter)));
    assert!(app.modal.is_none());
    assert!(app.dirty());
    assert!(!routed_key(&mut app, true, key(KeyCode::Esc)));
    assert!(!routed_key(&mut app, true, key(KeyCode::Right)));
    assert!(rendered(&mut app, 40, 10).contains("[Yes]"));
    assert!(!routed_key(&mut app, true, key(KeyCode::Enter)));
    assert!(matches!(app.view, View::Actions));
    assert_eq!(
        std::fs::read(root.path().join("sample.toml")).unwrap(),
        saved
    );
    press(&mut app, KeyCode::Enter);
    assert!(!app.dirty());
    assert!(routed_key(
        &mut app,
        true,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    ));
    assert_eq!(
        std::fs::read(root.path().join("sample.toml")).unwrap(),
        saved
    );
}

#[test]
fn pty_editor_resize_recovers_filtered_revision_and_saves_only_on_request() {
    use super::common;
    let (root, mut expected_app) = app();
    edit(&mut expected_app);
    for _ in 0..3 {
        press(&mut expected_app, KeyCode::Down);
    }
    press(&mut expected_app, KeyCode::Enter);
    expected_app.paste("test");
    press(&mut expected_app, KeyCode::Char(' '));
    press(&mut expected_app, KeyCode::Tab);
    press(&mut expected_app, KeyCode::Down);
    let baseline = expected_screen(&mut expected_app);
    let path = root.path().join("sample.toml");
    let mut session = common::Session::start_program(
        &std::env::current_exe().unwrap(),
        root.path(),
        &[
            "--exact",
            "commands::saved_setups::tests::setup_pty_driver",
            "--nocapture",
            "--test-threads=1",
        ],
        &[
            ("RPROJ_SETUP_TEST_ROOT", root.path().to_str().unwrap()),
            ("RPROJ_NO_LOG", "1"),
        ],
    );
    session.wait_for("Filter:");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Filter:");
    session.send(common::ENTER);
    session.wait_for("Actions");
    session.send(common::ENTER);
    session.wait_for("Edit Saved Setup");
    for _ in 0..3 {
        session.send(common::DOWN);
    }
    session.send(common::ENTER);
    session.wait_for("[x] test");
    session.send("test");
    session.wait_for("Filter: test");
    session.send(" ");
    session.wait_for("[ ] test");
    session.send("\t");
    session.send(common::DOWN);
    let checkpoint = session.output_checkpoint();
    session.send("?");
    session.wait_for_output_since(checkpoint, "Ctrl+S saves from Review");
    session.send(common::ESC);
    wait_screen(&session, |screen| screen == baseline);
    for (rows, cols) in [(24, 80), (16, 60), (10, 40)] {
        let checkpoint = session.output_checkpoint();
        session.resize(rows, cols);
        let expected = if cols < 60 {
            "Resize to at least 60x16"
        } else {
            "Filter: test"
        };
        session.wait_for_output_since(checkpoint, expected);
        session.send("?");
        session.wait_for("Help");
        session.send(common::ESC);
        session.wait_for(expected);
        if cols < 60 {
            session.send("\x13");
            session.send(common::ENTER);
            session.send(" ");
            session.send("blocked");
            session.send("?");
            session.wait_for(" Help ");
            let checkpoint = session.output_checkpoint();
            session.send(common::ESC);
            session.wait_for_output_since(checkpoint, expected);
        }
        assert_eq!(std::fs::read(&path).unwrap(), SOURCE.as_bytes());
    }
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Filter: test");
    wait_screen(&session, |screen| screen == baseline);
    let checkpoint = session.output_checkpoint();
    session.send("\x1b[A"); // Up in details; scrolling must still work.
    session.send("?");
    session.wait_for_output_since(checkpoint, "Ctrl+S saves from Review");
    session.send(common::ESC);
    press(&mut expected_app, KeyCode::Up);
    let unscrolled = expected_screen(&mut expected_app);
    wait_screen(&session, |screen| screen == unscrolled);
    assert_ne!(unscrolled, baseline);
    session.send("\t");
    session.send(common::ENTER);
    session.wait_for(" Review ");
    press(&mut expected_app, KeyCode::Tab);
    press(&mut expected_app, KeyCode::Enter);
    let review = expected_screen(&mut expected_app);
    wait_screen(&session, |screen| screen == review);
    let checkpoint = session.output_checkpoint();
    session.resize(10, 40);
    session.wait_for_output_since(checkpoint, "Resize to at least 60x16");
    session.send("\x13"); // Save is blocked at Review too.
    session.send(common::ENTER);
    // Drain the blocked keys before the next resize reaches the event queue.
    session.send("?");
    session.wait_for(" Help ");
    let checkpoint = session.output_checkpoint();
    session.send(common::ESC);
    session.wait_for_output_since(checkpoint, "Resize to at least 60x16");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, " Review ");
    wait_screen(&session, |screen| screen == review);
    assert_eq!(std::fs::read(&path).unwrap(), SOURCE.as_bytes());
    session.send("\x13");
    session.wait_for("Saved. Existing projects are unchanged.");
    let saved = std::fs::read(&path).unwrap();
    let graph: crate::graph::ProjectGraph =
        toml::from_str(std::str::from_utf8(&saved).unwrap()).unwrap();
    assert!(!graph.capabilities.contains_key("test"));
    assert_eq!(graph.mode, "like:original");
    assert_eq!(graph.dropped, ["future-file"]);
    session.send("\x13");
    session.wait_for("No changes to save.");
    session.send(common::ESC);
    session.wait_for("Actions");
    session.send("\x03");
    session.wait_for("Manager returned");
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(std::fs::read(&path).unwrap(), saved);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn pty_undersized_dirty_editor_can_cancel_and_confirm_back_and_home() {
    use super::common;
    for home in [false, true] {
        let (root, _app) = app();
        let mut session = common::Session::start_program(
            &std::env::current_exe().unwrap(),
            root.path(),
            &[
                "--exact",
                "commands::saved_setups::tests::setup_pty_driver",
                "--nocapture",
                "--test-threads=1",
            ],
            &[
                ("RPROJ_SETUP_TEST_ROOT", root.path().to_str().unwrap()),
                ("RPROJ_NO_LOG", "1"),
            ],
        );
        session.wait_for("Filter:");
        let checkpoint = session.output_checkpoint();
        session.resize(30, 120);
        session.wait_for_output_since(checkpoint, "Filter:");
        session.send(common::ENTER);
        session.wait_for("Actions");
        session.send(common::ENTER);
        session.wait_for("Edit Saved Setup");
        for _ in 0..3 {
            session.send(common::DOWN);
        }
        session.send(common::ENTER);
        session.wait_for("[x] test");
        session.send("test");
        session.wait_for("Filter: test");
        session.send(" ");
        session.wait_for("[ ] test");
        session.send(common::ENTER);
        session.wait_for(" Review ");
        let checkpoint = session.output_checkpoint();
        session.resize(10, 40);
        session.wait_for_output_since(checkpoint, "Resize to at least 60x16");
        let exit = if home { "\x03" } else { common::ESC };
        session.send(exit);
        session.wait_for("[No]");
        session.send(common::ENTER);
        session.wait_for("Resize to at least 60x16");
        session.send(exit);
        session.wait_for("[No]");
        session.send("\x1b[C");
        session.wait_for("[Yes]");
        session.send("n");
        session.wait_for("Resize to at least 60x16");
        // Cancel twice, then verify the dirty draft survived before discarding.
        let checkpoint = session.output_checkpoint();
        session.resize(30, 120);
        session.wait_for_output_since(checkpoint, " Review ");
        for _ in 0..3 {
            session.send(common::DOWN);
        }
        session.send(common::ENTER);
        session.wait_for("[ ] test");
        session.send(common::ESC); // Cancel revision back to the dirty Review.
        session.wait_for(" Review ");
        let checkpoint = session.output_checkpoint();
        session.resize(10, 40);
        session.wait_for_output_since(checkpoint, "Resize to at least 60x16");
        session.send(exit);
        session.wait_for("[No]");
        session.send("y");
        session.wait_for("[Yes]");
        session.send(common::ENTER);
        if !home {
            let checkpoint = session.output_checkpoint();
            session.resize(30, 120);
            session.wait_for_output_since(checkpoint, "Actions");
            session.send(common::ENTER);
            session.wait_for("Edit Saved Setup");
            for _ in 0..3 {
                session.send(common::DOWN);
            }
            session.send(common::ENTER);
            session.wait_for("[x] test");
            session.send(common::ESC);
            session.wait_for(" Review ");
            session.send("\x03");
        }
        session.wait_for("Manager returned");
        let outcome = session.finish();
        assert_eq!(outcome.code, 0, "home={home}: {}", outcome.text);
        assert_eq!(
            std::fs::read(root.path().join("sample.toml")).unwrap(),
            SOURCE.as_bytes()
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

#[test]
fn pty_resize_preserves_filtered_selection_scrolled_details_and_back() {
    use super::common;
    let (root, mut expected_app) = app();
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
    // ConPTY can deliver the end marker before clearing the previous Actions pane.
    // Derive expected rows from TestBackend instead of capturing a partial frame.
    expected_app.refresh(None);
    for ch in "sample".chars() {
        press(&mut expected_app, KeyCode::Char(ch));
    }
    press(&mut expected_app, KeyCode::End);
    press(&mut expected_app, KeyCode::Tab);
    expected_screen(&mut expected_app);
    press(&mut expected_app, KeyCode::End);
    let expected = expected_screen(&mut expected_app);
    let details = tui::responsive_panes(ratatui::layout::Rect::new(0, 2, 120, 25), 40)[1];
    let composition = |screen: &str| {
        screen
            .lines()
            .skip(details.y as usize)
            .take(details.height as usize)
            .map(|line| {
                line.chars()
                    .skip(details.x as usize)
                    .take(details.width as usize)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
    };
    let baseline = composition(&expected);
    let mut session = common::Session::start_program(
        &std::env::current_exe().unwrap(),
        root.path(),
        &[
            "commands::saved_setups::tests::setup_pty_driver",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ],
        &[
            ("RPROJ_SETUP_TEST_ROOT", root.path().to_str().unwrap()),
            ("RPROJ_NO_LOG", "1"),
        ],
    );
    session.wait_for("Filter:");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Filter:");
    session.send("sample");
    // ConPTY can deliver text as individual keys; finish filtering before navigating.
    session.wait_for("Filter: sample ");
    session.send("\x1b[F"); // End in the setup list
    session.wait_for("sample29.toml");
    session.send("\t");
    session.send("\x1b[F"); // End in composition details
    session.wait_for("Recovery detail 39");
    wait_screen(&session, |screen| composition(screen) == baseline);
    let scrolled = session.text();
    assert!(!scrolled.contains("Future reuse only"), "{scrolled}");
    assert!(!scrolled.contains("other"), "{scrolled}");
    assert!(!scrolled.contains("sample00"), "{scrolled}");

    for (rows, cols) in [(24, 80), (16, 60), (10, 40)] {
        let checkpoint = session.output_checkpoint();
        session.resize(rows, cols);
        let expected = if cols < 60 {
            "Resize to at least 60x16"
        } else {
            "Filter: sample "
        };
        session.wait_for_output_since(checkpoint, expected);
        if cols >= 60 {
            let list =
                tui::responsive_panes(ratatui::layout::Rect::new(0, 2, cols, rows - 5), 40)[0];
            // A fresh filter update can precede the list redraw in ConPTY's diff stream.
            let screen = wait_screen(&session, |screen| {
                screen
                    .lines()
                    .skip((list.y + 1) as usize)
                    .take((list.height - 2) as usize)
                    .any(|line| {
                        line.chars()
                            .skip((list.x + 1) as usize)
                            .take((list.width - 2) as usize)
                            .collect::<String>()
                            .starts_with("sample29")
                    })
            });
            assert!(!screen.contains("other"), "{screen}");
        }
        session.send("?");
        session.wait_for("Help | Esc close");
        session.send(common::ESC);
        session.wait_for(expected);
    }
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Recovery detail 39");
    let restored = session.text();
    // List offsets may adapt to the viewport; composition must return unchanged.
    wait_screen(&session, |screen| composition(screen) == baseline);
    assert!(
        session
            .screen()
            .rows(1, details.x - 2)
            .skip(3)
            .take(23)
            .any(|line| line.starts_with("sample29")),
        "{restored}"
    );
    session.send("\x1b[H"); // Home in composition details
    session.wait_for("Future reuse only");
    let checkpoint = session.output_checkpoint();
    session.send("\x1b[F");
    session.wait_for_output_since(checkpoint, "Recovery detail 39");
    wait_screen(&session, |screen| composition(screen) == baseline);
    session.send("\t");
    session.send(common::ENTER);
    session.wait_for("Actions");
    session.wait_for("sample29.toml");
    let checkpoint = session.output_checkpoint();
    session.send(common::ESC);
    session.wait_for_output_since(checkpoint, "Filter: sample ");
    session.wait_for("Recovery detail 39");
    wait_screen(&session, |screen| composition(screen) == baseline);
    session.send("\x03");
    session.wait_for("Manager returned");
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 32);
    for n in 0..30 {
        assert_eq!(
            std::fs::read_to_string(root.path().join(format!("sample{n:02}.toml"))).unwrap(),
            source
        );
    }
    for name in ["sample", "other"] {
        assert_eq!(
            std::fs::read_to_string(root.path().join(format!("{name}.toml"))).unwrap(),
            SOURCE
        );
    }
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
            "--test-threads=1",
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
