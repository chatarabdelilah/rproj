//! The project's quality gate: what `.lute/check.luau` runs, and what CI
//! runs it with.
//!
//! Data-driven: each step below declares the rokit tool it needs, the
//! `@std`/`@lute` modules it imports, and its Luau body. `render_check`
//! emits only the steps whose tool the project actually selected, imports
//! exactly the modules those steps use, and aggregates their results.
//! Adding a check is a `CHECK_STEPS` entry — no code changes.

use crate::config::PackageWorkflow;
use crate::graph::TestRunner;

/// One command in the quality gate.
pub struct CheckStep {
    /// Catalog key of the rokit tool this step invokes. The step is only
    /// emitted if the project selected that tool, so a project without
    /// (say) StyLua gets a check script that doesn't call it.
    pub tool_key: &'static str,
    /// Lute modules this step's body imports (`fs`, `net`, `process`).
    /// Unioned across emitted steps so the generated file never imports
    /// something it doesn't use — an unused local is exactly what the
    /// linters this script runs would complain about.
    pub imports: &'static [&'static str],
    /// Local holding this step's result, aggregated into the final exit
    /// status. Empty for steps whose failure shouldn't fail the gate.
    pub result_var: &'static str,
    /// Explains, in the generated file, what this step is checking.
    pub comment: &'static str,
    /// The step's Luau. `{targets}` is substituted with the comma-separated
    /// quoted paths this project checks — `"src"`, plus `"tests"` when the
    /// project has a TestEZ tree. Every step happens to want those in the
    /// same shape (elements of a Luau list), so one placeholder covers all
    /// of them and adding a step still needs no code change.
    pub body: &'static str,
}

/// Paths the gate checks.
///
/// `tests` is conditional because it only exists for TestEZ projects, and
/// every one of these tools errors on a path that isn't there — so naming it
/// unconditionally would fail the gate on exactly the projects that have no
/// tests to check.
fn targets(has_tests: bool) -> &'static str {
    if has_tests {
        r#""src", "tests""#
    } else {
        r#""src""#
    }
}

/// `luau-lsp analyze` needs Roblox's global type definitions, which aren't
/// shipped with the binary - they're published per-commit by the luau-lsp
/// project and fetched at check time, then deleted so they never end up
/// committed or stale.
///
/// API names here are lute 1.0.0's and were verified against the installed
/// typedefs, not copied from an existing project: older lute exposed
/// `fs.writestringtofile` and `net.request`, both of which are now
/// `fs.writeStringToFile` and `net.client.request` (`net` became a
/// namespace over `client`/`server`). The old spellings fail at runtime
/// with "attempt to call a nil value", not at type-check time.
const ANALYZE_BODY: &str = r#"fs.writeStringToFile(
	"roblox.d.luau",
	net.client.request("https://luau-lsp.pages.dev/globalTypes.None.d.luau", { method = "GET" }).body
)

local analyze = process.run({
	"luau-lsp",
	"analyze",
	"--sourcemap=sourcemap.json",
	-- Vendored dependencies are third-party code; their type errors are
	-- not this project's to fix.
	"--ignore=**/Packages/**",
	"--ignore=**/ServerPackages/**",
	"--ignore=**/DevPackages/**",
	"--base-luaurc=.luaurc",
	"--definitions=roblox.d.luau",
	"--flag:LuauSolverV2=true",
	{targets},
}, { stdio = "inherit" })

fs.remove("roblox.d.luau")"#;

pub const CHECK_STEPS: &[CheckStep] = &[
    CheckStep {
        tool_key: "rojo",
        imports: &["process"],
        result_var: "",
        comment: "Regenerate the sourcemap so the type checker resolves requires\n-- against the current file tree rather than a stale snapshot.",
        body: r#"process.run({ "rojo", "sourcemap", "--output=sourcemap.json" }, { stdio = "inherit" })"#,
    },
    CheckStep {
        tool_key: "luau-lsp-cli",
        imports: &["fs", "net", "process"],
        result_var: "analyze",
        comment: "Type-check the project with Roblox's API types and the new solver.",
        body: ANALYZE_BODY,
    },
    CheckStep {
        tool_key: "selene",
        imports: &["process"],
        result_var: "selene",
        comment: "Lint for suspicious constructs (selene.toml decides severity).",
        body: r#"local selene = process.run({ "selene", {targets} }, { stdio = "inherit" })"#,
    },
    CheckStep {
        tool_key: "stylua",
        imports: &["process"],
        result_var: "stylua",
        comment: "Fail if anything isn't formatted, rather than reformatting it here -\n-- CI must not rewrite the tree it was asked to check.",
        body: r#"local stylua = process.run({ "stylua", "--check", {targets} }, { stdio = "inherit" })"#,
    },
];

