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

use std::path::{Path, PathBuf};
use std::process::Command;

use common::{Session, DOWN, ENTER};

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
        let path = root.join(name);
        let _ = std::fs::remove_dir_all(&path);

        let mut session = Session::start(&root, &["new", name]);
        session.wait_for("How do you want to set up this project's packages?");
        session.send(&format!("{DOWN}{ENTER}"));
        session.wait_for("Pick every package this project needs");
        for key in packages {
            session.send(key);
            session.wait_for(&format!("{key} - "));
            session.send(" ");
            // Clear the filter so the next key searches the whole list.
            session.send(&"\x7f".repeat(key.len()));
        }
        session.send(ENTER);

        if !packages.is_empty() {
            session.wait_for("How do you want to pull in this project's packages?");
            let keys = if submodules { format!("{DOWN}{ENTER}") } else { ENTER.to_string() };
            session.send(&keys);
        }
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
            entry.file_stem().is_some_and(|stem| stem.eq_ignore_ascii_case(key))
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

/// Where `rproj new` puts projects: the recorded root, or the default.
fn projects_root() -> PathBuf {
    let configured = std::env::var_os("APPDATA")
        .map(|appdata| PathBuf::from(appdata).join("rproj").join("config.toml"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.trim().strip_prefix("roblox_projects_root = ")?.trim().strip_prefix('"')?.strip_suffix('"').map(str::to_string))
        });
    match configured {
        Some(root) => PathBuf::from(root.replace("\\\\", "\\")),
        None => PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"))
            .join("Documents")
            .join("RobloxProjects"),
    }
}

/// Runs a toolchain command in `dir`, returning its exit code and output.
/// Tools are invoked through rokit's shims, the same way a developer's
/// shell would find them.
fn run(dir: &Path, tool: &str, args: &[&str]) -> (i32, String) {
    let shim = PathBuf::from(std::env::var_os("USERPROFILE").expect("USERPROFILE"))
        .join(".rokit")
        .join("bin")
        .join(format!("{tool}.exe"));
    let program = if shim.is_file() { shim } else { PathBuf::from(tool) };

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
        ".vscode/settings.json",
    ] {
        assert!(project.exists(file), "{file} was not scaffolded");
    }

    // Wally actually resolved and vendored, rather than leaving an empty
    // folder that everything downstream would fail on. Found by scanning
    // rather than named outright: the link file takes its name from the
    // alias in `wally.toml`, which is wally's business, not this test's.
    let link = project.link_file("Packages", "charm");
    // `wally install` rewrites link files without their `export type`
    // lines; `wally::sync` puts them back. Losing this is silent - the
    // types just stop existing.
    assert!(link.contains("export type"), "package types were stripped:\n{link}");

    // Case-insensitive on purpose. A Wally package is mounted under the
    // alias from `wally.toml`, so the instance is `charm`, not `Charm` -
    // the casing is wally's business here. It is *not* in the submodule
    // test below, where the mount name is load-bearing (§8.2).
    let sourcemap = project.read("sourcemap.json").to_lowercase();
    assert!(sourcemap.contains("\"charm\""), "charm missing from the sourcemap");

    assert_eq!(project.gate(), 0, "a freshly scaffolded project failed its own gate");
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
    assert!(project.exists("modules/CharmSync.luau"), "link file missing");
    assert!(project.exists("modules/submodules/default.project.json"));

    let sourcemap = project.read("sourcemap.json");
    for name in ["Charm", "CharmSync"] {
        assert!(sourcemap.contains(&format!("\"{name}\"")), "{name} missing from the sourcemap");
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

    let spec = project.path().join("tests").join("shared").join("hello.spec.luau");
    let original = std::fs::read_to_string(&spec).expect("starter spec");

    // Each of these is a well-formed Luau file that exactly one gate step
    // objects to - a syntax error would fail every step at once and prove
    // only that something ran.
    let breakages: &[(&str, &str)] = &[
        ("luau-lsp: a string where a number is declared", "\nlocal wrong: number = \"not a number\"\nprint(wrong)\n"),
        ("selene: an undefined global", "\nprint(someUndefinedGlobalName)\n"),
        ("stylua: space indentation where the config says tabs", "\nlocal function f()\n    return 1\nend\nprint(f())\n"),
    ];

    for (what, addition) in breakages {
        std::fs::write(&spec, format!("{original}{addition}")).expect("break the spec");
        assert_ne!(project.gate(), 0, "the gate accepted bad code ({what})");
        std::fs::write(&spec, &original).expect("restore the spec");
        assert_eq!(project.gate(), 0, "restoring should go green again ({what})");
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
        &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "scaffold"],
    );
    assert_eq!(code, 0, "{output}");

    let clone = projects_root().join("live-clone-copy");
    let _ = std::fs::remove_dir_all(&clone);
    let (code, output) = run(
        projects_root().as_path(),
        "git",
        &["clone", "-q", &project.path().display().to_string(), &clone.display().to_string()],
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
        std::fs::read_dir(&vendored).map(|mut d| d.next().is_none()).unwrap_or(true),
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
        std::fs::read_dir(&vendored).map(|mut d| d.next().is_some()).unwrap_or(false),
        "watch did not fetch the submodule"
    );
    assert!(clone.join("sourcemap.json").exists(), "no sourcemap after watch");
}
