//! `rproj configure` end to end: the real binary, real prompts, real files.
//!
//! Every test here exists because the command shipped broken in a way no
//! unit test could see. It re-rendered each config file from the catalog,
//! so it deleted the `exclude` key that keeps selene out of `Packages/`,
//! and it seeded every prompt from the catalog default rather than the
//! file, so pressing enter through it reverted the project's own settings
//! (`std = "roblox+testez"` back to plain `roblox`, among others). Both
//! were found by driving the command, not by reading it.

mod common;

use common::{Session, TempProject, DOWN, ENTER};

/// Exactly what `rproj new` writes for a Wally project that picked TestEZ.
/// Held as a literal rather than generated: the point is to test what
/// configure does to a file it didn't write, and `catalog::tool_settings`'
/// own unit tests already pin what the scaffold renders.
const SCAFFOLDED_SELENE: &str = r#"std = "roblox+testez"

exclude = ["Packages/**", "ServerPackages/**"]

[rules]
undefined_variable = "deny"
unused_variable = "warn"
shadowing = "warn"
global_usage = "warn"
incorrect_standard_library_use = "deny"
mixed_table = "warn"
multiple_statements = "allow"
roblox_incorrect_roact_usage = "deny"
"#;

const SELENE_PROMPTS: &[&str] = &[
    "std",
    "rules.undefined_variable",
    "rules.unused_variable",
    "rules.shadowing",
    "rules.global_usage",
    "rules.incorrect_standard_library_use",
    "rules.mixed_table",
    "rules.multiple_statements",
    "rules.roblox_incorrect_roact_usage",
];

const STYLUA_PROMPTS: &[&str] = &[
    "syntax",
    "column_width",
    "indent_type",
    "indent_width",
    "quote_style",
    "call_parentheses",
    "collapse_simple_statement",
    "line_endings",
    "sort_requires.enabled",
];

const VSCODE_STYLUA_PROMPTS: &[&str] = &["editor.formatOnSave", "stylua.searchParentDirectories"];

/// The headline guarantee: opening the walkthrough and agreeing with it
/// changes nothing. Byte-exact, so it also covers `exclude` surviving,
/// `std` keeping its TestEZ value, and the file's blank lines and key
/// order staying put.
#[test]
fn agreeing_with_every_prompt_leaves_the_file_byte_identical() {
    let project = TempProject::new("selene-noop");
    project.write("selene.toml", SCAFFOLDED_SELENE);

    let mut session = Session::start(project.path(), &["configure", "selene"]);
    session.enter_through(SELENE_PROMPTS);
    let outcome = session.finish();

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(project.read("selene.toml"), SCAFFOLDED_SELENE);
}

/// A changed answer has to land, without disturbing anything else -
/// including the keys the catalog knows nothing about.
#[test]
fn changing_one_lint_rewrites_only_that_line() {
    let project = TempProject::new("selene-change");
    project.write("selene.toml", SCAFFOLDED_SELENE);

    let mut session = Session::start(project.path(), &["configure", "selene"]);
    session.enter_through(&SELENE_PROMPTS[..3]);
    // `shadowing` is currently "warn", so the cursor starts there and one
    // step down is "allow".
    session.wait_for_prompt("rules.shadowing");
    session.send(&format!("{DOWN}{ENTER}"));
    session.enter_through(&SELENE_PROMPTS[4..]);
    let outcome = session.finish();

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let after = project.read("selene.toml");
    assert_eq!(after, SCAFFOLDED_SELENE.replace(r#"shadowing = "warn""#, r#"shadowing = "allow""#));
}

/// Failing after thirteen questions is the same failure plus wasted effort.
#[test]
fn an_unreadable_config_is_refused_before_a_single_question() {
    let project = TempProject::new("selene-broken");
    project.write("selene.toml", "std = \"roblox\n[rules]\n");

    let outcome = Session::start(project.path(), &["configure", "selene"]).finish();

    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("could not parse");
    outcome.assert_lacks("Value:");
}

/// A typo'd tool name should say what the real ones are.
#[test]
fn an_unknown_tool_is_refused_with_the_list_of_real_ones() {
    let project = TempProject::new("unknown-tool");

    let outcome = Session::start(project.path(), &["configure", "nosuchtool"]).finish();

    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("no configurable tool called `nosuchtool`");
    for key in ["stylua", "selene", "luau-lsp", "stylua-vscode"] {
        outcome.assert_contains(key);
    }
}

/// `rproj configure` with no argument: the picker offers everything, and
/// picking from it does the same work as naming the tool would have.
#[test]
fn the_tool_picker_offers_every_tool_and_runs_the_chosen_one() {
    let project = TempProject::new("tool-picker");

    let mut session = Session::start(project.path(), &["configure"]);
    session.wait_for("Which tool do you want to configure?");
    for key in ["stylua - StyLua", "selene - Selene", "luau-lsp - ", "stylua-vscode - "] {
        session.wait_for(key);
    }
    // StyLua is first in the list, so enter takes it.
    session.send(ENTER);
    session.enter_through(STYLUA_PROMPTS);
    let outcome = session.finish();

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let written = project.read("stylua.toml");
    assert!(written.contains(r#"syntax = "Luau""#), "{written}");
    assert!(written.contains("[sort_requires]"), "{written}");
}

/// `.vscode/settings.json` holds settings from every source the editor has,
/// so a write that isn't a merge throws away someone's editor preferences.
#[test]
fn vscode_settings_are_merged_not_replaced() {
    let project = TempProject::new("vscode-merge");
    project.write(".vscode/settings.json", "{\n  \"editor.tabSize\": 2\n}\n");

    let mut session = Session::start(project.path(), &["configure", "stylua-vscode"]);
    session.enter_through(VSCODE_STYLUA_PROMPTS);
    let outcome = session.finish();

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let written = project.read(".vscode/settings.json");
    assert!(written.contains(r#""editor.tabSize": 2"#), "unrelated setting lost:\n{written}");
    assert!(written.contains(r#""editor.formatOnSave": true"#), "{written}");
    assert!(written.contains(r#""stylua.searchParentDirectories": true"#), "{written}");
}

/// Turn something off, run the walkthrough again, agree with it: it has to
/// stay off. It didn't - every prompt proposed the catalog default, so a
/// second run silently turned formatting-on-save back on.
#[test]
fn a_second_run_proposes_what_you_chose_the_first_time() {
    let project = TempProject::new("vscode-remember");

    let mut first = Session::start(project.path(), &["configure", "stylua-vscode"]);
    first.wait_for_prompt("editor.formatOnSave");
    first.send(&format!("n{ENTER}"));
    first.enter_through(&VSCODE_STYLUA_PROMPTS[1..]);
    assert_eq!(first.finish().code, 0);
    assert!(
        project.read(".vscode/settings.json").contains(r#""editor.formatOnSave": false"#),
        "first run should have turned it off"
    );

    let mut second = Session::start(project.path(), &["configure", "stylua-vscode"]);
    second.wait_for_prompt("editor.formatOnSave");
    // inquire renders the default as the capitalised letter, so this is
    // also a check that the *prompt* changed, not just the written file.
    second.wait_for("(y/N)");
    second.enter_through(VSCODE_STYLUA_PROMPTS);
    let outcome = second.finish();

    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let written = project.read(".vscode/settings.json");
    assert!(written.contains(r#""editor.formatOnSave": false"#), "reverted:\n{written}");
}
