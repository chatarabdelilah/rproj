use std::{ffi::OsString, fs, process::Command};

fn fixture_tools() -> (tempfile::TempDir, OsString) {
    let tools = tempfile::tempdir().unwrap();
    let executable = tools
        .path()
        .join(format!("tool{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new("rustc")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/test_tools.rs"
        ))
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    for name in [
        "rojo",
        "wally",
        "wally-package-types",
        "jest-roblox-cli",
        "lute",
    ] {
        fs::copy(
            &executable,
            tools
                .path()
                .join(format!("{name}{}", std::env::consts::EXE_SUFFIX)),
        )
        .unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(tools.path().to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    (tools, path)
}

#[test]
fn testing_reuses_complete_packages_and_recovers_missing_packages() {
    let (_tools, path) = fixture_tools();
    for (runner, section, folder, alias, source) in [
        (
            "testez",
            "dependencies",
            "Packages",
            "testez",
            "roblox/testez@0.4.1",
        ),
        (
            "jest-roblox-open-cloud",
            "dev-dependencies",
            "DevPackages",
            "Jest",
            "roblox/jest@3.20.1",
        ),
    ] {
        for (ready, retyped) in [(true, true), (true, false), (false, true)] {
            let project = tempfile::tempdir().unwrap();
            fs::write(
                project.path().join("rproj.toml"),
                format!(
                    "mode='expert'\npackage_workflow='wally'\n[capabilities]\ntest='{runner}'\n"
                ),
            )
            .unwrap();
            fs::write(
                project.path().join("default.project.json"),
                r#"{"tree":{"ReplicatedStorage":{},"ServerScriptService":{},"StarterPlayer":{"StarterPlayerScripts":{}}}}"#,
            )
            .unwrap();
            fs::write(project.path().join("wally.toml"), format!(
                "[package]\nname='rproj/demo'\nversion='0.1.0'\nrealm='shared'\n[{section}]\n{alias}='{source}'\n"
            )).unwrap();
            fs::write(project.path().join("wally.lock"), format!(
                "[[package]]\nname='rproj/demo'\nversion='0.1.0'\ndependencies=[['{alias}', '{source}']]\n"
            )).unwrap();
            let package = project.path().join(folder);
            fs::create_dir_all(package.join("_Index")).unwrap();
            let original = if retyped {
                "local REQUIRED_MODULE = require(script.Parent._Index.Library)\nexport type Example = REQUIRED_MODULE.Example\nreturn REQUIRED_MODULE\n"
            } else {
                "return require(script.Parent._Index.Library)\n"
            };
            let link = package.join(format!("{alias}.lua"));
            fs::write(&link, original).unwrap();
            if !ready {
                fs::remove_dir(package.join("_Index")).unwrap();
            }

            for _ in 0..2 {
                let output = Command::new(env!("CARGO_BIN_EXE_rproj"))
                    .arg("test")
                    .current_dir(project.path())
                    .env("PATH", &path)
                    .env("RPROJ_NO_LOG", "1")
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{runner}, ready={ready}, retyped={retyped}: {}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                assert_eq!(fs::read_to_string(&link).unwrap(), original);
                assert!(project.path().join("sourcemap.json").is_file());
            }
            let calls = fs::read_to_string(project.path().join("tool-calls.txt")).unwrap();
            let names: Vec<_> = calls
                .lines()
                .map(|line| line.split_once(' ').unwrap().0)
                .collect();
            let runner_command = if runner == "testez" {
                "lute"
            } else {
                "jest-roblox-cli"
            };
            if ready {
                assert_eq!(names, ["rojo", runner_command, "rojo", runner_command]);
            } else {
                assert_eq!(
                    names,
                    [
                        "wally",
                        "rojo",
                        "wally-package-types",
                        runner_command,
                        "rojo",
                        runner_command
                    ]
                );
            }
        }
    }
}

#[test]
#[ignore = "requires RPROJ_LIVE_JEST_CLI pointing to an installed Jest Roblox CLI; no cloud credentials"]
fn open_cloud_missing_credentials_report_names_and_preserve_exit_code() {
    let runner = std::env::var_os("RPROJ_LIVE_JEST_CLI")
        .expect("set RPROJ_LIVE_JEST_CLI to an installed Jest Roblox CLI executable");
    let (tools, path) = fixture_tools();
    fs::copy(
        runner,
        tools
            .path()
            .join(format!("jest-roblox-cli{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("copy the installed runner into the isolated tool fixture");
    let credentials = [
        ("ROBLOX_OPEN_CLOUD_API_KEY", "apiKey", "rproj-fixture-key"),
        ("ROBLOX_UNIVERSE_ID", "universeId", "1"),
        ("ROBLOX_PLACE_ID", "placeId", "2"),
    ];
    for prefix in ["", "JEST_"] {
        // Never supply all three credentials to the real runner. The base URL
        // also targets loopback, so this fixture cannot upload a cloud place.
        for present in 0_u8..7 {
            let project = tempfile::tempdir().unwrap();
            fs::write(
                project.path().join("rproj.toml"),
                "mode='expert'\npackage_workflow='wally'\n[capabilities]\ntest='jest-roblox-open-cloud'\n",
            ).unwrap();
            fs::write(
                project.path().join("default.project.json"),
                r#"{"name":"Refusal","tree":{"$className":"DataModel","ReplicatedStorage":{"$className":"ReplicatedStorage","devPackages":{"$path":"DevPackages"}},"ServerScriptService":{},"StarterPlayer":{"StarterPlayerScripts":{}}}}"#,
            ).unwrap();
            fs::write(
                project.path().join("wally.toml"),
                "[package]\nname='rproj/demo'\nversion='0.1.0'\nrealm='shared'\n[dev-dependencies]\nJest='roblox/jest@3.20.1'\n",
            ).unwrap();
            fs::write(
                project.path().join("wally.lock"),
                "[[package]]\nname='rproj/demo'\nversion='0.1.0'\ndependencies=[['Jest', 'roblox/jest@3.20.1']]\n",
            ).unwrap();
            fs::create_dir_all(project.path().join("DevPackages/_Index")).unwrap();
            fs::write(project.path().join("DevPackages/Jest.lua"), "return {}\n").unwrap();
            for area in ["shared", "server", "client"] {
                fs::create_dir_all(project.path().join("tests").join(area)).unwrap();
            }
            let spec = project.path().join("tests/shared/refusal.spec.luau");
            let source =
                "return function() it('runs', function() expect(true).to.equal(true) end) end\n";
            fs::write(&spec, source).unwrap();
            let production = fs::read(project.path().join("default.project.json")).unwrap();
            let mut command = Command::new(env!("CARGO_BIN_EXE_rproj"));
            command
                .arg("test")
                .current_dir(project.path())
                .env("PATH", &path)
                .env("RPROJ_NO_LOG", "1")
                .env("JEST_ROBLOX_OPEN_CLOUD_BASE_URL", "http://127.0.0.1:9");
            let mut missing = Vec::new();
            for (index, (name, field, value)) in credentials.iter().enumerate() {
                command.env_remove(name).env_remove(format!("JEST_{name}"));
                if present & (1 << index) != 0 {
                    command.env(format!("{prefix}{name}"), value);
                } else {
                    missing.push(*field);
                }
            }
            let output = command.output().unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.status.code(), Some(2), "{prefix}, {present}: {text}");
            assert!(
                text.contains(&format!("Missing: {}.", missing.join(", "))),
                "{text}"
            );
            for (index, (name, _, _)) in credentials.iter().enumerate() {
                if present & (1 << index) == 0 {
                    assert!(text.contains(&format!("{name} (or JEST_{name})")), "{text}");
                }
            }
            assert!(!text.contains("rproj-fixture-key"), "{text}");
            assert_eq!(fs::read_to_string(&spec).unwrap(), source);
            assert_eq!(
                fs::read(project.path().join("default.project.json")).unwrap(),
                production
            );
            assert_eq!(
                fs::read_to_string(project.path().join("DevPackages/Jest.lua")).unwrap(),
                "return {}\n"
            );
            let calls = fs::read_to_string(project.path().join("tool-calls.txt")).unwrap();
            assert_eq!(calls.lines().count(), 1, "unexpected preparation: {calls}");
            assert!(calls.starts_with("rojo sourcemap "), "{calls}");
        }
    }
}
