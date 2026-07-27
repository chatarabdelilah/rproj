//! The project's quality gate: what `.lute/check.luau` runs, and what CI
//! runs it with.
//!
//! Data-driven: each step below declares the rokit tool it needs, the
//! `@std`/`@lute` modules it imports, and its Luau body. `render_check`
//! emits only the steps whose tool the project actually selected, imports
//! exactly the modules those steps use, and aggregates their results.
//! Adding a check is a `CHECK_STEPS` entry — no code changes.

use crate::config::PackageWorkflow;

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
fn targets(testez_selected: bool) -> &'static str {
    if testez_selected { r#""src", "tests""# } else { r#""src""# }
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
	-- not this project's to fix, and submodules in particular are pinned
	-- checkouts we don't control.
	"--ignore=**/submodules/**",
	"--ignore=**/Packages/**",
	"--ignore=**/ServerPackages/**",
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
pub fn render_check(selected_tools: &[String], testez_selected: bool) -> Option<String> {
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
        out.push_str(&format!("local {name} = require(\"{}\")\n", import_path(name)));
    }

    // Plain replace rather than `format!`: these bodies are Luau and full
    // of literal braces (`{ stdio = "inherit" }`), which a format string
    // would demand be doubled — turning every step's body into something
    // that no longer reads like the code it generates.
    let targets = targets(testez_selected);
    for step in &steps {
        out.push_str(&format!("\n-- {}\n{}\n", step.comment, step.body.replace("{targets}", targets)));
    }

    let vars: Vec<&str> = steps.iter().map(|s| s.result_var).filter(|v| !v.is_empty()).collect();
    if !vars.is_empty() {
        let condition =
            vars.iter().map(|v| format!("{v}.ok")).collect::<Vec<_>>().join(" and ");
        out.push_str(&format!(
            "\n-- Every check runs before exiting, so one run reports all problems\n\
             -- rather than stopping at the first.\n\
             if not ({condition}) then\n\tprocess.exit(1)\nend\n"
        ));
    }

    Some(out)
}

/// The wally-package-types commit CI builds from source.
///
/// Not a version, because there is no release to point at: the newest one
/// (`1.6.2`) generates Luau that doesn't parse for packages whose generics
/// mix defaulted and non-defaulted parameters - `remo` is one - and the fix
/// (PR #28) is merged but unreleased. See §7 of docs/architecture.md.
///
/// Pinned to a full sha rather than a branch so CI is reproducible, and to
/// *this* sha rather than PR #28's merge commit because it also carries #30
/// (the `full-moon` bump that parses the `const` keyword). It is the same
/// commit installed on the development machine: pinning CI to anything
/// earlier would mean the gate accepts locally what it rejects in CI, which
/// is the failure mode this file exists to prevent.
const WPT_FIXED_REV: &str = "daf5c97bf451e9fed47080769cfaa75d419eb768";

