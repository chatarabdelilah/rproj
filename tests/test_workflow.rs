use std::{fs, process::Command};

#[test]
fn testing_reuses_complete_packages_and_recovers_missing_packages() {
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
