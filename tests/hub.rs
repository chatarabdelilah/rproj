mod common;

use std::process::{Command, Stdio};

use common::{ENTER, ESC, Session, TempProject, screen_rows};

#[test]
fn physical_screen_rows_ignore_soft_wrap_but_preserve_layout() {
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

    let mut moved = vt100::Parser::new(2, 4, 0);
    moved.process(b"abc\x1b[2;1Hde");
    assert_eq!(
        screen_rows(wrapped.screen()).replace('\n', ""),
        screen_rows(moved.screen()).replace('\n', "")
    );
    assert_ne!(screen_rows(wrapped.screen()), screen_rows(moved.screen()));
}

fn tool_path(project: &TempProject) -> String {
    use std::sync::OnceLock;
    static TOOL: OnceLock<tempfile::TempDir> = OnceLock::new();
    let dir = TOOL.get_or_init(|| {
        let dir = tempfile::tempdir().unwrap();
        let result = Command::new("rustc")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/foreground_tool.rs"
            ))
            .arg("-o")
            .arg(dir.path().join("tool.exe"))
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        dir
    });
    let bin = project.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for name in ["rojo", "rokit", "lute"] {
        std::fs::copy(dir.path().join("tool.exe"), bin.join(format!("{name}.exe"))).unwrap();
    }
    std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap()
    .to_string_lossy()
    .into_owned()
}

fn open_project(session: &mut Session, project: &TempProject) {
    session.send(ENTER);
    session.wait_for("Filter:");
    session.send(project.path().file_name().unwrap().to_str().unwrap());
    session.wait_for(project.path().file_name().unwrap().to_str().unwrap());
    session.wait_for(if project.exists("rproj.toml") {
        "Workflow:"
    } else {
        "Rojo project without rproj.toml."
    });
    session.send(ENTER);
    session.wait_for("Project actions");
}

#[test]
fn project_configure_saves_and_returns_without_a_terminal_handoff() {
    let project = TempProject::new("home-configure-tools");
    project.write("default.project.json", "{}");
    project.write("stylua.toml", "column_width = 91\ncustom = 'preserve'\n");
    let logs = project.path().join("logs");
    let mut session = Session::start_with_env(
        project.path(),
        &[],
        &[
            ("RPROJ_LOG_DIR", logs.to_str().unwrap()),
            ("RPROJ_NO_LOG", "0"),
        ],
    );
    session.wait_for("Tasks");
    open_project(&mut session, &project);
    session.send(common::DOWN);
    session.send(ENTER);
    session.wait_for("Search:");
    session.send("stylua");
    session.send(ENTER);
    session.wait_for("column_width = 91");
    session.send(common::DOWN); // syntax is first, column_width second
    session.send(ENTER);
    session.wait_for("Enter accepts");
    session.send("\x08\x08100");
    session.send(ENTER);
    session.send("\x13"); // Ctrl+S
    session.wait_for("Save these changes?");
    session.send("y");
    session.wait_for("Saved");
    session.send(ESC);
    session.wait_for("Search:");
    session.send(ESC);
    session.wait_for("Project actions");
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    let result = session.finish();
    assert_eq!(result.code, 0, "{}", result.text);
    let settings: toml::Value = toml::from_str(&project.read("stylua.toml")).unwrap();
    assert_eq!(settings["column_width"].as_integer(), Some(100));
    assert_eq!(settings["custom"].as_str(), Some("preserve"));
    let path = std::fs::read_dir(logs)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let log = std::fs::read_to_string(path).unwrap();
    assert_eq!(log.matches("[tui.enter]").count(), 1);
    assert_eq!(log.matches("[tui.leave]").count(), 1);
    assert!(!log.contains("[tui.suspend]"));
}

