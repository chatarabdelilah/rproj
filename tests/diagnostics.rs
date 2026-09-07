//! Real command outcomes and prompt decisions reach a private per-run text log.
mod common;

use common::{Session, TempProject};
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .env("RPROJ_LOG_DIR", root.join("logs"))
        .env("RPROJ_NO_LOG", "0")
        .env(
            "OPEN_CLOUD_API_KEY",
            "opaque-environment-value-do-not-record",
        )
        .output()
        .expect("run rproj")
}

fn log(root: &Path) -> String {
    let files: Vec<_> = std::fs::read_dir(root.join("logs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "one log per run: {files:?}");
    assert_eq!(files[0].extension().unwrap(), "txt");
    let text = std::fs::read_to_string(&files[0]).unwrap();
    assert!(!text.contains('\x1b'), "plain text, not terminal escapes");
    assert!(!text.contains("opaque-environment-value-do-not-record"));
    text
}

#[test]
fn successful_command_reports_a_plain_log_without_changing_stdout() {
    let root = TempProject::new("diagnostics-success");
    let output = run(root.path(), &["info", "stylua"]);
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("Diagnostic log:"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Diagnostic log:"));
    let text = log(root.path());
    assert!(
        text.contains("[command] info key=Some(\"stylua\")"),
        "{text}"
    );
    assert!(text.contains("[run.end] exit_code=0"), "{text}");
}

#[test]
fn runner_passthrough_and_environment_values_are_not_recorded() {
    let root = TempProject::new("diagnostics-passthrough");
    let output = run(
        root.path(),
        &["test", "--api-key", "opaque-argument-do-not-record"],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = log(root.path());
    assert!(
        text.contains("2 passthrough arguments (values omitted)"),
        "{text}"
    );
    assert!(!text.contains("opaque-argument-do-not-record"));
    assert!(text.contains("[error]"));
    assert!(text.contains("[run.end] exit_code=1"));
}

#[test]
fn invalid_cli_arguments_get_a_log_without_copying_raw_values() {
    let root = TempProject::new("diagnostics-parse");
    let output = run(
        root.path(),
        &["invalid-command", "opaque-argument-do-not-record"],
    );
    assert_eq!(output.status.code(), Some(2));
    let text = log(root.path());
    assert!(text.contains("[cli.invalid]"));
    assert!(!text.contains("opaque-argument-do-not-record"));
    assert!(text.contains("exit_code=2"));
}

#[test]
fn sensitive_named_values_are_redacted_in_command_and_error_events() {
    let root = TempProject::new("diagnostics-redaction");
    let output = run(
        root.path(),
        &["configure", "password=opaque-value-do-not-record"],
    );
    assert_eq!(output.status.code(), Some(1));
    let text = log(root.path());
    assert!(text.contains("[redacted: potentially sensitive value]"));
    assert!(!text.contains("opaque-value-do-not-record"));
}

#[test]
fn tui_navigation_is_logged_without_recording_filter_keystrokes() {
    let root = TempProject::new("diagnostics-tui");
    let log_dir = root.path().join("logs");
    let mut session = Session::start_with_env(
        root.path(),
        &["info"],
        &[
            ("RPROJ_LOG_DIR", log_dir.to_str().unwrap()),
            ("RPROJ_NO_LOG", "0"),
        ],
    );
    session.wait_for("Catalog");
    session.send("Packages");
    session.send(common::ENTER);
    session.wait_for("charm");
    session.send("opaque-filter-do-not-record");
    session.send(common::ENTER);
    session.send(common::ESC);
    session.wait_for("Sections");
    session.send(common::ESC);
    let result = session.finish();
    assert_eq!(result.code, 0, "{}", result.text);
    let text = log(root.path());
    assert!(text.contains("[tui.enter]"), "{text}");
    assert!(text.contains("[catalog.open]"), "{text}");
    assert!(text.contains("[catalog.choice] Packages"), "{text}");
    assert!(text.contains("[tui.leave]"), "{text}");
    assert!(!text.contains("opaque-filter-do-not-record"));
}

#[test]
fn unavailable_log_directory_does_not_fail_the_command() {
    let root = TempProject::new("diagnostics-unavailable");
    root.write("logs", "existing file, not a directory");
    let output = run(root.path(), &["info", "stylua"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("log unavailable"));
    assert_eq!(root.read("logs"), "existing file, not a directory");
}

#[test]
fn logging_can_be_disabled_without_creating_files() {
    let root = TempProject::new("diagnostics-disabled");
    let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(["info", "stylua"])
        .current_dir(root.path())
        .env("RPROJ_LOG_DIR", root.path().join("logs"))
        .env("RPROJ_NO_LOG", "1")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!root.exists("logs"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Diagnostic log:"));
}

#[test]
fn interactive_setting_choices_are_logged_with_their_prompt_names() {
    let root = TempProject::new("diagnostics-choices");
    let log_dir = root.path().join("logs");
    let mut session = Session::start_with_env(
        root.path(),
        &["configure", "stylua"],
        &[
            ("RPROJ_LOG_DIR", log_dir.to_str().unwrap()),
            ("RPROJ_NO_LOG", "0"),
        ],
    );
    session.enter_through(&[
        "syntax",
        "column_width",
        "indent_type",
        "indent_width",
        "quote_style",
        "call_parentheses",
        "collapse_simple_statement",
        "line_endings",
        "sort_requires.enabled",
    ]);
    let result = session.finish();
    assert_eq!(result.code, 0, "{}", result.text);
    let text = log(root.path());
    assert!(text.contains("[prompt.setting] indent_type"), "{text}");
    assert!(
        text.contains("[choice.setting] indent_type=\"Tabs\""),
        "{text}"
    );
    assert!(text.contains("[choice.setting] column_width=120"), "{text}");
    assert!(text.contains("[run.end] exit_code=0"), "{text}");
}