/// Which `@std`/`@lute` module each import name comes from.
fn import_path(name: &str) -> &'static str {
    match name {
        "fs" => "@std/fs",
        "net" => "@lute/net",
        "process" => "@lute/process",
        other => panic!("unknown lute import `{other}` in CHECK_STEPS"),
    }
}

/// Builds `.lute/check.luau` for a project that selected `selected_tools`.
/// Returns `None` if no step applies, so callers don't write an empty
/// script (or a CI workflow that runs one).
pub fn render_check(selected_tools: &[String], has_tests: bool) -> Option<String> {
    let steps: Vec<&CheckStep> = CHECK_STEPS
        .iter()
        .filter(|s| selected_tools.iter().any(|t| t == s.tool_key))
        .collect();
    if steps.is_empty() {
        return None;
    }

    let mut imports: Vec<&str> = Vec::new();
    for step in &steps {
        for name in step.imports {
            if !imports.contains(name) {
                imports.push(name);
            }
        }
    }
    imports.sort_unstable();

    let mut out = String::from(
        "--!strict\n\
         -- Generated by `rproj new`. The project's quality gate.\n\
         -- Run locally with `lute run check`; CI runs the same script.\n\n",
    );
    for name in &imports {
        out.push_str(&format!(
            "local {name} = require(\"{}\")\n",
            import_path(name)
        ));
    }

    // Plain replace rather than `format!`: these bodies are Luau and full
    // of literal braces (`{ stdio = "inherit" }`), which a format string
    // would demand be doubled — turning every step's body into something
    // that no longer reads like the code it generates.
    let targets = targets(has_tests);
    for step in &steps {
        out.push_str(&format!(
            "\n-- {}\n{}\n",
            step.comment,
            step.body.replace("{targets}", targets)
        ));
    }

    let vars: Vec<&str> = steps
        .iter()
        .map(|s| s.result_var)
        .filter(|v| !v.is_empty())
        .collect();
    if !vars.is_empty() {
        let condition = vars
            .iter()
            .map(|v| format!("{v}.ok"))
            .collect::<Vec<_>>()
            .join(" and ");
        out.push_str(&format!(
            "\n-- Every check runs before exiting, so one run reports all problems\n\
             -- rather than stopping at the first.\n\
             if not ({condition}) then\n\tprocess.exit(1)\nend\n"
        ));
    }

    Some(out)
}

/// The steps that put this project's dependencies on disk in CI.
///
/// Wally's `Packages/` is gitignored, so a fresh checkout doesn't have it -
/// and `default.project.json` maps that path, which means rojo refuses to
/// generate a sourcemap and *every* step of the gate fails before it runs.
/// Nothing installed them, so the workflow was broken for every Wally
/// project from the moment it was written; it went unnoticed because
/// Windows resolves the mapped `packages` to wally's `Packages` anyway and
/// nobody had run the gate on the Linux runner.
fn wally_ci_steps(has_server_packages: bool, runner: Option<TestRunner>) -> String {
    // Naming a directory that doesn't exist is an error, and ServerPackages/
    // only exists when something server-realm was selected.
    let mut dirs = vec!["Packages"];
    if has_server_packages {
        dirs.push("ServerPackages");
    }
    if runner == Some(TestRunner::JestRoblox) {
        dirs.push("DevPackages");
    }
    let dirs = dirs.join(" ");
    let project_file = if runner == Some(TestRunner::JestRoblox) {
        "jest.project.json"
    } else {
        "default.project.json"
    };
    format!(
        r#"
      # Packages/ is gitignored, so it has to be installed here. The
      # retyping step matters as much as the install: wally rewrites every
      # link file without type information, so skipping it would have the
      # type checker see `any` for every package.
      - name: Install packages
        run: |
          wally install
          mkdir -p Packages
          rojo sourcemap {project_file} --output sourcemap.json
          wally-package-types --sourcemap sourcemap.json {dirs}
"#
    )
}