/// The steps that put this project's dependencies on disk in CI.
///
/// Wally's `Packages/` is gitignored, so a fresh checkout doesn't have it -
/// and `default.project.json` maps that path, which means rojo refuses to
/// generate a sourcemap and *every* step of the gate fails before it runs.
/// Nothing installed them, so the workflow was broken for every Wally
/// project from the moment it was written; it went unnoticed because
/// Windows resolves the mapped `packages` to wally's `Packages` anyway and
/// nobody had run the gate on the Linux runner.
///
/// Submodule projects need no equivalent - `submodules: true` on the
/// checkout already brings their packages in.
fn wally_ci_steps(has_server_packages: bool) -> String {
    // Naming a directory that doesn't exist is an error, and ServerPackages/
    // only exists when something server-realm was selected.
    let dirs = if has_server_packages { "Packages ServerPackages" } else { "Packages" };
    format!(
        r#"
      # wally-package-types 1.6.2 - the newest release, and what rokit.toml
      # pins - emits Luau that doesn't parse for packages whose generics mix
      # defaults and non-defaults. Fixed upstream but not yet released, so
      # the fix has to be built from a pinned commit. Delete this step and
      # the cache above it once a release past 1.6.2 exists, and call the
      # rokit-installed binary below instead.
      - name: Cache wally-package-types
        id: cache-wpt
        uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/wally-package-types
          key: wally-package-types-{WPT_FIXED_REV}

      - name: Build wally-package-types
        if: steps.cache-wpt.outputs.cache-hit != 'true'
        run: |
          cargo install --locked --git https://github.com/JohnnyMorganz/wally-package-types --rev {WPT_FIXED_REV}

      # Packages/ is gitignored, so it has to be installed here. The
      # retyping step matters as much as the install: wally rewrites every
      # link file without type information, so skipping it would have the
      # type checker see `any` for every package.
      #
      # wally-package-types is called by absolute path on purpose. rokit's
      # shim directory is also on PATH, and it holds the broken 1.6.2 - a
      # bare `wally-package-types` would silently resolve to whichever comes
      # first.
      - name: Install packages
        run: |
          wally install
          rojo sourcemap default.project.json --output sourcemap.json
          ~/.cargo/bin/wally-package-types --sourcemap sourcemap.json {dirs}
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
pub fn ci_workflow(workflow: PackageWorkflow, has_server_packages: bool) -> String {
    let install = match workflow {
        PackageWorkflow::Wally => wally_ci_steps(has_server_packages),
        PackageWorkflow::GitSubmodules => String::new(),
    };
    format!(
        r#"name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          # Packages vendored as git submodules are part of the build.
          submodules: true

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

      - name: Run tests
        run: lute test
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!script.contains("\"stylua\""), "stylua not selected:\n{script}");
        assert!(!script.contains("luau-lsp"), "luau-lsp not selected:\n{script}");
    }

    /// An unused local is exactly what selene/luau-lsp - which this very
    /// script runs - would flag, so the gate would fail itself.
    #[test]
    fn imports_only_what_the_emitted_steps_use() {
        let script = render_check(&tools(&["rojo", "stylua"]), false).unwrap();
        assert!(script.contains(r#"local process = require("@lute/process")"#));
        assert!(!script.contains("@std/fs"), "fs is unused here:\n{script}");
        assert!(!script.contains("@lute/net"), "net is unused here:\n{script}");

        // luau-lsp's step is the only one needing fs and net.
        let with_analyze = render_check(&tools(&["luau-lsp-cli"]), false).unwrap();
        assert!(with_analyze.contains("@std/fs"));
        assert!(with_analyze.contains("@lute/net"));
    }

    /// Every result variable must be both declared and aggregated, or the
    /// script fails to compile / silently ignores a failing check.
    #[test]
    fn aggregates_exactly_the_declared_result_vars() {
        let script = render_check(&tools(&["rojo", "luau-lsp-cli", "selene", "stylua"]), false).unwrap();
        for var in ["analyze", "selene", "stylua"] {
            assert!(script.contains(&format!("local {var} = process.run")), "{var} not declared");
            assert!(script.contains(&format!("{var}.ok")), "{var} not aggregated");
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
        assert!(with_tests.contains(r#"{ "selene", "src", "tests" }"#), "{with_tests}");
        assert!(with_tests.contains(r#"{ "stylua", "--check", "src", "tests" }"#), "{with_tests}");
        assert!(with_tests.contains("\t\"src\", \"tests\",\n"), "analyze:\n{with_tests}");

        // Without TestEZ there is no tests/ directory, and every one of
        // these tools errors on a path that doesn't exist - so naming it
        // would fail the gate on precisely the projects with no tests.
        let without = render_check(&tools, false).unwrap();
        assert!(!without.contains("tests"), "{without}");
        assert!(without.contains(r#"{ "selene", "src" }"#), "{without}");

        // The placeholder is an implementation detail; none may survive
        // into the generated Luau.
        for script in [&with_tests, &without] {
            assert!(!script.contains("{targets}"), "unsubstituted placeholder:\n{script}");
        }
    }

    /// A script whose only step is the sourcemap has nothing to fail on,
    /// so it must not emit a dangling `if not () then`.
    #[test]
    fn omits_exit_check_when_no_step_produces_a_result() {
        let script = render_check(&tools(&["rojo"]), false).unwrap();
        assert!(!script.contains("process.exit"), "nothing to gate on:\n{script}");
    }

    /// No selected tool means no gate, and callers use that to decide
    /// whether to write a CI workflow at all.
    #[test]
    fn renders_nothing_when_no_step_applies() {
        assert!(render_check(&tools(&["wally", "tarmac"]), false).is_none());
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
        for workflow in [PackageWorkflow::Wally, PackageWorkflow::GitSubmodules] {
            let ci = ci_workflow(workflow, false);
            assert!(ci.contains("lute run check"));
            assert!(ci.contains("lute test"));
            assert!(!ci.contains("lute run tests"));
            assert!(ci.contains("submodules: true"));
        }
    }

    /// Packages/ is gitignored, so without an install step the gate's very
    /// first action - generating a sourcemap over a mapped $path that
    /// doesn't exist - fails, and every Wally project's CI is red.
    #[test]
    fn wally_ci_installs_packages_and_restores_their_types() {
        let ci = ci_workflow(PackageWorkflow::Wally, false);
        assert!(ci.contains("wally install"), "{ci}");
        assert!(ci.contains("wally-package-types"), "{ci}");
        // The install must come before the gate, or it's pointless.
        assert!(
            ci.find("wally install") < ci.find("lute run check"),
            "packages must be installed before the gate runs:\n{ci}"
        );

        // Submodule projects get their packages from `submodules: true` and
        // don't pin wally at all - invoking it would fail on a missing
        // binary.
        let submodules = ci_workflow(PackageWorkflow::GitSubmodules, false);
        assert!(!submodules.contains("wally"), "{submodules}");
    }

    /// The released wally-package-types generates Luau that doesn't parse
    /// (§7), so CI builds a pinned commit. Three things have to hold or CI
    /// silently goes back to using the broken one.
    #[test]
    fn wally_ci_builds_the_fixed_wally_package_types() {
        let ci = ci_workflow(PackageWorkflow::Wally, false);

        // A branch or tag would make CI non-reproducible, and a short sha
        // is ambiguous.
        assert_eq!(WPT_FIXED_REV.len(), 40, "pin a full 40-char sha, not {WPT_FIXED_REV}");
        assert!(WPT_FIXED_REV.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(ci.contains(&format!("--rev {WPT_FIXED_REV}")), "{ci}");
        assert!(ci.contains("cargo install --locked"), "unpinned deps aren't reproducible:\n{ci}");

        // It must be built before it's used.
        assert!(
            ci.find("cargo install") < ci.find("--sourcemap sourcemap.json"),
            "the build has to precede the invocation:\n{ci}"
        );

        // And it must be invoked by absolute path: rokit's shim dir is also
        // on PATH and holds the broken 1.6.2, so a bare command name would
        // resolve to whichever came first - the exact bug this step exists
        // to avoid, reintroduced invisibly.
        assert!(
            ci.contains("~/.cargo/bin/wally-package-types --sourcemap"),
            "must call the built binary by path, not by name:\n{ci}"
        );
    }

    /// wally-package-types errors on a directory argument that doesn't
    /// exist, and `ServerPackages/` only exists when something server-realm
    /// was selected - so the argument list has to track the manifest.
    #[test]
    fn ci_retypes_server_packages_only_when_there_are_any() {
        let with = ci_workflow(PackageWorkflow::Wally, true);
        assert!(with.contains("sourcemap.json Packages ServerPackages"), "{with}");

        let without = ci_workflow(PackageWorkflow::Wally, false);
        assert!(without.contains("sourcemap.json Packages\n"), "{without}");
        assert!(!without.contains("ServerPackages"), "{without}");
    }

    /// Wally hardcodes `Packages`; the lowercase spelling only resolves on
    /// a case-insensitive filesystem, which the Linux runner is not.
    #[test]
    fn vendored_paths_use_wallys_real_capitalisation() {
        assert!(ANALYZE_BODY.contains("**/Packages/**"), "{ANALYZE_BODY}");
        assert!(ci_workflow(PackageWorkflow::Wally, false).contains("sourcemap.json Packages"));
    }
}
