use std::process::{Command, Stdio};

fn output(args: &[&str]) -> String {
    let result = Command::new(env!("CARGO_BIN_EXE_rproj"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(result.status.success(), "{args:?}: {:?}", result.stderr);
    String::from_utf8(result.stdout)
        .unwrap()
        .replace("\r\n", "\n")
}

// Keep the captured fixture unchanged. The one intentional output correction
// closes the outer call in the pre-existing Jest sample (including worker JSON).
fn corrected_sample(text: &str) -> String {
    let old = "JestGlobals.describe(\"addition\", function()\n    JestGlobals.it(\"adds two numbers\", function()\n        JestGlobals.expect(2 + 3).toBe(5)\n    end)\nend";
    let new = format!("{old})");
    let escaped_old = serde_json::to_string(old).unwrap();
    let escaped_new = serde_json::to_string(&new).unwrap();
    text.replace(old, &new).replace(
        &escaped_old[1..escaped_old.len() - 1],
        &escaped_new[1..escaped_new.len() - 1],
    )
}

#[test]
fn terminal_and_private_package_catalog_preserve_output_except_corrected_jest_sample() {
    let baseline: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/catalog-baseline.json")).unwrap();
    assert_eq!(output(&["info"]), baseline["listing"].as_str().unwrap());
    for (key, detail) in baseline["details"].as_object().unwrap() {
        assert_eq!(
            output(&["info", key]),
            corrected_sample(detail.as_str().unwrap()),
            "{key}"
        );
    }
    assert_eq!(
        output(&["info", "__catalog_fixture_unknown__"]),
        baseline["unknown"].as_str().unwrap()
    );
    assert_eq!(
        output(&[rproj_core::operations::WORKER_FLAG, "catalog"]),
        corrected_sample(baseline["worker"].as_str().unwrap())
    );
}
