mod common;

use std::process::{Command, Stdio};

use common::{ENTER, ESC, Session, TempProject};

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
    for _ in 0..3 {
        session.send(common::DOWN);
    }
    session.send(ENTER);
    session.wait_for("System apps");
    session.send(ESC);
    session.wait_for("Cancelled.");
    session.wait_for("Press Enter to return Home.");
    session.send(ENTER);
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
    for _ in 0..2 {
        session.send(common::DOWN);
    }
    for _ in 0..2 {
        session.send(ENTER);
        session.wait_for("fixture child running");
        session.send("\x03");
        session.wait_for("Stopped.");
        session.wait_for("Press Enter to return the project.");
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
    for _ in 0..3 {
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
fn the_hub_opens_the_catalog_and_returns() {
    let project = TempProject::new("hub-catalog");
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    for _ in 0..4 {
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