#[test]
fn project_upgrade_cancel_borrows_home_terminal_and_preserves_every_file() {
    for cancel in [ESC, "\x03"] {
        let project = TempProject::new("home-upgrade-cancel");
        project.write("default.project.json", "{}\n");
        project.write("rproj.toml", "package_workflow='none'\n");
        project.write("src/custom.luau", "return 'keep'\r\n");
        let before = common::project_snapshot(project.path());
        let logs = TempProject::new("home-upgrade-cancel-logs");
        let mut session = Session::start_with_env(
            project.path(),
            &[],
            &[
                ("RPROJ_LOG_DIR", logs.path().to_str().unwrap()),
                ("RPROJ_NO_LOG", "0"),
            ],
        );
        session.wait_for("Tasks");
        open_project(&mut session, &project);
        session.send(common::DOWN);
        session.send(common::DOWN);
        session.send(ENTER);
        session.wait_for("Upgrade review");
        assert_eq!(common::project_snapshot(project.path()), before);
        session.send(cancel);
        session.wait_for("Cancelled. No project changes were made.");
        if cancel == ESC {
            session.wait_for("Project actions");
            session.send("\x03");
        }
        session.wait_for("Tasks");
        session.send(ESC);
        let result = session.finish();
        assert_eq!(result.code, 0, "{}", result.text);
        assert_eq!(common::project_snapshot(project.path()), before);
        let path = std::fs::read_dir(logs.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let log = std::fs::read_to_string(path).unwrap();
        assert_eq!(log.matches("[tui.enter]").count(), 1);
        assert_eq!(log.matches("[tui.leave]").count(), 1);
        assert!(!log.contains("[tui.suspend]"), "{log}");
    }
}

#[test]
fn project_upgrade_applies_reviewed_plan_and_refuses_conflicts_before_returning() {
    for conflict in [false, true] {
        let project = TempProject::new("home-upgrade-apply");
        project.write("default.project.json", "{}\n");
        project.write("rproj.toml", "package_workflow='none'\n");
        project.write("src/custom.luau", "return 'keep'\r\n");
        let logs = TempProject::new("home-upgrade-apply-logs");
        let mut session = Session::start_with_env(
            project.path(),
            &[],
            &[
                ("RPROJ_LOG_DIR", logs.path().to_str().unwrap()),
                ("RPROJ_NO_LOG", "0"),
            ],
        );
        session.wait_for("Tasks");
        open_project(&mut session, &project);
        session.send(common::DOWN);
        session.send(common::DOWN);
        session.send(ENTER);
        session.wait_for("Upgrade review");
        session.send("a");
        session.wait_for("Apply these changes?");
        if conflict {
            project.write(".gitignore", "external edit\n");
        }
        let before = common::project_snapshot(project.path());
        session.send("y");
        session.wait_for(if conflict {
            "changed while upgrade was being reviewed"
        } else {
            "wrote .luaurc"
        });
        session.wait_for("Press Enter to return");
        if conflict {
            assert_eq!(common::project_snapshot(project.path()), before);
        } else {
            assert!(project.exists(".gitignore"));
            assert!(project.exists(".luaurc"));
            assert_eq!(project.read("src/custom.luau"), "return 'keep'\r\n");
        }
        session.send(ENTER);
        session.wait_for("Project actions");
        if !conflict {
            // Upgrade remains selected, and an empty plan bypasses the viewer.
            session.send(ENTER);
            session.wait_for("already up to date");
            session.wait_for("Press Enter to return");
            session.send(ENTER);
            session.wait_for("Project actions");
        }
        session.send("\x03");
        session.wait_for("Tasks");
        session.send(ESC);
        let result = session.finish();
        assert_eq!(result.code, 0, "{}", result.text);
        let path = std::fs::read_dir(logs.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let log = std::fs::read_to_string(path).unwrap();
        let expected_handoffs = if conflict { 1 } else { 2 };
        assert_eq!(log.matches("[tui.enter]").count(), 1);
        assert_eq!(log.matches("[tui.leave]").count(), 1);
        assert_eq!(log.matches("[tui.suspend]").count(), expected_handoffs);
        assert_eq!(log.matches("[tui.resume]").count(), expected_handoffs);
    }
}

#[test]
fn saved_setups_returns_home_without_a_terminal_handoff() {
    let project = TempProject::new("home-saved-setups");
    let logs = project.path().join("logs");
    let mut session = Session::start_with_env(
        project.path(),
        &[],
        &[
            ("RPROJ_LOG_DIR", logs.to_str().unwrap()),
            ("RPROJ_NO_LOG", "0"),
        ],
    );
    session.wait_for("Tasks");
    session.send(common::DOWN);
    session.send(common::DOWN);
    session.send(ENTER);
    session.wait_for("F5 refresh");
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
    let path = std::fs::read_dir(logs)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let log = std::fs::read_to_string(path).unwrap();
    assert_eq!(log.matches("[tui.enter]").count(), 1);
    assert_eq!(log.matches("[tui.leave]").count(), 1);
    assert!(!log.contains("[tui.suspend]"));
}

#[test]
fn template_cancel_and_catalog_share_one_terminal_session() {
    let project = TempProject::new("home-template");
    let logs = project.path().join("logs");
    let mut session = Session::start_with_env(
        project.path(),
        &[],
        &[
            ("RPROJ_LOG_DIR", logs.to_str().unwrap()),
            ("RPROJ_NO_LOG", "0"),
        ],
    );
    session.wait_for("Tasks");
    session.send(common::DOWN);
    session.send(common::DOWN);
    session.send(common::DOWN);
    session.send(ENTER);
    session.wait_for("rproj project template");
    session.send(ESC);
    session.wait_for("Completed.");
    session.send(ENTER);
    session.wait_for("rproj project template");
    session.send("\x03");
    session.wait_for("Completed.");
    session.send(ESC);
    let result = session.finish();
    assert_eq!(result.code, 0);
    assert!(!result.text.contains("Diagnostic log:"));
    let path = std::fs::read_dir(logs)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let log = std::fs::read_to_string(path).unwrap();
    assert_eq!(log.matches("[tui.enter]").count(), 1);
    assert_eq!(log.matches("[tui.leave]").count(), 1);
    assert!(!log.contains("[tui.suspend]"));
}

#[test]
fn cancelled_setup_returns_home_without_writing_configuration() {
    let project = TempProject::new("home-setup-cancel");
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    for _ in 0..4 {
        session.send(common::DOWN);
    }
    session.send(ENTER);
    session.wait_for("System Apps");
    session.send(ESC);
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
}

#[test]
#[cfg(windows)]
fn foreground_watch_interrupt_and_failure_return_to_a_usable_home() {
    let project = TempProject::new("home-watch");
    project.write("default.project.json", "{}");
    project.write("hold-tool", "");
    let path = tool_path(&project);
    let mut session = Session::start_with_env(project.path(), &[], &[("PATH", &path)]);
    session.wait_for("Tasks");
    open_project(&mut session, &project);
    for _ in 0..9 {
        session.send(common::DOWN);
    }
    for attempt in 1..=2 {
        let marker = format!("watch-attempt-{attempt}");
        project.write("hold-tool", &marker);
        session.send(ENTER);
        session.wait_for(&format!("fixture child running {marker}"));
        // ConPTY can repaint the previous acknowledgement when leaving the alternate screen.
        let checkpoint = session.output_checkpoint();
        session.send("\x03");
        session.wait_for_output_since(checkpoint, "Stopped.");
        session.wait_for_output_since(checkpoint, "Press Enter to return the project.");
        assert!(
            std::fs::File::open(project.path().join("active-child.lock")).is_ok(),
            "child must have exited"
        );
        session.send(ENTER);
        session.wait_for("Project actions");
    }
    std::fs::remove_file(project.path().join("hold-tool")).unwrap();
    project.write("fail-tool", "");
    session.send(ENTER);
    session.wait_for("Rojo watcher failed");
    session.wait_for("Press Enter to return the project.");
    session.send(ENTER);
    session.wait_for("Project actions");
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
}

#[test]
#[cfg(windows)]
fn interrupted_tool_restore_never_starts_the_test_runner() {
    let project = TempProject::new("home-tool-cancel");
    project.write(
        "rproj.toml",
        "package_workflow = \"none\"\n[capabilities]\ntest = \"testez\"\n",
    );
    project.write("rokit.toml", "[tools]\n");
    project.write("hold-tool", "");
    let path = tool_path(&project);
    let mut session = Session::start_with_env(project.path(), &[], &[("PATH", &path)]);
    session.wait_for("Tasks");
    open_project(&mut session, &project);
    for _ in 0..4 {
        session.send(common::DOWN);
    }
    session.send(ENTER);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while !project.exists("active-child.lock") {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    session.send("\x03");
    session.wait_for("Cancelled.");
    session.wait_for("Press Enter to return the project.");
    assert_eq!(project.read("tools.log").trim(), "rokit");
    assert!(std::fs::File::open(project.path().join("active-child.lock")).is_ok());
    session.send(ENTER);
    session.wait_for("Project actions");
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
}

#[test]
fn bare_rproj_opens_the_hub_and_exits_cleanly() {
    let project = TempProject::new("hub-exit");
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    session.send(ESC);
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
}

#[test]
fn projects_resize_recovery_preserves_filtered_details_and_navigation() {
    let project = TempProject::new("projects-pty-resize");
    project.write("default.project.json", "{}");
    let packages: Vec<_> = (0..40)
        .map(|n| format!("resize-fixture-package-{n:02}"))
        .collect();
    project.write(
        "rproj.toml",
        &format!(
            "package_workflow = \"none\"\npackages = {}\n[capabilities]\n",
            toml::Value::try_from(packages).unwrap()
        ),
    );
    let filter = project.path().file_name().unwrap().to_str().unwrap();
    let mut session = Session::start_with_env(project.path(), &[], &[("RPROJ_NO_LOG", "1")]);
    session.wait_for("Tasks");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Tasks");
    session.send(ENTER);
    session.wait_for("Filter:");
    session.send(filter);
    session.wait_for(&format!("Filter: {filter}"));
    session.wait_for("Workflow: None");
    session.send("\t");
    session.send("\x1b[F"); // End in the details pane
    session.wait_for("resize-fixture-package-39");
    let scrolled = screen_rows(&session.screen());
    assert!(!scrolled.contains("Workflow: None"), "{scrolled}");

    for (rows, cols) in [(24, 80), (16, 60), (10, 40)] {
        let checkpoint = session.output_checkpoint();
        session.resize(rows, cols);
        let restored = if cols < 60 {
            "Resize to at least 60 x 16.".to_string()
        } else {
            format!("Filter: {filter}")
        };
        session.wait_for_output_since(checkpoint, &restored);
        session.send("?");
        session.wait_for("Help");
        session.send(ESC);
        session.wait_for(&restored);
    }
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "resize-fixture-package-39");
    session.wait_screen(&scrolled);
    assert_eq!(screen_rows(&session.screen()), scrolled);
    session.send("\x1b[H"); // Home in the details pane
    session.wait_for("Workflow: None");
    session.send("\x1b[F");
    session.wait_screen(&scrolled);
    session.send("\t");
    session.send(ENTER);
    session.wait_for(&format!("Project: {filter}"));
    session.wait_for("Project actions");
    session.send(ESC);
    session.wait_for(&format!("Filter: {filter}"));
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
}

#[test]
fn the_hub_opens_the_catalog_and_returns() {
    let project = TempProject::new("hub-catalog");
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    for _ in 0..5 {
        session.send(common::DOWN);
    }
    session.send(ENTER);
    session.wait_for("rproj catalog");
    session.send(ESC);
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
}

#[test]
fn a_disabled_action_explains_itself_without_launching() {
    let project = TempProject::new("hub-disabled");
    project.write("default.project.json", "{}");
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    open_project(&mut session, &project);
    session.send(common::DOWN);
    session.send(common::DOWN);
    session.send(ENTER);
    session.wait_for("requires a valid rproj.toml");
    session.send("\x03");
    session.wait_for("Tasks");
    session.send(ESC);
    assert_eq!(session.finish().code, 0);
}

#[test]
fn redirected_bare_rproj_keeps_the_plain_welcome() {
    let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .stdin(Stdio::null())
        .output()
        .expect("run bare rproj");
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Commands"), "{text}");
    assert!(
        !text.contains("\x1b["),
        "plain output contained terminal escapes"
    );
}
