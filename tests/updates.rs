use std::{fs, process::Command};

#[test]
fn unsupported_sources_are_reported_without_writes_by_cli_and_worker() {
    let root = tempfile::tempdir().unwrap();
    let manifest = "[package]\nname='sample/game'\nversion='0.1.0'\nregistry='https://example.invalid/private'\n[dependencies]\nCustom='sample/signals@1'\n";
    fs::write(root.path().join("wally.toml"), manifest).unwrap();
    let invoke = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
            .current_dir(root.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let cli: serde_json::Value =
        serde_json::from_str(&invoke(&["updates", ".", "--json"])).unwrap();
    let worker: serde_json::Value =
        serde_json::from_str(&invoke(&["--rproj-workbench", "updates-v1"])).unwrap();
    assert_eq!(cli["entries"], worker["entries"]);
    assert_eq!(cli["entries"][0]["comparison"]["status"], "unknown");
    assert!(
        cli["entries"][0]["comparison"]["reason"]
            .as_str()
            .unwrap()
            .contains("unsupported Wally registry")
    );
    assert_eq!(cli["schema_version"], 1);
    assert!(invoke(&["updates"]).contains("dependencies.Custom"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    assert_eq!(
        fs::read_to_string(root.path().join("wally.toml")).unwrap(),
        manifest
    );
}
