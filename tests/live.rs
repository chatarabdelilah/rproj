//! The steps that shell out and touch the network, exercised for real.
//!
//! **Not part of `cargo test`.** Every test here is `#[ignore]`d because it
//! scaffolds a genuine project: it installs rokit tools, resolves packages
//! from a registry, clones repos, and runs the quality gate. That takes
//! minutes and needs a working toolchain and network, which is the wrong
//! trade for a suite meant to run on every edit.
//!
//! ```text
//! cargo test --test live -- --ignored --test-threads=1
//! ```
//!
//! `--test-threads=1` is not optional: these share rokit's global manifest
//! and wally's package cache, and two scaffolds racing on those is a flake
//! nobody can reproduce.
//!
//! What this covers that nothing else can: every §7 landmine was found by
//! running the real thing, and the four biggest gaps in §9 are all here —
//! the shell-out steps, `rproj new`'s pickers, the generated check script,
//! and §8.2's claim about how submodule packages resolve.

mod common;

// Jest regression only (requires an already provisioned Studio/Jest machine).
// Normal `rproj new` syncs shared Rokit/Wally caches and refreshes the Studio
// runner plugin; only the temporary project itself is isolated and removed.
// $env:RPROJ_TEST_TIMEOUT = '180'
// cargo test --locked --test live jest_starter_specs_pass_and_report_failure -- --ignored --test-threads=1 --nocapture

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{DOWN, ENTER, ESC, LEFT, Session};

/// A scaffolded project, removed when the test ends however it ends.
struct LiveProject {
    path: PathBuf,
}

