//! `rproj info` end to end: the real binary in a real terminal.

mod common;

use std::process::{Command, Stdio};

#[cfg(windows)]
use common::{Session, TempProject};

#[cfg(windows)]
#[test]
fn conpty_collects_the_exit_status_of_a_silent_child() {
    let project = TempProject::new("silent-pty-child");
    let shell = std::env::var_os("ComSpec").expect("Windows command interpreter");
    let outcome = Session::start_program(
        std::path::Path::new(&shell),
        project.path(),
        &["/D", "/C", "exit", "0"],
        &[],
    )
    .finish();

    assert_eq!(outcome.code, 0);
    assert!(outcome.text.trim().is_empty(), "{}", outcome.text);
}

/// With no terminal on stdin, print the flat listing instead of prompting.
///
/// The failure this prevents is not a wrong answer, it is a hang: inquire
/// reads the console input handle rather than stdin, so a prompt with nothing
/// attached renders and then waits forever - `rproj info > notes.txt` would
/// never return, and neither would a CI step that ran it.
#[test]
fn without_a_terminal_it_lists_instead_of_prompting() {
    let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .arg("info")
        .stdin(Stdio::null())
        .output()
        .expect("run rproj info");

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in ["WALLY PACKAGES", "TOOLS", "GENERATED FILES", "TOPICS"] {
        assert!(text.contains(expected), "expected {expected:?} in:\n{text}");
    }
    assert!(!text.contains("PLACE TEMPLATE"));
    assert!(!text.contains("\x1b["));
    assert!(
        !text.contains("What do you want to look up?"),
        "must not try to prompt:\n{text}"
    );
}

/// A direct lookup still works and still exits, which is what every "try
/// next" hint in the rest of rproj points at.
#[test]
fn a_named_lookup_prints_one_entry_and_exits() {
    let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(["info", "rojo"])
        .stdin(Stdio::null())
        .output()
        .expect("run rproj info rojo");

    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("rojo\n----"), "{text}");
    assert!(
        text.contains("rojo serve"),
        "the commands are the point:\n{text}"
    );
    assert!(
        !text.contains("WALLY PACKAGES"),
        "one entry, not the catalog:\n{text}"
    );
}