/// The GitHub Actions workflow that runs the gate on every push and PR.
///
/// `lute test` rather than `lute run tests`: the latter needs a `tests`
/// script to exist and hard-errors when it doesn't, which a freshly
/// scaffolded project has no reason to have. `lute test` discovers
/// `.test.luau`/`.spec.luau` files and exits 0 when there are none, so
/// this workflow is green on a new project and still runs tests once
/// there are some.
pub fn ci_workflow(
    workflow: PackageWorkflow,
    has_server_packages: bool,
    runner: Option<TestRunner>,
    backend: crate::graph::JestBackend,
) -> String {
    let install = match workflow {
        PackageWorkflow::Wally => wally_ci_steps(has_server_packages, runner),
        // Submodules arrive with the checkout, and a project with no
        // dependency manager has nothing to install in the first place.
        PackageWorkflow::None => String::new(),
    };
    let tests = match runner {
        None => String::new(),
        Some(TestRunner::TestEz) => {
            "\n      - name: Run tests\n        run: lute test\n".to_string()
        }
        Some(TestRunner::JestRoblox) if backend == crate::graph::JestBackend::Studio => String::new(),
        Some(TestRunner::JestRoblox) => r#"
      # In repository Settings > Secrets and variables > Actions, set secret
      # ROBLOX_OPEN_CLOUD_API_KEY and variables ROBLOX_UNIVERSE_ID/ROBLOX_PLACE_ID.
      # Use a dedicated test place: the runner uploads its test build there.
      # Key scopes: universe-places:write, universe.place.luau-execution-session:write,
      # memory-store.sorted-map:read and memory-store.sorted-map:write.
      - name: Check Jest Roblox credentials
        env:
          ROBLOX_OPEN_CLOUD_API_KEY: ${{ secrets.ROBLOX_OPEN_CLOUD_API_KEY }}
          ROBLOX_UNIVERSE_ID: ${{ vars.ROBLOX_UNIVERSE_ID }}
          ROBLOX_PLACE_ID: ${{ vars.ROBLOX_PLACE_ID }}
        run: |
          if [ -z "$ROBLOX_OPEN_CLOUD_API_KEY" ] || [ -z "$ROBLOX_UNIVERSE_ID" ] || [ -z "$ROBLOX_PLACE_ID" ]; then
            echo "Jest Roblox requires secret ROBLOX_OPEN_CLOUD_API_KEY and variables ROBLOX_UNIVERSE_ID and ROBLOX_PLACE_ID."
            exit 1
          fi

      - name: Run tests
        env:
          ROBLOX_OPEN_CLOUD_API_KEY: ${{ secrets.ROBLOX_OPEN_CLOUD_API_KEY }}
          ROBLOX_UNIVERSE_ID: ${{ vars.ROBLOX_UNIVERSE_ID }}
          ROBLOX_PLACE_ID: ${{ vars.ROBLOX_PLACE_ID }}
        run: jest-roblox-cli --backend open-cloud --formatters github-actions --passWithNoTests
"#
        .to_string(),
    };
    format!(
        r#"name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7

      # Installs every tool pinned in rokit.toml (rojo, luau-lsp, selene,
      # stylua, lute...) at the exact versions this project uses.
      - uses: CompeyDev/setup-rokit@v0.2.1
{install}
      # Writes the ~/.lute typedef aliases into .luaurc. Merges into the
      # committed file rather than replacing it, so languageMode survives.
      - name: Set up Lute
        run: lute setup --with-luaurc

      - name: Check code quality
        run: lute run check
{tests}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, process::Command};

    fn bash() -> PathBuf {
        if cfg!(windows) {
            let git = Command::new("git")
                .arg("--exec-path")
                .output()
                .expect("Git for Windows is required for the generated CI shell test");
            assert!(git.status.success(), "git --exec-path failed");
            let exec_path = PathBuf::from(String::from_utf8(git.stdout).unwrap().trim());
            exec_path
                .ancestors()
                .flat_map(|root| [root.join("bin/bash.exe"), root.join("usr/bin/bash.exe")])
                .find(|path| path.is_file())
                .expect("Git for Windows must include Bash")
        } else {
            PathBuf::from("bash")
        }
    }

    #[test]
    fn cloud_ci_refuses_missing_credentials_before_the_runner() {
        let ci = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::JestRoblox),
            crate::graph::JestBackend::OpenCloud,
        );
        let (_, credential_step) = ci
            .split_once("      - name: Check Jest Roblox credentials\n")
            .expect("credential check precedes the test runner");
        let (credential_step, _) = credential_step
            .split_once("\n      - name: Run tests")
            .expect("test runner follows the credential check");
        let (_, body) = credential_step.split_once("        run: |\n").unwrap();
        let script = body
            .lines()
            .map(|line| line.strip_prefix("          ").unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        // Execute the generated guard, then an offline stand-in for the next
        // step. No real runner is launched even in the all-present case.
        let script = format!("{script}\nprintf 'credential-check-passed\\n'\n");
        let credentials = [
            ("ROBLOX_OPEN_CLOUD_API_KEY", "rproj-fixture-key"),
            ("ROBLOX_UNIVERSE_ID", "1"),
            ("ROBLOX_PLACE_ID", "2"),
        ];
        let shell = bash();
        let project = tempfile::tempdir().unwrap();
        for empty in [false, true] {
            for present in 0_u8..8 {
                let mut command = Command::new(&shell);
                command
                    .args(["--noprofile", "--norc", "-e", "-c", &script])
                    .current_dir(project.path())
                    .env_remove("BASH_ENV")
                    .env_remove("ENV");
                for (index, (name, value)) in credentials.iter().enumerate() {
                    if present & (1 << index) != 0 {
                        command.env(name, value);
                    } else if empty {
                        command.env(name, "");
                    } else {
                        command.env_remove(name);
                    }
                }
                let output = command.output().unwrap();
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                if present == 7 {
                    assert!(output.status.success(), "{text}");
                    assert_eq!(text.trim(), "credential-check-passed");
                } else {
                    assert_eq!(output.status.code(), Some(1), "{text}");
                    assert!(!text.contains("credential-check-passed"), "{text}");
                    for (name, _) in credentials {
                        assert!(text.contains(name), "missing guidance for {name}: {text}");
                    }
                }
                assert!(!text.contains("rproj-fixture-key"), "{text}");
            }
        }
    }

    #[test]
    fn local_jest_ci_has_no_test_steps_or_cloud_configuration() {
        let ci = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::JestRoblox),
            crate::graph::JestBackend::Studio,
        );
        assert!(ci.contains("lute run check"));
        assert!(ci.contains("jest.project.json"));
        assert!(!ci.contains("Run tests"));
        assert!(!ci.contains("ROBLOX_OPEN_CLOUD"));
        assert!(!ci.contains("JEST_OPEN_CLOUD"));
        let cloud = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::JestRoblox),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(cloud.contains("Run tests"));
        assert!(cloud.contains("exit 1"));
        assert!(!cloud.contains("JEST_OPEN_CLOUD"));
    }

    fn tools(keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| (*k).to_string()).collect()
    }

    /// The gate must never invoke a tool the project didn't install -
    /// that's a guaranteed CI failure on a valid project.
    #[test]
    fn only_emits_steps_for_selected_tools() {
        let script = render_check(&tools(&["rojo", "selene"]), false).unwrap();
        assert!(script.contains("\"rojo\""));
        assert!(script.contains("\"selene\""));
        assert!(
            !script.contains("\"stylua\""),
            "stylua not selected:\n{script}"
        );
        assert!(
            !script.contains("luau-lsp"),
            "luau-lsp not selected:\n{script}"
        );
    }

    /// An unused local is exactly what selene/luau-lsp - which this very
    /// script runs - would flag, so the gate would fail itself.
    #[test]
    fn imports_only_what_the_emitted_steps_use() {
        let script = render_check(&tools(&["rojo", "stylua"]), false).unwrap();
        assert!(script.contains(r#"local process = require("@lute/process")"#));
        assert!(!script.contains("@std/fs"), "fs is unused here:\n{script}");
        assert!(
            !script.contains("@lute/net"),
            "net is unused here:\n{script}"
        );

        // luau-lsp's step is the only one needing fs and net.
        let with_analyze = render_check(&tools(&["luau-lsp-cli"]), false).unwrap();
        assert!(with_analyze.contains("@std/fs"));
        assert!(with_analyze.contains("@lute/net"));
    }

    /// Every result variable must be both declared and aggregated, or the
    /// script fails to compile / silently ignores a failing check.
    #[test]
    fn aggregates_exactly_the_declared_result_vars() {
        let script =
            render_check(&tools(&["rojo", "luau-lsp-cli", "selene", "stylua"]), false).unwrap();
        for var in ["analyze", "selene", "stylua"] {
            assert!(
                script.contains(&format!("local {var} = process.run")),
                "{var} not declared"
            );
            assert!(
                script.contains(&format!("{var}.ok")),
                "{var} not aggregated"
            );
        }
        // rojo's step declares no result var, so it must not be aggregated.
        assert!(!script.contains("rojo.ok"));
        assert!(script.contains("process.exit(1)"));
    }

    /// Before this, every step checked `src` only, so a broken or
    /// unformatted spec passed the gate and only surfaced when someone ran
    /// the tests. Verified against a real project: a type error, an
    /// undefined global and a spaces-instead-of-tabs spec each take the
    /// gate from 0 to 1.
    #[test]
    fn testez_projects_check_their_tests_too() {
        let tools = tools(&["rojo", "luau-lsp-cli", "selene", "stylua"]);

        let with_tests = render_check(&tools, true).unwrap();
        assert!(
            with_tests.contains(r#"{ "selene", "src", "tests" }"#),
            "{with_tests}"
        );
        assert!(
            with_tests.contains(r#"{ "stylua", "--check", "src", "tests" }"#),
            "{with_tests}"
        );
        assert!(
            with_tests.contains("\t\"src\", \"tests\",\n"),
            "analyze:\n{with_tests}"
        );

        // Without TestEZ there is no tests/ directory, and every one of
        // these tools errors on a path that doesn't exist - so naming it
        // would fail the gate on precisely the projects with no tests.
        let without = render_check(&tools, false).unwrap();
        assert!(!without.contains("tests"), "{without}");
        assert!(without.contains(r#"{ "selene", "src" }"#), "{without}");

        // The placeholder is an implementation detail; none may survive
        // into the generated Luau.
        for script in [&with_tests, &without] {
            assert!(
                !script.contains("{targets}"),
                "unsubstituted placeholder:\n{script}"
            );
        }
    }

    /// A script whose only step is the sourcemap has nothing to fail on,
    /// so it must not emit a dangling `if not () then`.
    #[test]
    fn omits_exit_check_when_no_step_produces_a_result() {
        let script = render_check(&tools(&["rojo"]), false).unwrap();
        assert!(
            !script.contains("process.exit"),
            "nothing to gate on:\n{script}"
        );
    }

    /// No selected tool means no gate, and callers use that to decide
    /// whether to write a CI workflow at all.
    #[test]
    fn renders_nothing_when_no_step_applies() {
        assert!(render_check(&tools(&["wally", "asphalt"]), false).is_none());
        assert!(render_check(&[], false).is_none());
    }

    /// Guards the `import_path` panic: every import named by a step must
    /// map to a real module.
    #[test]
    fn every_declared_import_resolves() {
        for step in CHECK_STEPS {
            for name in step.imports {
                let path = import_path(name);
                assert!(path.starts_with('@'), "{name} -> {path}");
            }
        }
    }

    /// CI must run the tools it installs, and `lute run tests` hard-errors
    /// without a tests script - `lute test` exits 0 when none are found.
    #[test]
    fn ci_workflow_runs_the_gate_and_tolerates_no_tests() {
        for &workflow in PackageWorkflow::ALL {
            let ci = ci_workflow(
                workflow,
                false,
                Some(TestRunner::TestEz),
                crate::graph::JestBackend::OpenCloud,
            );
            assert!(ci.contains("lute run check"));
            assert!(ci.contains("lute test"));
            assert!(!ci.contains("lute run tests"));
            assert!(!ci.contains("submodules:"), "{ci}");
        }
    }

    #[test]
    fn ci_omits_tests_when_testing_is_disabled() {
        let ci = ci_workflow(
            PackageWorkflow::None,
            false,
            None,
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(ci.contains("lute run check"));
        assert!(!ci.contains("Run tests"), "{ci}");
    }

    #[test]
    fn jest_ci_uses_dev_packages_and_fails_when_credentials_are_missing() {
        let ci = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::JestRoblox),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(ci.contains("jest.project.json"), "{ci}");
        assert!(ci.contains("Packages DevPackages"), "{ci}");
        assert!(ci.contains("ROBLOX_OPEN_CLOUD_API_KEY"), "{ci}");
        assert!(ci.contains("ROBLOX_UNIVERSE_ID"), "{ci}");
        assert!(ci.contains("ROBLOX_PLACE_ID"), "{ci}");
        assert!(ci.contains("exit 1"), "{ci}");
        assert!(ci.contains("--backend open-cloud"), "{ci}");
        assert!(ci.contains("--formatters github-actions"), "{ci}");
    }

    /// Packages/ is gitignored, so without an install step the gate's very
    /// first action - generating a sourcemap over a mapped $path that
    /// doesn't exist - fails, and every Wally project's CI is red.
    #[test]
    fn wally_ci_installs_packages_and_restores_their_types() {
        let ci = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::TestEz),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(ci.contains("wally install"), "{ci}");
        assert!(ci.contains("mkdir -p Packages"), "{ci}");
        assert!(ci.contains("wally-package-types"), "{ci}");
        // The install must come before the gate, or it's pointless.
        assert!(
            ci.find("wally install") < ci.find("lute run check"),
            "packages must be installed before the gate runs:\n{ci}"
        );

        let without_dependencies = ci_workflow(
            PackageWorkflow::None,
            false,
            Some(TestRunner::TestEz),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(
            !without_dependencies.contains("wally"),
            "{without_dependencies}"
        );
    }

    #[test]
    fn wally_ci_uses_the_released_rokit_tool() {
        let ci = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::TestEz),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(
            ci.contains("wally-package-types --sourcemap sourcemap.json Packages"),
            "{ci}"
        );
        for obsolete in [
            "cargo install",
            "cache-wpt",
            "Cache wally-package-types",
            "~/.cargo/bin/wally-package-types",
            "daf5c97",
        ] {
            assert!(!ci.contains(obsolete), "{ci}");
        }
        assert!(ci.find("setup-rokit") < ci.find("wally install"), "{ci}");
        assert!(
            ci.find("rojo sourcemap") < ci.find("wally-package-types --sourcemap"),
            "{ci}"
        );
    }

    /// wally-package-types errors on a directory argument that doesn't
    /// exist, and `ServerPackages/` only exists when something server-realm
    /// was selected - so the argument list has to track the manifest.
    #[test]
    fn ci_retypes_server_packages_only_when_there_are_any() {
        let with = ci_workflow(
            PackageWorkflow::Wally,
            true,
            Some(TestRunner::TestEz),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(
            with.contains("sourcemap.json Packages ServerPackages"),
            "{with}"
        );

        let without = ci_workflow(
            PackageWorkflow::Wally,
            false,
            Some(TestRunner::TestEz),
            crate::graph::JestBackend::OpenCloud,
        );
        assert!(without.contains("sourcemap.json Packages\n"), "{without}");
        assert!(!without.contains("ServerPackages"), "{without}");
    }

    /// Wally hardcodes `Packages`; the lowercase spelling only resolves on
    /// a case-insensitive filesystem, which the Linux runner is not.
    #[test]
    fn vendored_paths_use_wallys_real_capitalisation() {
        assert!(ANALYZE_BODY.contains("**/Packages/**"), "{ANALYZE_BODY}");
        assert!(
            ci_workflow(
                PackageWorkflow::Wally,
                false,
                Some(TestRunner::TestEz),
                crate::graph::JestBackend::OpenCloud
            )
            .contains("sourcemap.json Packages")
        );
    }
}
