mod common;

use std::process::{Command, Stdio};

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

#[test]
fn interactive_bare_rproj_prints_usage_and_exits() {
    let project = common::TempProject::new("welcome");
    let result = common::Session::start(project.path(), &[]).finish();
    assert_eq!(result.code, 0, "{}", result.text);
    result.assert_contains("Commands");
    result.assert_contains("rproj app");
}
