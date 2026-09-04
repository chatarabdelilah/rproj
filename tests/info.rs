//! `rproj info` end to end: the real binary in a real terminal.

mod common;

use std::process::{Command, Stdio};

use common::{ENTER, ESC, Session, TempProject};

/// Section menu, entry list, detail page, and back out again.
///
/// Navigates by *filtering* rather than counting arrow presses: the order of
/// the catalog is not this test's business, and a test that fails when a
/// package is added is a test nobody keeps.
#[test]
fn the_browser_navigates_to_a_detail_page_and_back_out() {
    let project = TempProject::new("info-browse");
    let mut session = Session::start(project.path(), &["info"]);

    session.wait_for("rproj catalog");
    session.send("Generated");
    session.wait_for("Generated files");
    session.send(ENTER);

    // The section stays open across lookups, so this is one keystroke per
    // entry rather than a round trip through the section menu.
    session.wait_for("Type to filter");
    session.send("wally.toml");
    session.wait_for("wally.toml");
    session.send(ENTER);

    // The detail page answers the question the artifact model created:
    // *why* does my project have this file? And it answers it by naming a
    // cause the user can act on, not by describing a mechanism.
    session.wait_for("when this project uses Wally");
    let screen = session.text();
    assert!(screen.contains("wally.toml"), "{screen}");
    assert!(screen.contains("Wally manifest"), "{screen}");

    // Escape backs out through detail, entries, and sections.
    session.send(ESC);
    session.wait_for("Generated files");
    session.send(ESC);
    session.wait_for("Sections");
    session.send(ESC);

    let outcome = session.finish();
    assert_eq!(
        outcome.code, 0,
        "leaving the browser is a clean exit:\n{}",
        outcome.text
    );
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
