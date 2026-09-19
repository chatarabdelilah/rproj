mod common;

#[test]
fn redirected_machine_setup_refuses_without_terminal_controls() {
    let root = common::TempProject::new("setup-redirected");
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_rproj"))
        .arg("setup")
        .current_dir(root.path())
        .env("RPROJ_NO_LOG", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.contains("interactive terminal"));
    assert!(!output.contains('\u{1b}'));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn standalone_setup_cancel_restores_terminal() {
    let root = common::TempProject::new("setup-cancel");
    let mut session = common::Session::start(root.path(), &["setup"]);
    session.wait_for("Apply Setup");
    session.send(common::ESC);
    let result = session.finish();
    assert_eq!(result.code, 1);
    result.assert_contains("Operation cancelled");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
