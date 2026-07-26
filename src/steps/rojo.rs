use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::catalog::place_template;
use crate::config::PackageWorkflow;
use crate::steps::{capture, run};
use crate::ui;

/// Writes `default.project.json` from scratch (rather than `rojo init` +
/// patching) with the conventional server/client/shared split, and creates
/// the matching `src/` folders. The packages folder mapped into
/// `ReplicatedStorage` depends on which package workflow this project uses:
/// Wally's `packages/` or git submodules' `modules/`.
/// Starter files, one per source folder.
///
/// These are named `hello.*`, never `init.*`: Rojo turns a directory
/// containing an `init.luau` into a *script* named after the directory, so
/// `src/shared/init.luau` would make `ReplicatedStorage.shared` a
/// ModuleScript-with-children and `src/server/init.server.luau` would make
/// it a Script, instead of the plain Folders these are meant to be.
/// (Verified from the sourcemap's `className` both ways.)
///
/// They also do the job a `.gitkeep` was previously doing - git doesn't
/// track empty directories, so without *some* file a fresh clone would be
/// missing the very paths `default.project.json` maps - while actually
/// being useful, unlike an empty placeholder.
const STARTER_FILES: &[(&str, &str)] = &[
    ("src/shared/hello.luau", "--!strict\n\nreturn {\n\tgreeting = \"hello from shared\",\n}\n"),
    (
        "src/server/hello.server.luau",
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\n\nlocal hello = require(ReplicatedStorage.shared.hello)\n\nprint(hello.greeting, \"- server\")\n",
    ),
    (
        "src/client/hello.client.luau",
        "--!strict\n\nlocal ReplicatedStorage = game:GetService(\"ReplicatedStorage\")\n\nlocal hello = require(ReplicatedStorage.shared.hello)\n\nprint(hello.greeting, \"- client\")\n",
    ),
];

pub fn scaffold_project_json(
    project_dir: &Path,
    project_name: &str,
    package_workflow: PackageWorkflow,
    testez_selected: bool,
) -> Result<()> {
    let path = project_dir.join("default.project.json");
    if path.exists() {
        ui::ok("default.project.json already exists");
        return Ok(());
    }

    for (rel_path, contents) in STARTER_FILES {
        let file = project_dir.join(rel_path);
        fs::create_dir_all(file.parent().expect("starter paths have a parent"))?;
        if !file.exists() {
            fs::write(&file, contents)?;
        }
    }

    // Instance names mirroring source folders stay lowercase, matching the
    // folder they map. Roblox's own service names (ReplicatedStorage,
    // ServerScriptService...) keep their real casing - those aren't ours
    // to rename.
    let mut replicated_storage = serde_json::Map::new();
    replicated_storage.insert("shared".to_string(), json!({ "$path": "src/shared" }));

    let mut project = serde_json::Map::new();
    project.insert("name".to_string(), json!(project_name));

    match package_workflow {
        PackageWorkflow::Wally => {
            // Instance name lowercase (it's ours), $path capitalised
            // (it isn't): `Packages` is hardcoded in wally and is the
            // folder it actually creates. Writing `packages` here works
            // on Windows purely because the filesystem is
            // case-insensitive - on the ubuntu-latest runner the CI
            // workflow uses, rojo can't find the path at all and every
            // step of the gate fails before it starts.
            replicated_storage.insert("packages".to_string(), json!({ "$path": "Packages" }));
        }
        PackageWorkflow::GitSubmodules => {
            // Mapped wholesale, which is safe *because* of the nested
            // project file `steps::modules` writes at
            // modules/submodules/default.project.json. Rojo auto-detects
            // that file and uses it for the submodules folder, and it only
            // ever $paths into specific source subfolders
            // (./charm/packages/charm/src), so Rojo never walks a vendored
            // repo's root and never sees the vendored default.project.json
            // that would otherwise be loaded as a nested project and fail
            // on paths only an npm/pnpm install would create. See §7 of
            // docs/architecture.md.
            replicated_storage.insert("modules".to_string(), json!({ "$path": "modules" }));
        }
    }

    let mut server_scripts = serde_json::Map::new();
    server_scripts.insert("server".to_string(), json!({ "$path": "src/server" }));
    let mut client_scripts = serde_json::Map::new();
    client_scripts.insert("client".to_string(), json!({ "$path": "src/client" }));

    // TestEZ tests live in their own top-level `tests/` tree rather than
    // beside the code, and each half is mounted next to the source it
    // covers as `test`. The names here are what `steps::testez` writes
    // into testez-companion.toml's roots - a root naming an instance this
    // tree doesn't create finds no tests, which reads exactly like every
    // test passing.
    if testez_selected {
        replicated_storage.insert("test".to_string(), json!({ "$path": "tests/shared" }));
        server_scripts.insert("test".to_string(), json!({ "$path": "tests/server" }));
        client_scripts.insert("test".to_string(), json!({ "$path": "tests/client" }));
    }

    let mut tree = serde_json::Map::new();
    tree.insert("$className".to_string(), json!("DataModel"));
    tree.insert("ReplicatedStorage".to_string(), json!(replicated_storage));
    tree.insert("ServerScriptService".to_string(), json!(server_scripts));
    tree.insert("StarterPlayer".to_string(), json!({ "StarterPlayerScripts": client_scripts }));

    // Place-level defaults (Lighting and friends) come from the
    // place_template catalog, so changing the look every new project starts
    // with is a data edit, not a code change.
    for (name, node) in place_template::render() {
        tree.insert(name, node);
    }

    project.insert("tree".to_string(), serde_json::Value::Object(tree));

    fs::write(&path, serde_json::to_string_pretty(&project)?)?;
    ui::ok("wrote default.project.json");
    Ok(())
}

/// Installs/updates the Rojo Studio plugin via Rojo's own CLI command -
/// no generic file-copy logic needed for this one. This targets Studio's
/// plugin folder directly, not any particular project, so it only needs
/// to run once from `rproj setup`, not per-project.
pub fn install_studio_plugin() -> Result<()> {
    run("rojo", &["plugin", "install"])
}

/// Generates `sourcemap.json` once.
///
/// Deliberately *not* `--watch`: the scaffold only needs one sourcemap,
/// and the watch-then-kill approach it replaced was the source of two
/// bugs. It polled for the file with a 10s timeout, so any rojo failure
/// surfaced as "timed out waiting for sourcemap.json" with rojo's actual
/// explanation captured and thrown away - and on that timeout path it
/// bailed without killing the child, leaving a `rojo sourcemap --watch`
/// running forever. Running it once and reading the exit status reports
/// whatever rojo actually said, and leaves nothing behind.
pub fn generate_sourcemap(project_dir: &Path) -> Result<()> {
    let output = capture(
        "rojo",
        &["sourcemap", "default.project.json", "-o", "sourcemap.json"],
        Some(project_dir),
    )?;
    if !output.success || ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if !output.success {
        bail!("rojo could not generate a sourcemap (see above)");
    }
    ui::ok("generated sourcemap.json");
    Ok(())
}

/// Runs `rojo sourcemap --watch` in the foreground until interrupted, for
/// `rproj watch`. Unlike the one-shot `generate_sourcemap`, this genuinely
/// wants a long-lived process, so stdio is inherited: the user is watching
/// this run and rojo's own "Created sourcemap at ..." on each rebuild is
/// the feedback that it's working.
pub fn watch_sourcemap(project_dir: &Path) -> Result<()> {
    let args = ["sourcemap", "--watch", "default.project.json", "-o", "sourcemap.json"];
    ui::command("rojo", &args);
    let status = Command::new("rojo")
        .args(args)
        .current_dir(project_dir)
        .status()
        .context("failed to start `rojo sourcemap --watch`")?;
    // Ctrl+C reaches the child too and is the normal way to stop watching,
    // so a non-zero exit here is expected rather than a failure worth
    // reporting as one.
    let _ = status;
    Ok(())
}
