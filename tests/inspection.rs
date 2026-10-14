use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn inspect(root: &Path, arguments: &[&str]) -> Output {
    let mut invocation = vec!["inspect"];
    invocation.extend_from_slice(arguments);
    cli(root, &invocation)
}

fn cli(root: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(arguments)
        .current_dir(root)
        .env("RPROJ_LOG_DIR", root.join("diagnostic-logs"))
        .env_remove("RPROJ_NO_LOG")
        .output()
        .unwrap()
}

#[test]
fn invalid_inspect_arguments_preserve_absent_and_existing_diagnostic_logs() {
    for arguments in [
        &["inspect", "--bad-option"][..],
        &["--verbose", "inspect", "--bad-option"],
        &["-v", "inspect", "--bad-option"],
        &["inspect", "first-project", "extra-project"],
        &["inspect", "--json=false"],
    ] {
        for existing_logs in [false, true] {
            let project = TempDir::new().unwrap();
            let logs = project.path().join("diagnostic-logs");
            let preserved = b"existing diagnostic output\r\n";
            if existing_logs {
                fs::create_dir(&logs).unwrap();
                fs::write(logs.join("existing.txt"), preserved).unwrap();
            }

            let output = cli(project.path(), arguments);

            assert_eq!(output.status.code(), Some(2), "{arguments:?}");
            assert!(output.stdout.is_empty(), "{arguments:?}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("error:"),
                "{arguments:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            if existing_logs {
                assert_eq!(fs::read(logs.join("existing.txt")).unwrap(), preserved);
                assert_eq!(fs::read_dir(&logs).unwrap().count(), 1, "{arguments:?}");
            } else {
                assert!(!logs.exists(), "{arguments:?} created diagnostic logs");
            }
        }
    }
}

#[test]
fn inspect_as_an_ordinary_argument_does_not_disable_parse_error_logging() {
    for arguments in [
        &["new", "inspect", "--bad-option"][..],
        &["--verbose", "new", "--like", "inspect", "--bad-option"],
        &["info", "inspect", "--bad-option"],
        &["--bad-option", "inspect"],
    ] {
        let project = TempDir::new().unwrap();

        let output = cli(project.path(), arguments);

        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        let logs: Vec<_> = fs::read_dir(project.path().join("diagnostic-logs"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        assert_eq!(logs.len(), 1, "{arguments:?}");
        let text = fs::read_to_string(&logs[0]).unwrap();
        assert!(text.contains("cli.invalid"), "{arguments:?}: {text}");
    }
}

#[test]
fn json_inspection_is_clean_and_reads_manual_dependencies_without_writes() {
    let project = TempDir::new().unwrap();
    fs::write(
        project.path().join("default.project.json"),
        r#"{"tree":{}}"#,
    )
    .unwrap();
    let saved = "packages = ['old-selection']\n";
    fs::write(project.path().join("rproj.toml"), saved).unwrap();
    fs::write(project.path().join("wally.toml"), "[package]\nname = 'owner/game'\nversion = '0.1.0'\nrealm = 'shared'\n[dependencies]\nManual = 'unknown/manual@1'\n[dev-dependencies]\nDev = 'unknown/dev@2'\n").unwrap();

    let output = inspect(project.path(), &["--json"]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let snapshot: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot["schema_version"], 1);
    assert_eq!(snapshot["packages"][0]["alias"], "Manual");
    assert_eq!(snapshot["packages"][1]["realm"], "dev");
    assert_eq!(snapshot["saved_packages"][0], "old-selection");
    assert_eq!(snapshot["files"]["wally_lock"]["state"], "missing");
    assert!(!project.path().join("diagnostic-logs").exists());
    assert_eq!(
        fs::read_to_string(project.path().join("rproj.toml")).unwrap(),
        saved
    );
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 3);
}

#[test]
fn invalid_configuration_is_successfully_reported_as_structured_diagnostics() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("wally.toml"), "[broken").unwrap();
    let output = inspect(project.path(), &["--json"]);
    assert!(output.status.success());
    let snapshot: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(snapshot["files"]["wally"]["state"], "invalid");
    assert!(
        snapshot["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diagnostic| diagnostic["code"] == "invalid_toml")
    );
    assert!(!project.path().join("diagnostic-logs").exists());
}

#[test]
fn missing_location_returns_nonzero_and_stderr_without_json_or_logs() {
    let project = TempDir::new().unwrap();
    let output = inspect(project.path(), &["missing", "--json"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot inspect"));
    assert!(!project.path().join("diagnostic-logs").exists());
}

#[test]
fn explicit_plain_rojo_directory_gets_human_facts_without_adoption() {
    let root = TempDir::new().unwrap();
    let project = root.path().join("project with spaces");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("default.project.json"),
        r#"{"name":"Plain","tree":{}}"#,
    )
    .unwrap();
    let output = inspect(root.path(), &["project with spaces"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Name: Plain"));
    assert!(text.contains("Kind: Rojo"));
    assert!(text.contains("watch: available"));
    assert!(text.contains("rproj.toml: Missing"));
    assert!(!project.join("rproj.toml").exists());
    assert!(!root.path().join("diagnostic-logs").exists());
}
