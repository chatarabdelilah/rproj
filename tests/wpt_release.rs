//! Explicit compatibility check for the released tool; ordinary tests stay offline.
use std::{fs, process::Command};

#[test]
#[ignore = "requires official wally-package-types 1.7.0 via RPROJ_WPT_TEST_BIN; uses only disposable files"]
fn released_wpt_preserves_valid_generic_types_and_parses_const() {
    let tool = std::env::var_os("RPROJ_WPT_TEST_BIN")
        .expect("set RPROJ_WPT_TEST_BIN to the verified official 1.7.0 binary");
    let tool = fs::canonicalize(tool).unwrap();
    let version = Command::new(&tool).arg("--version").output().unwrap();
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8(version.stdout).unwrap().trim(),
        "wally-package-types 1.7.0"
    );
    for (label, source, expected) in [
        (
            "generic",
            "type ExternalType = { value: number }\nexport type Producer<State = any, Dispatchers = ExternalType> = { state: State, dispatchers: Dispatchers }\nreturn {}\n",
            "export type Producer<State, Dispatchers>",
        ),
        (
            "const",
            "const value: number = 5\nexport type WithConst = { value: number }\nreturn { value = value }\n",
            "export type WithConst",
        ),
    ] {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir_all(root.join("Packages/_Index/sample/Source")).unwrap();
        fs::write(root.join("Packages/_Index/sample/Source/init.luau"), source).unwrap();
        fs::write(
            root.join("Packages/Link.luau"),
            "return require(script.Parent._Index.sample.Source)\n",
        )
        .unwrap();
        let map = serde_json::json!({"name":"Fixture","className":"DataModel","children":[
            {"name":"Packages","className":"Folder","children":[
                {"name":"Link","className":"ModuleScript","filePaths":["Packages/Link.luau"]},
                {"name":"_Index","className":"Folder","children":[
                    {"name":"sample","className":"Folder","children":[
                        {"name":"Source","className":"ModuleScript","filePaths":["Packages/_Index/sample/Source/init.luau"]}
                    ]}
                ]}
            ]}
        ]});
        fs::write(
            root.join("sourcemap.json"),
            serde_json::to_vec(&map).unwrap(),
        )
        .unwrap();
        {
            let output = Command::new(&tool)
                .current_dir(root)
                .args(["--sourcemap", "sourcemap.json", "Packages"])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{label}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let link = fs::read_to_string(root.join("Packages/Link.luau")).unwrap();
            let compact: String = link.chars().filter(|ch| !ch.is_whitespace()).collect();
            let expected: String = expected.chars().filter(|ch| !ch.is_whitespace()).collect();
            assert!(compact.contains(&expected), "{label}: {link}");
            assert!(
                !compact.contains("State=any"),
                "invalid generic default order: {link}"
            );
            assert!(
                compact.contains("REQUIRED_MODULE"),
                "export was not restored: {link}"
            );
            assert_eq!(
                fs::read_to_string(root.join("Packages/_Index/sample/Source/init.luau")).unwrap(),
                source
            );
        }
    }
}