impl LiveProject {
    /// Scaffolds through the expert picker, selecting each key in
    /// `packages` by typing enough of its name to filter the list down.
    ///
    /// Filtering rather than counting arrow presses: the catalog's order is
    /// not this test's business, and a test that breaks when a package is
    /// added is a test nobody keeps.
    fn scaffold(name: &str, packages: &[&str], submodules: bool) -> Self {
        let root = projects_root();
        let name = unique_name(name);
        let path = root.join(&name);

        let mut session = Session::start(&root, &["new", &name]);

        // **Dependency strategy first.** It used to come after the packages,
        // which is what let a React selection silently overrule it.
        session.wait_for("How should this project get its dependencies?");
        if submodules {
            session.send("git-submodules");
            session.wait_for("git-submodules - ");
        }
        session.send(ENTER);

        session.wait_for("How do you want to pick packages?");
        session.send(&format!("{DOWN}{ENTER}")); // expert
        session.wait_for("Pick every package this project needs");
        for key in packages.iter().filter(|key| **key != "testez") {
            session.send(key);
            session.wait_for(&format!("{key} - "));
            session.send(" ");
            // Clear the filter so the next key searches the whole list.
            session.send(&"\x7f".repeat(key.len()));
        }
        session.send(ENTER);

        // One capability prompt where there used to be two pickers (tools,
        // then files). Enter accepts the defaults, which is what every
        // assertion below assumes: lint, format, typecheck, gate, editor.
        session.wait_for("What should this project do?");
        for key in ["ci"]
            .into_iter()
            .chain(packages.contains(&"testez").then_some("test"))
        {
            session.send(key);
            session.wait_for(&format!("{key} - "));
            session.send(" ");
            session.send(&"\x7f".repeat(key.len()));
        }
        session.send(ENTER);
        if packages.contains(&"testez") && !submodules {
            session.wait_for("test:");
            session.send("testez");
            session.wait_for("testez - ");
            session.send(ENTER);
        }

        // The summary is not a picker - nothing here is a new decision, so
        // there is exactly one keystroke to confirm it.
        session.wait_for("Create it?");
        session.send(ENTER);

        session.wait_for("is ready");
        let outcome = session.finish();
        assert_eq!(outcome.code, 0, "scaffold failed:\n{}", outcome.text);

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path.join(relative))
            .unwrap_or_else(|e| panic!("reading {relative}: {e}"))
    }

    fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }

    /// Contents of the generated link file for `key` in `dir`, matched on
    /// the file stem so the test doesn't hard-code a naming convention that
    /// belongs to wally. Lists the directory on failure, since "the file
    /// isn't there" is never the useful half of that message.
    fn link_file(&self, dir: &str, key: &str) -> String {
        let path = self.path.join(dir);
        let entries: Vec<PathBuf> = std::fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();
        let found = entries.iter().find(|entry| {
            entry
                .file_stem()
                .is_some_and(|stem| stem.eq_ignore_ascii_case(key))
        });
        match found {
            Some(file) => std::fs::read_to_string(file).expect("read link file"),
            None => panic!(
                "no link file for `{key}` in {}. It holds: {}",
                path.display(),
                entries
                    .iter()
                    .filter_map(|e| e.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }

    /// Runs the project's own quality gate and returns its exit code.
    fn gate(&self) -> i32 {
        run(&self.path, "lute", &["run", "check"]).0
    }
}

impl Drop for LiveProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn projects_root() -> PathBuf {
    let dirs = directories::ProjectDirs::from("", "", "rproj").expect("config directory");
    let text = std::fs::read_to_string(dirs.config_dir().join("config.toml"))
        .expect("live tests require an already provisioned machine");
    let config: toml::Value = toml::from_str(&text).expect("valid machine configuration");
    assert!(
        config
            .get("last_checked")
            .and_then(toml::Value::as_str)
            .is_some(),
        "run machine setup explicitly before live tests"
    );
    let root = config
        .get("roblox_projects_root")
        .and_then(toml::Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"))
                .join("Documents/RobloxProjects")
        });
    assert!(
        root.is_dir(),
        "live test project root must already exist: {}",
        root.display()
    );
    root
}

fn unique_name(label: &str) -> String {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!(
        "rproj-audit-{label}-{}-{nanos}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    assert!(
        !projects_root().join(&name).exists(),
        "scratch project already exists"
    );
    name
}

/// Runs a toolchain command in `dir`, returning its exit code and output.
/// Tools are invoked through rokit's shims, the same way a developer's
/// shell would find them.
fn run(dir: &Path, tool: &str, args: &[&str]) -> (i32, String) {
    let shim = PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"))
        .join(".rokit")
        .join("bin")
        .join(format!("{tool}.exe"));
    let program = if shim.is_file() {
        shim
    } else {
        PathBuf::from(tool)
    };

    let output = Command::new(&program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawning {}: {e}", program.display()));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code().unwrap_or(-1), text)
}

/// Exercises the released CLI boundary, including scaffolded pins/config,
/// runner discovery, and failure propagation. A no-tests success or a tool
/// startup failure must never satisfy either half of this regression.
#[test]
#[ignore = "requires provisioned Rokit/Wally, Studio with JestRobloxRunner, network; run serially"]
fn jest_starter_specs_pass_and_report_failure() {
    let root = projects_root();
    let plugin = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA"))
        .join("Roblox/Plugins/JestRobloxRunner.rbxm");
    assert!(
        plugin.is_file(),
        "install the Jest Studio runner before running this live test"
    );

    // Own the parent before starting the scaffold, so even a partial scaffold
    // is cleaned up, without deleting or reusing any pre-existing directory.
    // A relative name keeps rproj's generated Wally package name short.
    let scratch = tempfile::Builder::new()
        .prefix("rproj-jest-")
        .tempdir_in(&root)
        .expect("create unique Jest scratch directory");
    let name = format!(
        "{}/project",
        scratch.path().file_name().unwrap().to_str().unwrap()
    );
    let project = scratch.path().join("project");
    let mut session = Session::start(&root, &["new", &name]);
    session.wait_for("How should this project get its dependencies?");
    session.send(ENTER); // Wally
    session.wait_for("How do you want to pick packages?");
    session.send(&format!("{DOWN}{ENTER}")); // expert
    session.wait_for("Pick every package this project needs");
    session.send(&format!("{LEFT}{ENTER}")); // Jest adds its own dev dependencies
    session.wait_for("What should this project do?");
    session.send(LEFT);
    session.send("test");
    session.wait_for("test - ");
    session.send(&format!(" {ENTER}"));
    session.wait_for("test:");
    session.send("jest-roblox");
    session.wait_for("jest-roblox - ");
    session.send(ENTER);
    session.wait_for("Create it?");
    session.send(ENTER);
    session.wait_for("is ready");
    let scaffold = session.finish();
    assert_eq!(scaffold.code, 0, "{}", scaffold.text);

    // Global shims must not hide a scaffold that forgot its project pins.
    let manifest: toml::Value = toml::from_str(
        &std::fs::read_to_string(project.join("rokit.toml")).expect("generated Rokit manifest"),
    )
    .expect("valid Rokit manifest");
    let tools = manifest["tools"].as_table().expect("project tool pins");
    for source in ["christopher-buss/jest-roblox-cli@", "UpliftGames/wally@"] {
        assert!(
            tools
                .values()
                .any(|pin| pin.as_str().is_some_and(|pin| pin.starts_with(source))),
            "missing {source} pin: {manifest}\n{}",
            scaffold.text
        );
    }

    let passing = Session::start(
        &project,
        &[
            "test",
            "--backend",
            "studio-cli",
            "--no-color",
            "--outputFile",
            "passing.json",
        ],
    )
    .finish();
    assert_eq!(passing.code, 0, "{}", passing.text);
    let report = |name: &str| -> serde_json::Value {
        let text = std::fs::read_to_string(project.join(name)).expect("Jest result report");
        serde_json::from_str(&text).expect("valid Jest result JSON")
    };
    let passed = report("passing.json");
    assert_eq!(passed["success"], true, "{passed}");
    assert_eq!(passed["numPassedTests"], 3, "{passed}\n{}", passing.text);
    assert_eq!(passed["numFailedTests"], 0, "{passed}");
    let suites = passed["testResults"].as_array().expect("per-file results");
    assert_eq!(suites.len(), 3, "{passed}");
    for area in ["shared", "server", "client"] {
        let path = format!("tests/{area}/hello.spec.luau");
        let suite = suites
            .iter()
            .find(|suite| suite["testFilePath"] == path)
            .unwrap_or_else(|| panic!("missing {path}: {passed}"));
        assert_eq!(suite["numPassingTests"], 1, "{suite}");
        assert_eq!(suite["testResults"][0]["status"], "passed", "{suite}");
    }

    let spec_path = project.join("tests/shared/hello.spec.luau");
    let spec = std::fs::read_to_string(&spec_path).expect("generated starter spec");
    assert_eq!(spec.matches("expect(1 + 1).toBe(2)").count(), 1, "{spec}");
    let broken = spec
        .replace("expect(1 + 1).toBe(2)", "expect(1 + 1).toBe(987654)")
        .replace("it(\"runs\"", "it(\"rproj deliberate failure\"");
    std::fs::write(&spec_path, &broken).expect("break owned starter spec");
    let failing = Session::start(
        &project,
        &[
            "test",
            "--backend",
            "studio-cli",
            "--no-color",
            "--outputFile",
            "failing.json",
        ],
    )
    .finish();
    assert_eq!(failing.code, 1, "{}", failing.text);
    failing.assert_contains("rproj deliberate failure");
    failing.assert_contains("987654");
    failing.assert_contains("reported test failures (exit code 1)");
    let failed = report("failing.json");
    assert_eq!(failed["numPassedTests"], 2, "{failed}");
    assert_eq!(failed["numFailedTests"], 1, "{failed}");
    assert_eq!(failed["success"], false, "{failed}");
    let failed_suites = failed["testResults"].as_array().expect("per-file results");
    assert_eq!(failed_suites.len(), 3, "{failed}");
    let shared = failed_suites
        .iter()
        .find(|suite| suite["testFilePath"] == "tests/shared/hello.spec.luau")
        .expect("shared starter spec result");
    assert_eq!(shared["numFailingTests"], 1, "{shared}");
    assert_eq!(shared["testResults"][0]["status"], "failed", "{shared}");
    assert_eq!(
        shared["testResults"][0]["title"], "rproj deliberate failure",
        "{shared}"
    );
    assert_eq!(
        std::fs::read_to_string(spec_path).unwrap(),
        broken,
        "rproj test must preserve the user's failing spec"
    );
    println!(
        "Jest regression: 3 starter specs passed; deliberate failure returned 1 with 2 passed / 1 failed."
    );
}

/// **"rproj needs to be such that if I want, I can create a project with only
/// the rojo init basic stuff."** Driven end to end, saying no to everything.
///
/// Lives in the live suite because it drives `rproj new`'s real prompts, but
/// unlike its neighbours it needs **no network and no toolchain**: with no
/// packages, no pinned tools and no files ticked, every step that would shell
/// out is gated off. So it is the cheapest test here and the one that proves
/// the property the artifact model exists for.
///
/// Two things make it bare, and both are deliberate. `none` is a real answer
/// to the dependency question, so no manifest is offered for dependencies
/// that don't exist. And the housekeeping files (`rproj.toml`, `.gitignore`)
/// are written for every project rather than asked about, so reaching the
/// absolute minimum goes through the summary's escape hatch — one extra
/// screen for the person who wants it, none for everyone else. This test is
/// what pins that hatch open.
#[test]
#[ignore]
fn saying_no_to_everything_yields_only_the_rojo_basics() {
    let root = projects_root();
    let name = unique_name("rproj-bare-basics");
    let path = root.join(&name);

    let mut session = Session::start(&root, &["new", &name]);
    // `none` is now a real answer to the dependency question, and choosing
    // it skips the package prompts entirely rather than defaulting to Wally
    // and then offering a manifest for dependencies that don't exist.
    session.wait_for("How should this project get its dependencies?");
    session.send("none");
    session.wait_for("none - ");
    session.send(ENTER);
    session.wait_for("What should this project do?");
    session.send(&format!("{LEFT}{ENTER}")); // no capabilities

    // The housekeeping entries (`rproj.toml`, `.gitignore`) are written for
    // every project rather than asked about, so reaching the *truly* bare
    // project goes through the summary's escape hatch. One extra screen for
    // the person who wants it, none for everyone else - and this test is
    // what proves the hatch is real rather than decorative.
    session.wait_for("Create it?");
    session.send("customize");
    session.wait_for("customize - ");
    session.send(ENTER);
    session.wait_for("Files to keep");
    session.send(&format!("{LEFT}{ENTER}")); // keep nothing optional
    session.wait_for("Create it?");
    session.send(ENTER);
    session.wait_for("is ready");

    let outcome = session.finish();
    assert_eq!(outcome.code, 0, "scaffold failed:\n{}", outcome.text);

    let mut entries: Vec<String> = std::fs::read_dir(&path)
        .expect("read the project dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        [".git", "default.project.json", "src"],
        "a minimal answer must produce nothing else:\n{}",
        outcome.text
    );

    // And the mandatory two are real, not empty placeholders.
    assert!(path.join("default.project.json").is_file());
    assert!(path.join("src").join("shared").is_dir());

    let _ = std::fs::remove_dir_all(&path);
}

#[test]
#[ignore = "requires an already provisioned machine"]
fn confirmation_and_cancellation_preserve_a_concurrently_created_directory() {
    for cancel in [true, false] {
        let root = projects_root();
        let name = unique_name("concurrent");
        let path = root.join(&name);
        let mut session = Session::start(&root, &["new", &name]);
        session.wait_for("How should this project get its dependencies?");
        session.send("none");
        session.wait_for("none - ");
        session.send(ENTER);
        session.wait_for("What should this project do?");
        session.send(&format!("{LEFT}{ENTER}"));
        session.wait_for("Create it?");
        assert!(!path.exists());
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("user.txt"), "preserve me").unwrap();
        let project = LiveProject { path };
        if cancel {
            session.send("cancel");
            session.wait_for("cancel - ");
        }
        session.send(ENTER);
        let outcome = session.finish();
        assert_eq!(outcome.code, if cancel { 0 } else { 1 }, "{}", outcome.text);
        assert_eq!(project.read("user.txt"), "preserve me");
        assert!(!project.exists("default.project.json"));
    }
}

/// **The redesign, end to end.** Pick a package, accept the capability
/// defaults, and the summary must *explain* every file rather than offering
/// it as a checkbox.
///
/// Stops at the summary with `ESC` instead of completing, so this needs no
/// install: what is being checked is the text on screen before any work
/// happens. The assertions are on the reasons, not just the keys — a line
/// naming `wally.toml` with no explanation would be a tool announcing
/// decisions, and the whole point is that the user can see which earlier
/// answer to change.
#[test]
#[ignore]
fn the_summary_explains_every_file_instead_of_offering_it_as_a_choice() {
    let root = projects_root();
    let name = unique_name("rproj-summary-report");
    let path = root.join(&name);

    let mut session = Session::start(&root, &["new", &name]);
    session.wait_for("How should this project get its dependencies?");
    session.send(ENTER); // Wally, the recommended default
    session.wait_for("How do you want to pick packages?");
    session.send(&format!("{DOWN}{ENTER}")); // expert
    session.wait_for("Pick every package this project needs");
    session.send("promise");
    session.wait_for("promise - ");
    session.send(" ");
    session.send(ENTER);

    session.wait_for("What should this project do?");
    session.send(ENTER); // accept the defaults

    session.wait_for("Create it?");
    let screen = session.text();

    // Every file names the answer that caused it. A list without reasons
    // would be a receipt, and the summary's whole job is showing the work.
    for expected in [
        "wally.toml",
        "this project uses Wally",
        "selene.toml",
        "you chose lint",
        "rokit.toml",
    ] {
        assert!(
            screen.contains(expected),
            "expected {expected:?} in:\n{screen}"
        );
    }
    // The capability names its tool, so a user who enabled "lint" has seen
    // the word Selene by the time the project exists.
    assert!(screen.contains("lint (Selene)"), "{screen}");
    assert!(screen.contains("format (StyLua)"), "{screen}");

    // And none of this is a checkbox: the summary is a summary. The old
    // flow rendered these as `key - description (badge)` option lines.
    for offered in ["wally.toml - ", "rokit.toml - ", "selene.toml - "] {
        assert!(
            !screen.contains(offered),
            "{offered:?} must not be a checkbox:\n{screen}"
        );
    }

    assert!(
        !path.exists(),
        "answering prompts must not create the project"
    );
    session.send(ESC);
    let _ = session.finish();
    assert!(!path.exists(), "escape must leave no project behind");
}

/// **Revision, not navigation.** Change an early answer from the summary and
/// the later ones that it makes stale are re-asked - the ones it does not
/// touch are left alone.
///
/// Needs no network: it changes the strategy to `none`, which is the branch
/// where nothing installs. What is being checked is that the graph
/// re-derives rather than that anything is written, so it cancels at the end.
#[test]
#[ignore]
fn changing_an_early_answer_reasks_only_what_it_invalidates() {
    let root = projects_root();
    let name = unique_name("rproj-revise");
    let path = root.join(&name);

    let mut session = Session::start(&root, &["new", &name]);
    session.wait_for("How should this project get its dependencies?");
    session.send(ENTER); // Wally
    session.wait_for("How do you want to pick packages?");
    session.send(&format!("{DOWN}{ENTER}")); // expert
    session.wait_for("Pick every package this project needs");
    session.send("promise");
    session.wait_for("promise - ");
    session.send(" ");
    session.send(ENTER);
    session.wait_for("What should this project do?");
    session.send(&format!("{LEFT}{ENTER}")); // nothing, so nothing installs

    // The first summary: a Wally project with a package and a manifest.
    session.wait_for("Create it?");
    let before = session.text();
    assert!(before.contains("Dependencies  Wally"), "{before}");
    assert!(before.contains("wally.toml"), "{before}");

    session.send("change");
    session.wait_for("change - ");
    session.send(ENTER);

    // The menu shows what each answer currently is, so it is answerable
    // without remembering what was said four prompts ago.
    session.wait_for("Change which answer?");
    let menu = session.text();
    assert!(menu.contains("dependencies - Wally"), "{menu}");
    assert!(menu.contains("packages - promise"), "{menu}");

    session.send("dependencies");
    session.wait_for("dependencies - ");
    session.send(ENTER);

    // Says what it is about to discard *before* discarding it.
    session.wait_for("changing this re-asks: packages");
    session.wait_for("How should this project get its dependencies?");
    session.send("none");
    session.wait_for("none - ");
    session.send(ENTER);

    // With no dependency manager the package question is skipped entirely,
    // and the manifest that only existed for Wally is gone with it.
    session.wait_for("Dependencies  none");
    let after = session.text();
    assert!(
        after.contains("Packages      none"),
        "packages were invalidated:
{after}"
    );
    assert!(
        after
            .rfind("wally.toml")
            .is_none_or(|at| at < after.rfind("Dependencies  none").unwrap()),
        "the manifest must not survive the strategy that wanted it:
{after}"
    );

    session.send("cancel");
    session.wait_for("cancel - ");
    session.send(ENTER);

    let outcome = session.finish();
    assert_eq!(
        outcome.code, 0,
        "cancelling is a clean exit:
{}",
        outcome.text
    );
    assert!(!path.exists(), "cancel must leave nothing behind");
}

/// The whole Wally chain, from picker to a green gate.
///
/// Every link here is a step that shells out or hits the network, which is
/// exactly the set no other test touches.
#[test]
#[ignore]
fn a_wally_project_scaffolds_and_passes_its_own_gate() {
    let project = LiveProject::scaffold("live-wally", &["charm", "testez"], false);

    for file in [
        "rokit.toml",
        "wally.toml",
        "selene.toml",
        "stylua.toml",
        "default.project.json",
        "sourcemap.json",
        "testez.yml",
        ".luaurc",
        ".lute/check.luau",
        ".github/workflows/ci.yml",
        ".gitattributes",
    ] {
        assert!(project.exists(file), "{file} was not scaffolded");
    }
    let dirs = directories::ProjectDirs::from("", "", "rproj").unwrap();
    let config: toml::Value =
        toml::from_str(&std::fs::read_to_string(dirs.config_dir().join("config.toml")).unwrap())
            .unwrap();
    let vscode = config["selected_system_apps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|app| app.as_str() == Some("vscode"));
    assert_eq!(project.exists(".vscode/settings.json"), vscode);

    // Wally actually resolved and vendored, rather than leaving an empty
    // folder that everything downstream would fail on. Found by scanning
    // rather than named outright: the link file takes its name from the
    // alias in `wally.toml`, which is wally's business, not this test's.
    let link = project.link_file("Packages", "charm");
    // `wally install` rewrites link files without their `export type`
    // lines; `wally::sync` puts them back. Losing this is silent - the
    // types just stop existing.
    assert!(
        link.contains("export type"),
        "package types were stripped:\n{link}"
    );

    // Case-insensitive on purpose. A Wally package is mounted under the
    // alias from `wally.toml`, so the instance is `charm`, not `Charm` -
    // the casing is wally's business here. It is *not* in the submodule
    // test below, where the mount name is load-bearing (§8.2).
    let sourcemap = project.read("sourcemap.json").to_lowercase();
    assert!(
        sourcemap.contains("\"charm\""),
        "charm missing from the sourcemap"
    );

    assert_eq!(
        project.gate(),
        0,
        "a freshly scaffolded project failed its own gate"
    );
}

/// The §8.2 claim, which until now was only ever checked by hand: a
/// submodule package resolves under *both* the name project code requires
/// and the name the vendored source requires internally.
#[test]
#[ignore]
fn submodule_packages_resolve_under_both_names_and_build() {
    // charmSync pulls in charm, and both live in one upstream monorepo -
    // the case where the mount name is load-bearing.
    let project = LiveProject::scaffold("live-submodules", &["charmSync"], true);

    assert!(project.exists("modules/Charm.luau"), "link file missing");
    assert!(
        project.exists("modules/CharmSync.luau"),
        "link file missing"
    );
    assert!(project.exists("modules/submodules/default.project.json"));

    let sourcemap = project.read("sourcemap.json");
    for name in ["Charm", "CharmSync"] {
        assert!(
            sourcemap.contains(&format!("\"{name}\"")),
            "{name} missing from the sourcemap"
        );
    }

    // The real test of the mount: rojo refuses a `$path` it can't turn into
    // an instance, so a build succeeding means every declared path resolved.
    let (code, output) = run(project.path(), "rojo", &["build", "-o", "live-check.rbxlx"]);
    assert_eq!(code, 0, "rojo build failed:\n{output}");
    assert!(project.exists("live-check.rbxlx"));

    assert_eq!(project.gate(), 0, "submodule project failed its own gate");
}

/// A gate that only ever passes proves nothing. Each of the three steps is
/// broken in turn, and each has to take the gate from 0 to non-zero.
#[test]
#[ignore]
fn the_generated_gate_rejects_bad_code_one_step_at_a_time() {
    let project = LiveProject::scaffold("live-gate", &["testez"], false);
    assert_eq!(project.gate(), 0, "should start green");

    let spec = project
        .path()
        .join("tests")
        .join("shared")
        .join("hello.spec.luau");
    let original = std::fs::read_to_string(&spec).expect("starter spec");

    // Each of these is a well-formed Luau file that exactly one gate step
    // objects to - a syntax error would fail every step at once and prove
    // only that something ran.
    let breakages: &[(&str, &str)] = &[
        (
            "luau-lsp: a string where a number is declared",
            "\nlocal wrong: number = \"not a number\"\nprint(wrong)\n",
        ),
        (
            "selene: an undefined global",
            "\nprint(someUndefinedGlobalName)\n",
        ),
        (
            "stylua: space indentation where the config says tabs",
            "\nlocal function f()\n    return 1\nend\nprint(f())\n",
        ),
    ];

    for (what, addition) in breakages {
        std::fs::write(&spec, format!("{original}{addition}")).expect("break the spec");
        assert_ne!(project.gate(), 0, "the gate accepted bad code ({what})");
        std::fs::write(&spec, &original).expect("restore the spec");
        assert_eq!(
            project.gate(),
            0,
            "restoring should go green again ({what})"
        );
    }
}

/// A `git clone` records a submodule's commit and leaves its directory
/// empty, and `rproj watch` is what repairs that. Verified by cloning
/// without `--recurse-submodules`, which is what `git clone <url>` does.
#[test]
#[ignore]
fn watch_restores_submodules_in_a_fresh_clone() {
    let project = LiveProject::scaffold("live-clone", &["charm"], true);

    let (code, output) = run(project.path(), "git", &["add", "-A"]);
    assert_eq!(code, 0, "{output}");
    let (code, output) = run(
        project.path(),
        "git",
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-qm",
            "scaffold",
        ],
    );
    assert_eq!(code, 0, "{output}");

    let clone = projects_root().join(unique_name("clone-copy"));
    let (code, output) = run(
        projects_root().as_path(),
        "git",
        &[
            "clone",
            "-q",
            &project.path().display().to_string(),
            &clone.display().to_string(),
        ],
    );
    assert_eq!(code, 0, "clone failed:\n{output}");
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(clone.clone());

    // The state a teammate actually gets: the directory exists and is empty.
    let vendored = clone.join("modules").join("submodules").join("charm");
    assert!(
        std::fs::read_dir(&vendored)
            .map(|mut d| d.next().is_none())
            .unwrap_or(true),
        "the clone already has submodule contents, so this proves nothing"
    );

    // `watch` never returns on its own; it repairs, then blocks on the
    // sourcemap watcher. Getting as far as watching is the pass condition.
    let session = Session::start(&clone, &["watch"]);
    session.wait_for("submodules synced");
    // "Watching for changes" prints *before* the watcher starts, so it
    // proves nothing on its own; rojo's own line is the real signal.
    session.wait_for("Created sourcemap");
    drop(session);

    assert!(
        std::fs::read_dir(&vendored)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false),
        "watch did not fetch the submodule"
    );
    assert!(
        clone.join("sourcemap.json").exists(),
        "no sourcemap after watch"
    );
}
