//! TestEZ wiring for a scaffolded project.
//!
//! Selecting `testez` as a package only puts the *library* in the tree.
//! Two more things are needed before tests actually run:
//!
//! - `selene.toml` must use `std = "roblox+testez"`, or every `describe`/
//!   `it`/`expect` is reported as an undefined global. (Handled by
//!   `steps::toolchain::ensure_selene_config`.)
//! - ...and that `+testez` half has to *resolve*, which means a
//!   `testez.yml` standard-library file next to `selene.toml`. Without it
//!   selene doesn't fall back or warn - it refuses to run at all
//!   ("Could not find all standard library files"), so selecting TestEZ
//!   silently disabled linting for the whole project.
//! - The TestEZ Companion Studio plugin needs a `testez-companion.toml`
//!   telling it which DataModel locations hold `.spec` files.
//! - ...and selene's standard library says nothing to *luau-lsp*, which
//!   keeps its own idea of what globals exist and reports every
//!   `describe`/`it`/`expect` as `Unknown global` under `languageMode:
//!   strict`. That needs a `tests/.luaurc`.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::ui;

/// DataModel locations holding test files, as TestEZ Companion wants them:
/// service-rooted, slash-separated, and searched recursively (the plugin
/// finds `.spec` files as descendants).
///
/// These name the `test` instances `steps::rojo` mounts from `tests/` when
/// TestEZ is selected. A root naming an instance the tree doesn't create
/// finds no tests, and the plugin reporting nothing to run is
/// indistinguishable from every test passing - so these two must be
/// changed together.
pub const TEST_ROOTS: &[&str] = &[
    "ReplicatedStorage/test",
    "ServerScriptService/test",
    "StarterPlayer/StarterPlayerScripts/test",
];

/// One starter spec per test folder. Keeps the folders in git (which
/// doesn't track empty directories, so a fresh clone would otherwise be
/// missing paths the project file maps) and shows the TestEZ shape: a
/// module returning a function, with `describe`/`it` inside.
const STARTER_SPECS: &[(&str, &str)] = &[
    ("tests/shared/hello.spec.luau", "shared"),
    ("tests/server/hello.spec.luau", "server"),
    ("tests/client/hello.spec.luau", "client"),
];

/// Written with explicit `\n` and `\t` escapes rather than a multi-line
/// literal: a wrapped literal carries its own source indentation into the
/// generated file, which then fails the project's own `stylua --check` on
/// the very first run of the quality gate.
///
/// Tabs because that's what the scaffolded `stylua.toml` sets
/// (`indent_type = "Tabs"`), so the file it writes is already formatted.
fn starter_spec(area: &str) -> String {
    format!(
        "--!strict\n\nreturn function()\n\
         \tdescribe(\"{area}\", function()\n\
         \t\tit(\"runs\", function()\n\
         \t\t\texpect(true).to.equal(true)\n\
         \t\tend)\n\
         \tend)\n\
         end\n"
    )
}

/// Creates `tests/{shared,server,client}` with a starter spec in each.
pub fn ensure_test_folders(project_dir: &Path) -> Result<()> {
    let mut written = Vec::new();
    for (rel_path, area) in STARTER_SPECS {
        let file = project_dir.join(rel_path);
        fs::create_dir_all(file.parent().expect("spec paths have a parent"))?;
        if file.exists() {
            continue;
        }
        fs::write(&file, starter_spec(area))
            .with_context(|| format!("failed to write {}", file.display()))?;
        written.push(*area);
    }
    if written.is_empty() {
        ui::ok("tests/ already present");
    } else {
        ui::ok(&format!("created tests/ ({})", written.join(", ")));
    }
    Ok(())
}

pub fn companion_config() -> String {
    let roots = TEST_ROOTS
        .iter()
        .map(|r| format!("\t\"{r}\",\n"))
        .collect::<String>();
    format!(
        "# Where TestEZ Companion looks for .spec files. Descendants are\n\
         # included, so these are the three `test` instances rproj maps in\n\
         # default.project.json from tests/shared, tests/server, tests/client.\n\
         roots = [\n{roots}]\n"
    )
}

/// Selene's TestEZ globals, in its standard-library format.
///
/// Taken from rojo-rbx/rojo's own `testez.yml` rather than written from
/// memory - it covers the modifier variants (`itFOCUS`, `describeSKIP`,
/// `FIXME`...) that are easy to forget and that would each be reported as
/// an undefined global.
///
/// Must be `.yml`: selene does not recognise a `.yaml` extension when
/// resolving standard libraries.
const TESTEZ_STD: &str = r#"---
globals:
  FIXME:
    args:
      - required: false
        type: string
  FOCUS:
    args: []
  SKIP:
    args: []
  afterAll:
    args:
      - type: function
  afterEach:
    args:
      - type: function
  beforeAll:
    args:
      - type: function
  beforeEach:
    args:
      - type: function
  describe:
    args:
      - type: string
      - type: function
  describeFOCUS:
    args:
      - type: string
      - type: function
  describeSKIP:
    args:
      - type: string
      - type: function
  expect:
    args:
      - type: any
  it:
    args:
      - type: string
      - type: function
  itFIXME:
    args:
      - type: string
      - type: function
  itFOCUS:
    args:
      - type: string
      - type: function
  itSKIP:
    args:
      - type: string
      - type: function
"#;

/// Writes `testez.yml` so `std = "roblox+testez"` resolves.
pub fn ensure_selene_std(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("testez.yml");
    if path.exists() {
        ui::ok("testez.yml already exists");
        return Ok(());
    }
    fs::write(&path, TESTEZ_STD).with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote testez.yml (selene standard library)");
    Ok(())
}

