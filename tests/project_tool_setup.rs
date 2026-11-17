use std::{fs, path::Path, process::Command, sync::OnceLock};

fn tools() -> &'static Path {
    static TOOLS: OnceLock<tempfile::TempDir> = OnceLock::new();
    TOOLS
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let output = Command::new("rustc")
                .args([
                    "--edition=2024",
                    "tests/fixtures/project_tool_rokit.rs",
                    "-o",
                ])
                .arg(
                    dir.path()
                        .join(format!("rokit{}", std::env::consts::EXE_SUFFIX)),
                )
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            dir
        })
        .path()
}

fn run(mode: &str, manifest: Option<&str>) -> (tempfile::TempDir, std::process::Output) {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("project space é");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(project.join("default.project.json"), "{}").unwrap();
    if let Some(manifest) = manifest {
        fs::write(project.join("rokit.toml"), manifest).unwrap();
    }
    let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(["setup", "asphalt"])
        .current_dir(project.join("src"))
        .env("PATH", tools())
        .env("RPROJ_NO_LOG", "1")
        .env("RPROJ_TOOL_FIXTURE_MODE", mode)
        .env("RPROJ_TOOL_FIXTURE_LOG", dir.path().join("calls"))
        .output()
        .unwrap();
    (dir, output)
}

#[test]
fn failed_add_is_not_success_even_when_configuration_is_created() {
    for mode in [
        "failed-add",
        "misleading-output",
        "missing-pin",
        "failed-trust",
        "failed-after-pin",
    ] {
        let (dir, output) = run(mode, Some("[tools]\n"));
        assert!(
            !output.status.success(),
            "{mode}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(dir.path().join("project space é/asphalt.toml").is_file());
    }
}

#[test]
fn fatal_initialization_and_unexpected_manifest_changes_stop_file_creation() {
    for (mode, manifest) in [
        ("failed-init", None),
        (
            "changed-other-pin",
            Some("[tools]\nwally='owner/pinned@1'\n"),
        ),
    ] {
        let (dir, output) = run(mode, manifest);
        assert!(!output.status.success(), "{mode}");
        assert!(!dir.path().join("project space é/asphalt.toml").exists());
    }
}

#[test]
fn existing_custom_pin_is_preserved_without_external_commands() {
    let manifest = "# retain this comment\n[tools]\nasphalt = 'owner/fork@1.2.3'\n";
    let (dir, output) = run("failed-add", Some(manifest));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("project space é/rokit.toml")).unwrap(),
        manifest
    );
    assert!(!dir.path().join("calls").exists());
}

#[test]
fn missing_manifest_and_config_use_existing_generator_bytes() {
    let (dir, output) = run("success", None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let project = dir.path().join("project space é");
    let config = fs::read_to_string(project.join("asphalt.toml")).unwrap();
    assert!(config.contains("path = \"assets/**/*.png\""));
    assert!(config.contains("output_path = \"src/shared\""));
    assert!(project.join("assets/.gitkeep").is_file());
    assert!(!project.join("rproj.toml").exists());
}
