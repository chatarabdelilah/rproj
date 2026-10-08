//! `rproj upgrade` end to end, against hand-built fixture projects.
//!
//! No scaffolding and no network: `upgrade` only rewrites generated config,
//! so a directory holding `default.project.json`, `rproj.toml` and whatever
//! config is being tested is a complete and honest fixture. That keeps this
//! in the default `cargo test` run, where the regressions it guards against
//! actually get caught.

mod common;

use common::{ENTER, Session, TempProject, project_snapshot as snapshot};
use std::{fs, path::Path};

fn housekeeping_fixture(label: &str) -> TempProject {
    let project = TempProject::new(label);
    project.write("default.project.json", "{}\n");
    project.write("rproj.toml", "mode='expert'\npackage_workflow='none'\n");
    project
}

fn housekeeping_testez_fixture(label: &str) -> TempProject {
    let project = housekeeping_fixture(label);
    project.write(
        "rproj.toml",
        "mode='expert'\npackage_workflow='none'\n[capabilities]\ntest='testez'\n",
    );
    project
}

fn open_confirmation(session: &mut Session) {
    session.wait_for("Upgrade review");
    session.send("a");
    session.wait_for("Apply these changes?");
}

fn dismiss_confirmation(session: &mut Session, answer: &str) {
    session.send(answer);
    session.wait_for_screen("confirmation dismissed", |screen| {
        let text = screen.contents();
        text.contains("Upgrade review") && !text.contains("Apply these changes?")
    });
    session.send(common::ESC);
}

#[test]
fn upgrade_confirmation_defaults_to_no_and_preserves_the_project() {
    let project = housekeeping_fixture("upgrade-default-no");
    project.write("src/custom.luau", "return 'keep'\r\n");
    let before = snapshot(project.path());
    let mut session = Session::start(project.path(), &["upgrade"]);
    open_confirmation(&mut session);
    session.send(ENTER);
    session.wait_for_screen("default-No returned to review", |screen| {
        let text = screen.contents();
        text.contains("Upgrade review") && !text.contains("Apply these changes?")
    });
    assert_eq!(snapshot(project.path()), before);
    session.send(common::ESC);
    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(snapshot(project.path()), before);
    outcome.assert_lacks("wrote ");
}

#[test]
fn upgrade_review_resize_recovers_scrolled_diff_and_blocks_small_screen_apply() {
    let project = fixture("upgrade-viewer-resize", "\"vide\"");
    project.write(
        ".vscode/settings.json",
        &serde_json::json!({
            "custom": {"rows": (0..70).map(|n| format!("row{n:02} 東京")).collect::<Vec<_>>()}
        })
        .to_string(),
    );
    let before = snapshot(project.path());
    let mut session = Session::start(project.path(), &["upgrade"]);
    session.wait_for("Upgrade review");
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "Upgrade review");
    session.send(common::DOWN);
    session.wait_for_screen("settings diff selected", |screen| {
        screen.contents().contains(".vscode/settings.json — diff")
    });
    session.send("\t");
    session.send("\x1b[F");
    session.wait_for("row69 東京");
    let scrolled = common::screen_rows(&session.screen());
    for (rows, cols) in [(24, 80), (16, 60), (10, 40)] {
        let checkpoint = session.output_checkpoint();
        session.resize(rows, cols);
        session.wait_for_output_since(
            checkpoint,
            if cols < 60 {
                "Resize to at least 60 x 16"
            } else {
                "Upgrade review"
            },
        );
        session.send("?");
        session.wait_for("Help");
        session.send(common::ESC);
        session.wait_for_screen("Help dismissed", |screen| {
            !screen.contents().contains("Help")
        });
        if cols < 60 {
            session.send("ay?");
            session.wait_for("Help");
            assert_eq!(snapshot(project.path()), before);
            session.send(common::ESC);
            session.wait_for_screen("small-screen Apply probe acknowledged", |screen| {
                !screen.contents().contains("Help")
            });
        }
    }
    let checkpoint = session.output_checkpoint();
    session.resize(30, 120);
    session.wait_for_output_since(checkpoint, "row69 東京");
    session.wait_screen(&scrolled);
    session.send("\x1b[H");
    session.wait_for("LF unless marked");
    session.send("a");
    session.wait_for("Apply these changes?");
    let checkpoint = session.output_checkpoint();
    session.resize(10, 40);
    session.wait_for_output_since(checkpoint, "[No]");
    session.send("y?");
    session.wait_for("Help");
    assert_eq!(snapshot(project.path()), before);
    session.send(common::ESC);
    session.wait_for_screen("confirmation retained below minimum", |screen| {
        screen.contents().contains("[No]") && !screen.contents().contains("Help")
    });
    session.send(ENTER);
    session.wait_for("Resize to at least 60 x 16");
    session.send(common::ESC);
    let result = session.finish();
    assert_eq!(result.code, 0, "{}", result.text);
    assert_eq!(snapshot(project.path()), before);
}

