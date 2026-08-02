//! `rproj upgrade` end to end, against hand-built fixture projects.
//!
//! No scaffolding and no network: `upgrade` only rewrites generated config,
//! so a directory holding `default.project.json`, `rproj.toml` and whatever
//! config is being tested is a complete and honest fixture. That keeps this
//! in the default `cargo test` run, where the regressions it guards against
//! actually get caught.

mod common;

use common::{Session, TempProject};

/// The minimum that makes a directory an rproj project: the file `upgrade`
/// uses to recognise one, and the manifest it reads the composition from.
fn fixture(label: &str, packages: &str) -> TempProject {
    let project = TempProject::new(label);
    project.write("default.project.json", "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n");
    project.write(
        "rproj.toml",
        &format!(
            "mode = \"expert\"\npackage_workflow = \"wally\"\npackages = [{packages}]\ntools_at_creation = [\"selene\", \"stylua\"]\n"
        ),
    );
    project
}

/// The whole reason the command exists: a project scaffolded before the
/// Vide fix keeps `mixed_table = "warn"`, which fails its own quality gate
/// on every UI file, and nothing tells it otherwise.
#[test]
fn a_stale_selene_config_is_brought_up_to_date() {
    let project = fixture("selene-stale", "\"vide\", \"testez\"");
    project.write(
        "selene.toml",
        "std = \"roblox\"\n\n[rules]\nunused_variable = \"allow\"\nmixed_table = \"warn\"\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);

    let selene = project.read("selene.toml");
    assert!(selene.contains(r#"mixed_table = "allow""#), "vide waiver missing:\n{selene}");
    assert!(selene.contains(r#"std = "roblox+testez""#), "testez std missing:\n{selene}");
    assert!(selene.contains(r#"exclude = ["Packages/**"#), "vendored exclude missing:\n{selene}");
    // The one that makes this safe to run: a lint level the user chose
    // themselves is not a thing rproj derives, so it must survive.
    assert!(selene.contains(r#"unused_variable = "allow""#), "clobbered a user choice:\n{selene}");
}

/// A project with no Vide keeps the lint - the waiver is not a blanket one.
#[test]
fn a_project_without_a_create_style_ui_library_keeps_the_lint() {
    let project = fixture("selene-nonvide", "\"charm\"");
    project.write("selene.toml", "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n");

    assert_eq!(Session::start(project.path(), &["upgrade", "--yes"]).finish().code, 0);
    assert!(project.read("selene.toml").contains(r#"mixed_table = "warn""#));
}

/// Deprecated settings rproj itself wrote have to go, or the file rproj
/// manages keeps a deprecation squiggle rproj put there.
#[test]
fn deprecated_editor_settings_are_replaced_not_merely_supplemented() {
    let project = fixture("vscode-deprecated", "\"charm\"");
    project.write(
        ".vscode/settings.json",
        "{\n  \"editor.rulers\": [100],\n  \"luau-lsp.plugin.enabled\": true,\n  \"luau-lsp.types.roblox\": true\n}\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);

    let settings = project.read(".vscode/settings.json");
    assert!(settings.contains(r#""luau-lsp.studioPlugin.enabled": true"#), "{settings}");
    assert!(!settings.contains(r#""luau-lsp.plugin.enabled""#), "deprecated key kept:\n{settings}");
    assert!(!settings.contains(r#""luau-lsp.types.roblox""#), "deprecated key kept:\n{settings}");
    assert!(settings.contains(r#""files.eol": "\n""#), "{settings}");
    assert!(settings.contains(r#""editor.rulers""#), "unrelated key lost:\n{settings}");
}

/// Running it twice must do nothing the second time, or it churns git
/// history and nobody can tell a real change from a re-run.
#[test]
fn a_second_run_reports_nothing_to_do() {
    let project = fixture("idempotent", "\"vide\"");
    project.write("selene.toml", "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n");

    assert_eq!(Session::start(project.path(), &["upgrade", "--yes"]).finish().code, 0);
    let after_first = project.read("selene.toml");

    let second = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(second.code, 0, "{}", second.text);
    second.assert_contains("already up to date");
    assert_eq!(project.read("selene.toml"), after_first, "second run rewrote the file");
}

/// Files the user owns are seeded once and then edited by hand; rewriting
/// them from a template would throw away real work.
#[test]
fn files_the_user_owns_are_left_alone() {
    let project = fixture("hands-off", "\"vide\"");
    project.write("selene.toml", "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n");
    let stylua = "column_width = 80\nindent_type = \"Spaces\"\n";
    project.write("stylua.toml", stylua);
    let manifest = "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n";

    assert_eq!(Session::start(project.path(), &["upgrade", "--yes"]).finish().code, 0);

    assert_eq!(project.read("stylua.toml"), stylua, "stylua.toml is the user's");
    assert_eq!(project.read("default.project.json"), manifest, "the project file is the user's");
}

/// Without `rproj.toml` the package list is unknown, and guessing it from
/// what's on disk would be guessing.
#[test]
fn a_project_with_no_manifest_is_refused_with_a_reason() {
    let project = TempProject::new("no-manifest");
    project.write("default.project.json", "{}\n");

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("no rproj.toml");
}

/// Run from the wrong directory it should say so, not half-upgrade
/// something that isn't a project.
#[test]
fn a_directory_that_is_not_a_project_is_refused() {
    let project = TempProject::new("not-a-project");

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("no default.project.json");
}

/// **The graph is what upgrade re-derives from.** A project that never
/// chose CI must not acquire a workflow because a newer rproj knows how to
/// generate one - "picks up fixes made since" is not "picks up decisions you
/// declined".
///
/// Before `rproj.toml` recorded decisions there was nothing to ask: upgrade
/// rewrote whatever it could render, so every project got every file the
/// current version knew about.
#[test]
fn upgrade_does_not_restore_a_capability_the_project_declined() {
    let project = TempProject::new("declined-ci");
    project.write("default.project.json", "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n");
    // Lint and the gate, but no `ci` - and no `.github/` on disk.
    project.write(
        "rproj.toml",
        "mode = \"expert\"\npackage_workflow = \"none\"\npackages = []\n\n\
         [capabilities]\nlint = \"selene\"\ngate = \"lute\"\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);

    assert!(project.exists(".lute/check.luau"), "the gate was chosen:\n{}", outcome.text);
    assert!(
        !project.exists(".github/workflows/ci.yml"),
        "CI was never chosen and must not appear:\n{}",
        outcome.text
    );
}

/// A file dropped at the summary stays dropped. `dropped` is part of the
/// graph precisely so an upgrade cannot helpfully undo a deliberate removal.
#[test]
fn upgrade_respects_files_dropped_at_creation() {
    let project = TempProject::new("dropped-gate");
    project.write("default.project.json", "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n");
    project.write(
        "rproj.toml",
        "mode = \"expert\"\npackage_workflow = \"none\"\npackages = []\n\
         dropped = [\".lute/check.luau\"]\n\n\
         [capabilities]\ngate = \"lute\"\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert!(
        !project.exists(".lute/check.luau"),
        "a dropped file must not come back:\n{}",
        outcome.text
    );
}