/// The globals TestEZ injects into a spec file, for luau-lsp.
///
/// Same set as `TESTEZ_STD` above, which is selene's copy - the two tools
/// don't read each other's configuration, so both need telling and a test
/// below keeps them from drifting apart.
const TESTEZ_GLOBALS: &[&str] = &[
    "FIXME",
    "FOCUS",
    "SKIP",
    "afterAll",
    "afterEach",
    "beforeAll",
    "beforeEach",
    "describe",
    "describeFOCUS",
    "describeSKIP",
    "expect",
    "it",
    "itFIXME",
    "itFOCUS",
    "itSKIP",
];

/// Writes `tests/.luaurc` declaring TestEZ's globals to luau-lsp.
///
/// Without it, every spec file the scaffold generates opens with three red
/// errors ("Unknown global 'describe'; consider assigning to it first") in
/// a brand new project - TestEZ injects these at runtime, so nothing in the
/// source ever declares them and `languageMode: strict` is right to
/// complain.
///
/// Scoped to `tests/` rather than added to the project's root `.luaurc` on
/// purpose. Luau resolves configuration by walking from the root down to
/// the file, with nearer files layered *over* the ones above rather than
/// replacing them, so this one keeps the root's `languageMode` and lute
/// aliases while adding the globals - and `describe` stays an unknown
/// global in `src/`, where calling it really would be a mistake. (All four
/// halves of that verified with `luau-lsp analyze`.)
pub fn ensure_tests_luaurc(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("tests").join(".luaurc");

    let mut config = if path.exists() {
        let text = fs::read_to_string(&path)?;
        match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => map,
            // .luaurc allows comments, which serde_json rejects - leave a
            // file we can't parse alone rather than discarding it.
            _ => {
                ui::skip("tests/.luaurc exists but couldn't be parsed, leaving it alone");
                return Ok(());
            }
        }
    } else {
        serde_json::Map::new()
    };

    if config.contains_key("globals") {
        ui::ok("tests/.luaurc already configured");
        return Ok(());
    }
    config.insert("globals".to_string(), json!(TESTEZ_GLOBALS));

    fs::create_dir_all(path.parent().expect("joined path has a parent"))?;
    fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&Value::Object(config))?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote tests/.luaurc (TestEZ globals)");
    Ok(())
}

pub fn ensure_companion_config(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("testez-companion.toml");
    if path.exists() {
        ui::ok("testez-companion.toml already exists");
        return Ok(());
    }
    fs::write(&path, companion_config())
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote testez-companion.toml");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A root that doesn't match the generated tree finds nothing, and
    /// "no tests found" is indistinguishable from "all tests passed".
    #[test]
    fn roots_are_service_rooted_and_match_the_scaffolded_tree() {
        let config = companion_config();
        for root in TEST_ROOTS {
            assert!(config.contains(root), "{root} missing from:\n{config}");
            assert!(!root.starts_with('/'), "{root} should not start with a slash");
            assert!(!root.starts_with("game/"), "{root} should be service-rooted, not game-rooted");
        }
        assert!(config.starts_with('#'), "config should explain itself");
        assert!(config.contains("roots = ["));
    }

    /// selene refuses to run at all when a chained std can't be resolved,
    /// so the globals TestEZ projects rely on must all be declared.
    #[test]
    fn testez_std_declares_the_globals_selene_would_otherwise_reject() {
        for global in ["describe", "it", "expect", "beforeEach", "afterAll", "itFOCUS", "describeSKIP"] {
            assert!(TESTEZ_STD.contains(&format!("  {global}:")), "{global} missing from testez.yml");
        }
        assert!(TESTEZ_STD.starts_with("---"), "selene std files are YAML documents");
    }

    /// The roots must name the `test` instances the project file mounts
    /// from tests/, not the source folders.
    #[test]
    fn roots_name_the_test_instances_the_project_file_creates() {
        assert_eq!(
            TEST_ROOTS,
            ["ReplicatedStorage/test", "ServerScriptService/test", "StarterPlayer/StarterPlayerScripts/test"]
        );
    }

    /// selene and luau-lsp each need their own copy of TestEZ's globals and
    /// neither reads the other's config, so the two lists can drift apart
    /// and leave one tool flagging a global the other accepts.
    #[test]
    fn luau_lsp_globals_match_the_selene_standard_library() {
        let declared_to_selene: Vec<&str> = TESTEZ_STD
            .lines()
            // Globals are the two-space-indented keys under `globals:`;
            // their `args:`/`type:`/`required:` children are deeper.
            .filter(|l| l.starts_with("  ") && !l.starts_with("   ") && l.trim_end().ends_with(':'))
            .map(|l| l.trim().trim_end_matches(':'))
            .collect();
        assert!(!declared_to_selene.is_empty(), "parsed nothing out of testez.yml");
        assert_eq!(
            declared_to_selene, TESTEZ_GLOBALS,
            "selene's testez.yml and luau-lsp's tests/.luaurc must declare the same globals"
        );
    }

    /// A starter spec that doesn't parse as a TestEZ module would fail the
    /// first `lute test` on a brand new project.
    #[test]
    fn starter_specs_use_the_testez_shape() {
        let spec = starter_spec("shared");
        assert!(spec.starts_with("--!strict"), "{spec}");
        assert!(spec.contains("return function()"), "{spec}");
        assert!(spec.contains("describe(\"shared\""), "{spec}");
        assert!(spec.contains("it(\"runs\""), "{spec}");
        assert!(spec.trim_end().ends_with("end"), "{spec}");
        // A wrapped source literal bakes its own indentation into the
        // generated file, which then fails the project's own
        // `stylua --check` on the first run of the quality gate.
        for line in spec.lines() {
            assert!(
                !line.starts_with(' '),
                "generated spec must be tab-indented with no stray leading spaces:
{spec}"
            );
        }
    }
}
