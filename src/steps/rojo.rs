use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::catalog::place_template;
use crate::config::PackageWorkflow;
use crate::steps::run;
use crate::ui;

/// Writes `default.project.json` from scratch (rather than `rojo init` +
/// patching) with the conventional server/client/shared split, and creates
/// the matching `src/` folders. The packages folder mapped into
/// `ReplicatedStorage` depends on which package workflow this project uses:
/// Wally's `packages/` or git submodules' `modules/`.
pub fn scaffold_project_json(
    project_dir: &Path,
    project_name: &str,
    package_workflow: PackageWorkflow,
) -> Result<()> {
    let path = project_dir.join("default.project.json");
    if path.exists() {
        ui::ok("default.project.json already exists");
        return Ok(());
    }

    // Create the source folders empty, with only a .gitkeep.
    //
    // Deliberately no `init.luau`/`init.server.luau`/`init.client.luau`
    // starter files: Rojo turns a directory containing one into a
    // *script* named after the directory, so `src/shared/init.luau` makes
    // ReplicatedStorage.shared a ModuleScript-with-children and
    // `src/server/init.server.luau` makes it a Script, instead of the
    // plain Folders these are meant to be. (Verified: with the init files
    // present the sourcemap reports className ModuleScript/Script; without
    // them, Folder.) The .gitkeep is what keeps the directory in git -
    // git doesn't track empty directories, so without it a fresh clone
    // would be missing the paths default.project.json maps and Rojo would
    // fail on them.
    for rel_path in ["src/shared", "src/server", "src/client"] {
        let dir = project_dir.join(rel_path);
        fs::create_dir_all(&dir)?;
        let keep = dir.join(".gitkeep");
        if !keep.exists() {
            fs::write(&keep, "")?;
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
            replicated_storage.insert("packages".to_string(), json!({ "$path": "packages" }));
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

    let mut tree = serde_json::Map::new();
    tree.insert("$className".to_string(), json!("DataModel"));
    tree.insert("ReplicatedStorage".to_string(), json!(replicated_storage));
    tree.insert("ServerScriptService".to_string(), json!({ "server": { "$path": "src/server" } }));
    tree.insert(
        "StarterPlayer".to_string(),
        json!({ "StarterPlayerScripts": { "client": { "$path": "src/client" } } }),
    );

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

/// Starts `rojo sourcemap --watch`, blocking until sourcemap.json first
/// appears (or the timeout elapses), then leaves the watcher running in
/// the background and returns its child handle so the caller can decide
/// whether to wait on it or move on.
pub fn start_sourcemap_watcher(project_dir: &Path) -> Result<std::process::Child> {
    let args = ["sourcemap", "--watch", "default.project.json", "-o", "sourcemap.json"];
    ui::command("rojo", &args);
    let child = Command::new("rojo")
        .args(args)
        .current_dir(project_dir)
        .stdin(Stdio::null())
        // Rojo prints "Created sourcemap at ..." on every write; the
        // caller already reports the outcome, and under --watch this would
        // keep printing after the scaffold has moved on.
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to start `rojo sourcemap --watch`")?;

    let sourcemap_path = project_dir.join("sourcemap.json");
    let start = Instant::now();
    let timeout = Duration::from_secs(10);
    while !sourcemap_path.exists() {
        if start.elapsed() > timeout {
            bail!("timed out waiting for sourcemap.json");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(child)
}

