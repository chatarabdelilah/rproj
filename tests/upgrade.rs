//! `rproj upgrade` end to end, against hand-built fixture projects.
//!
//! No scaffolding and no network: `upgrade` only rewrites generated config,
//! so a directory holding `default.project.json`, `rproj.toml` and whatever
//! config is being tested is a complete and honest fixture. That keeps this
//! in the default `cargo test` run, where the regressions it guards against
//! actually get caught.

mod common;

use common::{ENTER, Session, TempProject};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_owned();
            if path.is_dir() {
                files.insert(relative, None);
                visit(root, &path, files);
            } else {
                files.insert(relative, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn edits_during_upgrade_confirmation_are_refused_before_any_write() {
    for mode in ["edit", "create", "delete"] {
        let project = fixture(&format!("upgrade-conflict-{mode}"), "\"vide\"");
        let original_selene = "std = \"roblox\"\n[rules]\nmixed_table = \"warn\"\n";
        project.write("selene.toml", original_selene);
        if mode != "create" {
            project.write(".vscode/settings.json", "{\"editor.rulers\":[100]}\n");
        }
        let mut session = Session::start(project.path(), &["upgrade"]);
        session.wait_for("Apply these changes?");
        let external = "{\"editor.rulers\":[80],\"custom\":{\"keep\":true}}\n";
        if mode == "delete" {
            fs::remove_file(project.path().join(".vscode/settings.json")).unwrap();
        } else {
            project.write(".vscode/settings.json", external);
        }
        let before = snapshot(project.path());
        session.send(ENTER);
        let outcome = session.finish();
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("changed while upgrade was being reviewed");
        assert_eq!(snapshot(project.path()), before);
        assert_eq!(project.read("selene.toml"), original_selene);
        assert!(!project.exists(".gitignore"));
        assert!(!project.exists(".luaurc"));
    }
}

#[test]
fn changed_upgrade_inputs_are_refused_even_when_they_are_not_rewritten() {
    for relative in ["rproj.toml", "default.project.json"] {
        let project = fixture("upgrade-input-conflict", "\"vide\"");
        let mut session = Session::start(project.path(), &["upgrade"]);
        session.wait_for("Apply these changes?");
        project.write(relative, &format!("{}\n", project.read(relative)));
        let before = snapshot(project.path());
        session.send(ENTER);
        let outcome = session.finish();
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("changed while upgrade was being reviewed");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn rejecting_or_cancelling_upgrade_keeps_every_file_and_directory_unchanged() {
    for answer in ["n\r", common::ESC] {
        let project = fixture("upgrade-cancel", "\"vide\", \"testez\"");
        project.write(
            "selene.toml",
            "std='roblox'\n[rules]\nunused_variable='allow'\n",
        );
        project.write(".vscode/settings.json", "{\"editor.rulers\":[100]}\n");
        project.write("src/custom.luau", "return 'keep'\n");
        let before = snapshot(project.path());
        let mut session = Session::start(project.path(), &["upgrade"]);
        session.wait_for("Apply these changes?");
        session.send(answer);
        let outcome = session.finish();
        if answer == "n\r" {
            assert_eq!(outcome.code, 0, "{}", outcome.text);
            outcome.assert_contains("nothing written");
        }
        outcome.assert_lacks("wrote ");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn confirmed_upgrade_preserves_custom_settings_and_user_owned_files() {
    let project = fixture("upgrade-confirm", "\"vide\", \"testez\"");
    project.write(
        "selene.toml",
        "# custom lint\nstd='roblox'\n[rules]\nunused_variable='allow'\nmixed_table='warn'\n",
    );
    project.write(
        ".vscode/settings.json",
        "{\"editor.rulers\":[100],\"custom\":{\"items\":[1,2]}}\n",
    );
    for relative in ["stylua.toml", "wally.toml", "rokit.toml", "src/custom.luau"] {
        project.write(relative, "user owned\r\n");
    }
    let record = project.read("rproj.toml");
    let production = project.read("default.project.json");
    let mut session = Session::start(project.path(), &["upgrade"]);
    session.wait_for("Apply these changes?");
    session.send(ENTER);
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let selene: toml::Value = toml::from_str(&project.read("selene.toml")).unwrap();
    assert_eq!(selene["std"].as_str(), Some("roblox+testez"));
    assert_eq!(selene["rules"]["mixed_table"].as_str(), Some("allow"));
    assert_eq!(selene["rules"]["unused_variable"].as_str(), Some("allow"));
    assert!(project.read("selene.toml").contains("# custom lint"));
    let settings: serde_json::Value =
        serde_json::from_str(&project.read(".vscode/settings.json")).unwrap();
    assert_eq!(settings["editor.rulers"], serde_json::json!([100]));
    assert_eq!(settings["custom"], serde_json::json!({"items":[1,2]}));
    assert_eq!(project.read("rproj.toml"), record);
    assert_eq!(project.read("default.project.json"), production);
    for relative in ["stylua.toml", "wally.toml", "rokit.toml", "src/custom.luau"] {
        assert_eq!(project.read(relative), "user owned\r\n");
    }
    assert!(project.exists(".gitignore"));
    assert!(project.exists(".luaurc"));
    assert!(project.exists("tests/.luaurc"));
}

#[test]
fn an_unreadable_target_is_not_treated_as_a_missing_file() {
    let project = fixture("upgrade-read-error", "\"vide\"");
    fs::create_dir_all(project.path().join(".vscode/settings.json")).unwrap();
    let before = snapshot(project.path());
    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("failed to read");
    assert_eq!(snapshot(project.path()), before);
}

#[test]
fn a_later_read_only_target_does_not_leave_earlier_files_upgraded() {
    let project = fixture("upgrade-readonly", "\"vide\"");
    project.write("selene.toml", "std='roblox'\n[rules]\nmixed_table='warn'\n");
    project.write(".vscode/settings.json", "{\"editor.rulers\":[100]}\n");
    let before = snapshot(project.path());
    let path = project.path().join(".vscode/settings.json");
    let original = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    let mut session = Session::start(project.path(), &["upgrade"]);
    session.wait_for("Apply these changes?");
    fs::set_permissions(&path, readonly).unwrap();
    session.send(ENTER);
    let outcome = session.finish();
    fs::set_permissions(&path, original).unwrap();
    assert_eq!(outcome.code, 1, "{}", outcome.text);
    assert_eq!(snapshot(project.path()), before);
    outcome.assert_lacks("wrote ");
}

#[cfg(windows)]
#[test]
fn locked_replacement_preserves_its_target_reports_progress_and_allows_retry() {
    use std::os::windows::fs::OpenOptionsExt;
    for (relative, saved) in [("selene.toml", 0), (".vscode/settings.json", 1)] {
        let project = fixture("upgrade-locked", "\"vide\"");
        project.write("selene.toml", "std='roblox'\n[rules]\nmixed_table='warn'\n");
        project.write(".vscode/settings.json", "{\"editor.rulers\":[100]}\n");
        let mut before = snapshot(project.path());
        let path = project.path().join(relative);
        // Reads/writes are shared, but Windows must deny file replacement.
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .unwrap();
        let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        drop(held);
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("failed to save");
        outcome.assert_contains(&format!("{saved} earlier upgrade files"));
        outcome.assert_contains("re-run `rproj upgrade`");
        let mut after = snapshot(project.path());
        if saved == 1 {
            assert_ne!(
                after.get(Path::new("selene.toml")),
                before.get(Path::new("selene.toml"))
            );
            after.remove(Path::new("selene.toml"));
            before.remove(Path::new("selene.toml"));
        }
        assert_eq!(
            after, before,
            "failed target, later files and temporary cleanup"
        );
        let retry = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(retry.code, 0, "{}", retry.text);
        let settings: serde_json::Value =
            serde_json::from_str(&project.read(".vscode/settings.json")).unwrap();
        assert_eq!(settings["editor.rulers"], serde_json::json!([100]));
    }
}

/// The minimum that makes a directory an rproj project: the file `upgrade`
/// uses to recognise one, and the manifest it reads the composition from.
fn fixture(label: &str, packages: &str) -> TempProject {
    let project = TempProject::new(label);
    project.write(
        "default.project.json",
        "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n",
    );
    let test_capability = if packages.contains("testez") {
        "test = \"testez\"\n"
    } else {
        ""
    };
    project.write(
        "rproj.toml",
        &format!(
            "mode = \"expert\"\npackage_workflow = \"wally\"\npackages = [{packages}]\n\n\
             [capabilities]\nlint = \"selene\"\nformat = \"stylua\"\neditor = \"vscode\"\n{test_capability}"
        ),
    );
    project
}

/// A project scaffolded before the Vide fix keeps `mixed_table = "warn"`,
/// which fails its own quality gate on every UI file.
#[test]
fn a_stale_selene_config_is_brought_up_to_date() {
    let project = fixture("selene-stale", "\"vide\", \"testez\"");
    project.write(
        "selene.toml",
        "std = \"roblox\"\nexclude = [\"Packages/**\", \"ServerPackages/**\", \"custom/**\"]\n\n[rules]\nunused_variable = \"allow\"\nmixed_table = \"warn\"\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);

    let selene = project.read("selene.toml");
    assert!(
        selene.contains(r#"mixed_table = "allow""#),
        "vide waiver missing:\n{selene}"
    );
    assert!(
        selene.contains(r#"std = "roblox+testez""#),
        "testez std missing:\n{selene}"
    );
    assert!(
        selene.contains(r#"exclude = ["Packages/**"#),
        "vendored exclude missing:\n{selene}"
    );
    assert!(selene.contains(r#""DevPackages/**""#), "{selene}");
    assert!(selene.contains(r#""ServerPackages/**""#), "{selene}");
    assert!(selene.contains(r#""custom/**""#), "{selene}");
    // The one that makes this safe to run: a lint level the user chose
    // themselves is not a thing rproj derives, so it must survive.
    assert!(
        selene.contains(r#"unused_variable = "allow""#),
        "clobbered a user choice:\n{selene}"
    );
}

/// A project with no Vide keeps the lint - the waiver is not a blanket one.
#[test]
fn a_project_without_a_create_style_ui_library_keeps_the_lint() {
    let project = fixture("selene-nonvide", "\"charm\"");
    project.write(
        "selene.toml",
        "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n",
    );

    assert_eq!(
        Session::start(project.path(), &["upgrade", "--yes"])
            .finish()
            .code,
        0
    );
    assert!(
        project
            .read("selene.toml")
            .contains(r#"mixed_table = "warn""#)
    );
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
    assert!(
        settings.contains(r#""luau-lsp.studioPlugin.enabled": true"#),
        "{settings}"
    );
    assert!(
        !settings.contains(r#""luau-lsp.plugin.enabled""#),
        "deprecated key kept:\n{settings}"
    );
    assert!(
        !settings.contains(r#""luau-lsp.types.roblox""#),
        "deprecated key kept:\n{settings}"
    );
    assert!(settings.contains(r#""files.eol": "\n""#), "{settings}");
    assert!(
        settings.contains(r#""editor.rulers""#),
        "unrelated key lost:\n{settings}"
    );
}

/// Running it twice must do nothing the second time, or it churns git
/// history and nobody can tell a real change from a re-run.
#[test]
fn a_second_run_reports_nothing_to_do() {
    let project = fixture("idempotent", "\"vide\"");
    project.write(
        "selene.toml",
        "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n",
    );

    assert_eq!(
        Session::start(project.path(), &["upgrade", "--yes"])
            .finish()
            .code,
        0
    );
    let after_first = project.read("selene.toml");

    let second = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(second.code, 0, "{}", second.text);
    second.assert_contains("already up to date");
    assert_eq!(
        project.read("selene.toml"),
        after_first,
        "second run rewrote the file"
    );
}

/// Files the user owns are seeded once and then edited by hand; rewriting
/// them from a template would throw away real work.
#[test]
fn files_the_user_owns_are_left_alone() {
    let project = fixture("hands-off", "\"vide\"");
    project.write(
        "selene.toml",
        "std = \"roblox\"\n\n[rules]\nmixed_table = \"warn\"\n",
    );
    let stylua = "column_width = 80\nindent_type = \"Spaces\"\n";
    project.write("stylua.toml", stylua);
    let manifest = "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n";

    assert_eq!(
        Session::start(project.path(), &["upgrade", "--yes"])
            .finish()
            .code,
        0
    );

    assert_eq!(
        project.read("stylua.toml"),
        stylua,
        "stylua.toml is the user's"
    );
    assert_eq!(
        project.read("default.project.json"),
        manifest,
        "the project file is the user's"
    );
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
    project.write(
        "default.project.json",
        "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n",
    );
    // Lint and the gate, but no `ci` - and no `.github/` on disk.
    project.write(
        "rproj.toml",
        "mode = \"expert\"\npackage_workflow = \"none\"\npackages = []\n\n\
         [capabilities]\nlint = \"selene\"\ngate = \"lute\"\n",
    );

    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);

    assert!(
        project.exists(".lute/check.luau"),
        "the gate was chosen:\n{}",
        outcome.text
    );
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
    project.write(
        "default.project.json",
        "{\n  \"name\": \"fixture\",\n  \"tree\": {}\n}\n",
    );
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

#[test]
fn jest_upgrade_regenerates_owned_files_and_preserves_user_options() {
    let project = TempProject::new("jest-upgrade");
    let production = r#"{
  "name": "fixture",
  "tree": {
    "$className": "DataModel",
    "ReplicatedStorage": { "$className": "ReplicatedStorage" },
    "ServerScriptService": { "$className": "ServerScriptService" },
    "StarterPlayer": {
      "$className": "StarterPlayer",
      "StarterPlayerScripts": { "$className": "StarterPlayerScripts" }
    }
  }
}
"#;
    project.write("default.project.json", production);
    project.write(
        "rproj.toml",
        "mode = \"expert\"\npackage_workflow = \"wally\"\npackages = [\"jest\", \"jest-globals\"]\n\n\
         [capabilities]\nlint = \"selene\"\ntest = \"jest-roblox\"\ngate = \"lute\"\nci = \"github-actions\"\n",
    );
    project.write(
        "jest.config.json",
        "{\"backend\":\"wrong\",\"timeout\":123,\"test\":{\"projects\":[\"old\"],\"verbose\":true}}\n",
    );
    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    for path in [
        "jest.project.json",
        "jest.config.json",
        ".github/workflows/ci.yml",
    ] {
        assert!(
            project.exists(path),
            "{path} was not generated:\n{}",
            outcome.text
        );
    }
    let mut expected: serde_json::Value = serde_json::from_str(production).unwrap();
    expected["tree"]["ReplicatedStorage"]["devPackages"] =
        serde_json::json!({"$path": "DevPackages"});
    let actual: serde_json::Value =
        serde_json::from_str(&project.read("default.project.json")).unwrap();
    assert_eq!(actual, expected);

    let test_project = project.read("jest.project.json");
    assert!(test_project.contains("DevPackages"), "{test_project}");
    assert!(test_project.contains("tests/shared"), "{test_project}");

    let config = project.read("jest.config.json");
    assert!(config.contains(r#""backend": "studio-cli""#), "{config}");
    assert!(config.contains(r#""timeout": 123"#), "{config}");
    assert!(config.contains(r#""verbose": true"#), "{config}");

    let ci = project.read(".github/workflows/ci.yml");
    assert!(!ci.contains("ROBLOX_OPEN_CLOUD_API_KEY"), "{ci}");
    assert!(!ci.contains("Run tests"), "{ci}");
    let record = project.read("rproj.toml").replace(
        "test = \"jest-roblox\"",
        "test = \"jest-roblox-open-cloud\"",
    );
    project.write("rproj.toml", &record);
    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    let ci = project.read(".github/workflows/ci.yml");
    assert!(ci.contains("jest-roblox-cli --backend open-cloud"), "{ci}");
    assert!(!ci.contains("JEST_OPEN_CLOUD"), "{ci}");
    assert!(
        project
            .read("jest.config.json")
            .contains(r#""backend": "open-cloud""#)
    );
}