#[test]
fn redirected_upgrade_refuses_review_and_yes_remains_plain() {
    use std::process::Command;
    let project = housekeeping_fixture("upgrade-redirected");
    let before = snapshot(project.path());
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rproj"))
            .current_dir(project.path())
            .args(args)
            .env("RPROJ_NO_LOG", "1")
            .output()
            .unwrap()
    };
    let refused = run(&["upgrade"]);
    assert!(!refused.status.success());
    let error = String::from_utf8_lossy(&refused.stderr);
    assert!(error.contains("interactive terminal"), "{error}");
    assert!(error.contains("--yes"), "{error}");
    assert_eq!(snapshot(project.path()), before);
    let applied = run(&["upgrade", "--yes"]);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let text = String::from_utf8_lossy(&applied.stdout);
    assert!(text.contains("wrote .gitignore"), "{text}");
    assert!(!text.contains("Upgrade review"));
    assert!(!text.contains("\x1b[?1049"));
    let after = snapshot(project.path());
    let noop = run(&["upgrade"]);
    assert!(noop.status.success());
    assert!(String::from_utf8_lossy(&noop.stdout).contains("already up to date"));
    assert_eq!(snapshot(project.path()), after);
}

#[test]
fn housekeeping_only_upgrade_is_reviewed_and_cancellable() {
    for answer in ["n", common::ESC, "\u{3}"] {
        let project = housekeeping_fixture("housekeeping-review");
        let before = snapshot(project.path());
        let mut session = Session::start(project.path(), &["upgrade"]);
        open_confirmation(&mut session);
        assert_eq!(snapshot(project.path()), before);
        if answer == "\u{3}" {
            session.send(answer);
        } else {
            dismiss_confirmation(&mut session, answer);
        }
        let outcome = session.finish();
        outcome.assert_contains("create .gitignore");
        outcome.assert_contains("create .luaurc");
        outcome.assert_lacks("already up to date");
        outcome.assert_lacks("wrote ");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn housekeeping_upgrade_preserves_custom_values_and_second_run_is_a_noop() {
    for args in [&["upgrade"][..], &["upgrade", "--yes"][..]] {
        let project = housekeeping_testez_fixture("housekeeping-preserve");
        let prepared = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(prepared.code, 0, "{}", prepared.text);
        project.write(".gitignore", "# custom\r\ncustom-output/");
        project.write(".luaurc", "{\"aliases\":{\"custom\":\"./custom\"}}\n");
        project.write("tests/.luaurc", "{\"aliases\":{\"test\":\"./helpers\"}}\n");
        project.write("src/custom.luau", "return 'keep'\r\n");
        let mut before = snapshot(project.path());
        let mut session = Session::start(project.path(), args);
        if args.len() == 1 {
            open_confirmation(&mut session);
            assert_eq!(snapshot(project.path()), before);
            session.send("y");
        }
        let outcome = session.finish();
        assert_eq!(outcome.code, 0, "{}", outcome.text);
        for relative in [".gitignore", ".luaurc", "tests/.luaurc"] {
            outcome.assert_contains(&format!("update {relative}"));
            before.remove(Path::new(relative));
        }
        let mut after = snapshot(project.path());
        for relative in [".gitignore", ".luaurc", "tests/.luaurc"] {
            after.remove(Path::new(relative));
        }
        assert_eq!(
            after.keys().collect::<Vec<_>>(),
            before.keys().collect::<Vec<_>>()
        );
        for (relative, contents) in before {
            assert_eq!(after[&relative], contents, "{} changed", relative.display());
        }
        assert!(
            project
                .read(".gitignore")
                .starts_with("# custom\r\ncustom-output/\n")
        );
        let config: serde_json::Value = serde_json::from_str(&project.read(".luaurc")).unwrap();
        assert_eq!(config["languageMode"], "strict");
        assert_eq!(config["aliases"]["custom"], "./custom");
        let tests: serde_json::Value =
            serde_json::from_str(&project.read("tests/.luaurc")).unwrap();
        assert_eq!(tests["aliases"]["test"], "./helpers");
        assert!(
            tests["globals"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("describe"))
        );
        // Existing choices, including whitespace, must remain byte-identical.
        project.write(
            ".luaurc",
            "{ \"languageMode\": \"nonstrict\", \"aliases\": {} }\r\n",
        );
        project.write("tests/.luaurc", "{ \"globals\": [\"custom\"] }\r\n");
        let before = snapshot(project.path());
        let outcome = Session::start(project.path(), &["upgrade"]).finish();
        assert_eq!(outcome.code, 0, "{}", outcome.text);
        outcome.assert_contains("already up to date");
        outcome.assert_lacks("Apply these changes?");
        outcome.assert_lacks("wrote ");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn housekeeping_conflicts_refuse_every_write() {
    for relative in [".gitignore", ".luaurc", "tests/.luaurc"] {
        for mode in ["edit", "delete", "create"] {
            let project = fixture("housekeeping-conflict", "\"testez\"");
            if mode != "create" {
                project.write(
                    relative,
                    if relative == ".gitignore" {
                        "custom/\n"
                    } else {
                        "{}\n"
                    },
                );
            }
            let mut session = Session::start(project.path(), &["upgrade"]);
            open_confirmation(&mut session);
            if mode == "delete" {
                fs::remove_file(project.path().join(relative)).unwrap();
            } else {
                project.write(relative, "external edit\n");
            }
            let before = snapshot(project.path());
            session.send("y");
            let outcome = session.finish();
            assert_eq!(outcome.code, 1, "{}", outcome.text);
            outcome.assert_contains("changed while upgrade was being reviewed");
            outcome.assert_contains("Nothing written");
            assert_eq!(snapshot(project.path()), before);
        }
    }
}

#[test]
fn unreadable_housekeeping_refuses_before_confirmation() {
    for relative in [".gitignore", ".luaurc", "tests/.luaurc"] {
        let project = fixture("housekeeping-unreadable", "\"testez\"");
        fs::create_dir_all(project.path().join(relative)).unwrap();
        let before = snapshot(project.path());
        let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("failed to read");
        outcome.assert_lacks("Apply these changes?");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn unparseable_luaurc_files_are_reported_and_preserved() {
    for contents in ["// commented\n{}\n", "broken [", "[]\n"] {
        let project = housekeeping_testez_fixture("housekeeping-unparseable");
        let prepared = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(prepared.code, 0, "{}", prepared.text);
        for relative in [".luaurc", "tests/.luaurc"] {
            project.write(relative, contents);
        }
        let before = snapshot(project.path());
        let outcome = Session::start(project.path(), &["upgrade"]).finish();
        assert_eq!(outcome.code, 0, "{}", outcome.text);
        outcome.assert_contains(".luaurc exists but couldn't be parsed");
        outcome.assert_contains("tests/.luaurc exists but couldn't be parsed");
        outcome.assert_contains("no applicable upgrade changes");
        outcome.assert_lacks("already up to date");
        outcome.assert_lacks("Apply these changes?");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn later_housekeeping_staging_failure_preserves_earlier_targets() {
    let project = fixture("housekeeping-readonly", "\"testez\"");
    project.write("selene.toml", "std='roblox'\n");
    project.write(".vscode/settings.json", "{}\n");
    project.write("tests/.luaurc", "{}\n");
    let before = snapshot(project.path());
    let path = project.path().join("tests/.luaurc");
    let original = fs::metadata(&path).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    let mut session = Session::start(project.path(), &["upgrade"]);
    open_confirmation(&mut session);
    fs::set_permissions(&path, readonly).unwrap();
    session.send("y");
    let outcome = session.finish();
    fs::set_permissions(&path, original).unwrap();
    assert_eq!(outcome.code, 1, "{}", outcome.text);
    outcome.assert_contains("No upgrade targets replaced");
    outcome.assert_lacks("wrote ");
    assert_eq!(snapshot(project.path()), before);
}

fn wally_ci_fixture(pin: Option<&str>) -> TempProject {
    let project = fixture("released-wpt", "\"charm\", \"testez\"");
    project.write(
        "rproj.toml",
        &format!(
            "{}gate='lute'\nci='github-actions'\n",
            project.read("rproj.toml")
        ),
    );
    if let Some(pin) = pin {
        project.write("rokit.toml", &format!("# owner choices\n[tools]\nwally-package-types='{pin}'\nwally='UpliftGames/wally@0.3.2'\n"));
    }
    project.write("src/custom.luau", "return 'keep'\n");
    project
}

#[test]
fn incompatible_wpt_pins_refuse_ci_upgrade_without_any_writes() {
    for pin in [
        None,
        Some("JohnnyMorganz/wally-package-types@1.6.2"),
        Some("JohnnyMorganz/wally-package-types@1.7.0-rc.1"),
        Some("JohnnyMorganz/wally-package-types@latest"),
        Some("Other/wally-package-types@1.7.0"),
        Some("JohnnyMorganz/wally-package-types@1.7"),
        Some("JohnnyMorganz/wally-package-types@1.7.0.1"),
    ] {
        for args in [&["upgrade"][..], &["upgrade", "--yes"][..]] {
            let project = wally_ci_fixture(pin);
            let before = snapshot(project.path());
            let outcome = Session::start(project.path(), args).finish();
            assert_eq!(outcome.code, 1, "{}", outcome.text);
            outcome.assert_contains("wally-package-types");
            outcome.assert_contains("1.7.0");
            outcome.assert_contains(if pin.is_none() {
                "rokit add wally-package-types"
            } else {
                "rokit update wally-package-types"
            });
            outcome.assert_lacks("Apply these changes?");
            assert_eq!(snapshot(project.path()), before);
        }
    }
}

#[test]
fn malformed_or_missing_wpt_entries_refuse_ci_upgrade_without_writes() {
    for manifest in [
        "invalid [toml",
        "[tools]\nwally='UpliftGames/wally@0.3.2'\n",
        "[tools]\nwally-package-types=17\n",
    ] {
        let project = wally_ci_fixture(None);
        project.write("rokit.toml", manifest);
        let before = snapshot(project.path());
        let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("rokit.toml");
        outcome.assert_contains("Nothing written");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn released_wpt_pins_allow_ci_upgrade_without_changing_tools() {
    for version in ["1.7.0", "1.10.0", "2.0.0", "v1.7.0", "1.7.0+build.1"] {
        let project = wally_ci_fixture(Some(&format!(
            "JohnnyMorganz/wally-package-types@{version}"
        )));
        let manifest = project.read("rokit.toml");
        let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
        assert_eq!(outcome.code, 0, "{}", outcome.text);
        assert_eq!(project.read("rokit.toml"), manifest);
        let ci = project.read(".github/workflows/ci.yml");
        assert!(ci.contains("wally-package-types --sourcemap"), "{ci}");
        assert!(!ci.contains("cargo install"), "{ci}");
        assert_eq!(project.read("src/custom.luau"), "return 'keep'\n");
    }
}

#[test]
fn released_wpt_upgrade_cancel_and_changed_pin_preserve_every_file() {
    for mode in ["cancel", "edit", "delete"] {
        let project = wally_ci_fixture(Some("JohnnyMorganz/wally-package-types@1.7.0"));
        let mut session = Session::start(project.path(), &["upgrade"]);
        open_confirmation(&mut session);
        if mode == "edit" {
            project.write(
                "rokit.toml",
                "[tools]\nwally-package-types='JohnnyMorganz/wally-package-types@1.6.2'\n",
            );
        } else if mode == "delete" {
            fs::remove_file(project.path().join("rokit.toml")).unwrap();
        }
        let before = snapshot(project.path());
        if mode == "cancel" {
            dismiss_confirmation(&mut session, "n");
        } else {
            session.send("y");
        }
        let outcome = session.finish();
        assert_eq!(
            outcome.code,
            if mode == "cancel" { 0 } else { 1 },
            "{}",
            outcome.text
        );
        if mode != "cancel" {
            outcome.assert_contains("changed while upgrade was being reviewed");
        }
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn projects_without_managed_wally_ci_do_not_require_a_released_wpt_pin() {
    let project = fixture("no-ci-wpt", "\"charm\"");
    project.write("rokit.toml", "user owned\n");
    let outcome = Session::start(project.path(), &["upgrade", "--yes"]).finish();
    assert_eq!(outcome.code, 0, "{}", outcome.text);
    assert_eq!(project.read("rokit.toml"), "user owned\n");
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
        open_confirmation(&mut session);
        let external = "{\"editor.rulers\":[80],\"custom\":{\"keep\":true}}\n";
        if mode == "delete" {
            fs::remove_file(project.path().join(".vscode/settings.json")).unwrap();
        } else {
            project.write(".vscode/settings.json", external);
        }
        let before = snapshot(project.path());
        session.send("y");
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
        open_confirmation(&mut session);
        project.write(relative, &format!("{}\n", project.read(relative)));
        let before = snapshot(project.path());
        session.send("y");
        let outcome = session.finish();
        assert_eq!(outcome.code, 1, "{}", outcome.text);
        outcome.assert_contains("changed while upgrade was being reviewed");
        assert_eq!(snapshot(project.path()), before);
    }
}

#[test]
fn rejecting_or_cancelling_upgrade_keeps_every_file_and_directory_unchanged() {
    for answer in ["n", common::ESC] {
        let project = fixture("upgrade-cancel", "\"vide\", \"testez\"");
        project.write(
            "selene.toml",
            "std='roblox'\n[rules]\nunused_variable='allow'\n",
        );
        project.write(".vscode/settings.json", "{\"editor.rulers\":[100]}\n");
        project.write("src/custom.luau", "return 'keep'\n");
        let before = snapshot(project.path());
        let mut session = Session::start(project.path(), &["upgrade"]);
        open_confirmation(&mut session);
        dismiss_confirmation(&mut session, answer);
        let outcome = session.finish();
        if answer == "n" {
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
    open_confirmation(&mut session);
    session.send("y");
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
    open_confirmation(&mut session);
    fs::set_permissions(&path, readonly).unwrap();
    session.send("y");
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

#[test]
fn unsafe_multiline_selene_merges_refuse_without_any_writes() {
    for (workflow, packages, source) in [
        (
            "none",
            "",
            "std = \"roblox\"\nnotes = '''\nstd = \"example\"\n'''\n",
        ),
        (
            "wally",
            "\"vide\", \"testez\"",
            "std = \"roblox\"\nnotes = '''\nstd = \"example\"\n'''\n",
        ),
        (
            "wally",
            "\"vide\"",
            "std = \"roblox\"\n[rules]\nmixed_table = \"warn\"\nnotes = '''\nmixed_table = \"example\"\n'''\n",
        ),
        (
            "wally",
            "\"charm\"",
            "std = \"roblox\"\nnotes = '''\nexclude = [\"custom/**\"]\n'''\n",
        ),
        (
            "wally",
            "\"vide\"",
            "std = \"roblox\"\nnotes = '''\n[rules]\n'''\n",
        ),
        ("none", "", "std = \"\"\"\nroblox\n\"\"\"\n"),
    ] {
        assert!(toml::from_str::<toml::Table>(source).is_ok());
        for args in [&["upgrade", "--yes"][..], &["upgrade"][..]] {
            let project = fixture("selene-unsafe-multiline", packages);
            if workflow == "none" {
                project.write(
                    "rproj.toml",
                    "mode='expert'\npackage_workflow='none'\n[capabilities]\nlint='selene'\n",
                );
            }
            project.write("selene.toml", source);
            project.write(".gitignore", "# custom\ncustom-output/\n");
            project.write(".luaurc", "{\"aliases\":{\"custom\":\"./custom\"}}\n");
            project.write("src/custom.luau", "return 'keep'\r\n");
            let before = snapshot(project.path());
            let outcome = Session::start(project.path(), args).finish();
            assert_eq!(outcome.code, 1, "{}", outcome.text);
            outcome.assert_contains("selene.toml");
            outcome.assert_contains("cannot safely edit this TOML layout");
            outcome.assert_contains("edit the file manually");
            outcome.assert_contains("re-run `rproj upgrade`");
            outcome.assert_lacks("Apply these changes?");
            outcome.assert_lacks("wrote ");
            assert_eq!(snapshot(project.path()), before);
        }
    }
}

#[test]
fn safe_multiline_selene_merges_preserve_values_and_settle_after_one_run() {
    let source = "# project note\nstd = \"custom\"\nnotes = '''\ncustom = \"keep\"\n'''\nexclude = [\"custom/**\"]\n\n[rules]\nmixed_table = \"warn\"\nshadowing = \"deny\"\n\n[owner]\nvalue = 42\n";
    let original: toml::Table = toml::from_str(source).unwrap();
    for workflow in ["none", "wally"] {
        for args in [&["upgrade", "--yes"][..], &["upgrade"][..]] {
            let project = fixture("selene-safe-multiline", "\"vide\", \"testez\"");
            if workflow == "none" {
                project.write(
                    "rproj.toml",
                    "mode='expert'\npackage_workflow='none'\n[capabilities]\nlint='selene'\n",
                );
            }
            project.write("selene.toml", source);
            project.write("src/custom.luau", "return 'keep'\r\n");
            let before = snapshot(project.path());
            let mut session = Session::start(project.path(), args);
            if args.len() == 1 {
                open_confirmation(&mut session);
                assert_eq!(snapshot(project.path()), before);
                session.send("y");
            }
            let first = session.finish();
            assert_eq!(first.code, 0, "{}", first.text);
            let text = project.read("selene.toml");
            let actual: toml::Table = toml::from_str(&text).unwrap();
            assert_eq!(actual["notes"], original["notes"]);
            assert_eq!(actual["owner"], original["owner"]);
            assert_eq!(actual["rules"]["shadowing"], original["rules"]["shadowing"]);
            assert!(text.contains("# project note\n"));
            assert_eq!(project.read("src/custom.luau"), "return 'keep'\r\n");
            assert_eq!(
                actual["std"].as_str(),
                Some(if workflow == "wally" {
                    "roblox+testez"
                } else {
                    "roblox"
                })
            );
            assert_eq!(
                actual["rules"]["mixed_table"].as_str(),
                Some(if workflow == "wally" { "allow" } else { "warn" })
            );
            let expected = if workflow == "wally" {
                vec![
                    "custom/**",
                    "Packages/**",
                    "ServerPackages/**",
                    "DevPackages/**",
                ]
            } else {
                vec!["custom/**"]
            };
            assert_eq!(
                actual["exclude"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| entry.as_str().unwrap())
                    .collect::<Vec<_>>(),
                expected
            );
            let after = snapshot(project.path());
            let second = Session::start(project.path(), args).finish();
            assert_eq!(second.code, 0, "{}", second.text);
            second.assert_contains("already up to date");
            assert_eq!(snapshot(project.path()), after);
        }
    }
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

#[test]
fn recreating_selene_is_complete_after_one_upgrade() {
    for (workflow, packages, std) in [
        ("wally", "\"vide\"", "roblox"),
        ("wally", "\"vide\", \"testez\"", "roblox+testez"),
        ("none", "", "roblox"),
    ] {
        for args in [&["upgrade"][..], &["upgrade", "--yes"][..]] {
            let project = fixture("selene-recreate", packages);
            if workflow == "none" {
                project.write(
                    "rproj.toml",
                    "mode='expert'\npackage_workflow='none'\n[capabilities]\nlint='selene'\n",
                );
            }
            let before = snapshot(project.path());
            let mut session = Session::start(project.path(), args);
            if args.len() == 1 {
                open_confirmation(&mut session);
                assert_eq!(snapshot(project.path()), before);
                session.send("y");
            }
            let first = session.finish();
            assert_eq!(first.code, 0, "{}", first.text);
            first.assert_contains("create selene.toml");
            let selene: toml::Value = toml::from_str(&project.read("selene.toml")).unwrap();
            assert_eq!(selene["std"].as_str(), Some(std));
            let excludes = selene["exclude"].as_array().unwrap();
            if workflow == "wally" {
                assert_eq!(
                    excludes
                        .iter()
                        .map(|value| value.as_str().unwrap())
                        .collect::<Vec<_>>(),
                    ["Packages/**", "ServerPackages/**", "DevPackages/**"]
                );
            } else {
                assert!(excludes.is_empty());
            }
            let after_first = snapshot(project.path());
            let second = Session::start(project.path(), args).finish();
            assert_eq!(second.code, 0, "{}", second.text);
            second.assert_contains("already up to date");
            second.assert_lacks("Apply these changes?");
            second.assert_lacks("wrote ");
            assert_eq!(snapshot(project.path()), after_first);
        }
    }
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
    project.write(
        "rokit.toml",
        "[tools]\nwally-package-types='JohnnyMorganz/wally-package-types@1.7.0'\n",
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
