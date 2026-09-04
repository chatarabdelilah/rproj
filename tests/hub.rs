mod common;

use std::process::{Command, Stdio};

use common::{ENTER, ESC, Session, TempProject};

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
    for _ in 0..7 {
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
    let mut session = Session::start(project.path(), &[]);
    session.wait_for("Tasks");
    for _ in 0..4 {
        session.send(common::DOWN);
    }
    session.send(ENTER);
    session.wait_for("Upgrade requires rproj.toml");
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
