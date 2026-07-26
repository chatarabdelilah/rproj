use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde_json::json;

use crate::config::PackageWorkflow;
use crate::steps::run;

/// Writes `default.project.json` from scratch (rather than `rojo init` +
/// patching) with the conventional Server/Client/Shared split, and creates
/// the matching `src/` folders with a starter file each so Rojo has
/// something to sync immediately. The packages folder mapped into
/// `ReplicatedStorage` depends on which package workflow this project uses:
/// Wally's `packages/` or git submodules' `Modules/`.
pub fn scaffold_project_json(
    project_dir: &Path,
    project_name: &str,
    package_workflow: PackageWorkflow,
) -> Result<()> {
    let path = project_dir.join("default.project.json");
    if path.exists() {
        println!("check: default.project.json already exists");
        return Ok(());
    }

    let starter_files = [
        ("src/shared/init.luau", "-- Shared code, available to both client and server.\nreturn {}\n"),
        ("src/server/init.server.luau", "-- Server entry point.\n"),
        ("src/client/init.client.luau", "-- Client entry point.\n"),
    ];
    for (rel_path, contents) in starter_files {
        let file_path = project_dir.join(rel_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !file_path.exists() {
            fs::write(&file_path, contents)?;
        }
    }

    let mut replicated_storage = serde_json::Map::new();
    replicated_storage.insert("Shared".to_string(), json!({ "$path": "src/shared" }));
    match package_workflow {
        PackageWorkflow::Wally => {
            replicated_storage.insert("packages".to_string(), json!({ "$path": "packages" }));
        }
        PackageWorkflow::GitSubmodules => {
            replicated_storage.insert("Modules".to_string(), json!({ "$path": "Modules" }));
        }
    }

    let project = json!({
        "name": project_name,
        "tree": {
            "$className": "DataModel",
            "ReplicatedStorage": replicated_storage,
            "ServerScriptService": {
                "Server": { "$path": "src/server" }
            },
            "StarterPlayer": {
                "StarterPlayerScripts": {
                    "Client": { "$path": "src/client" }
                }
            }
        }
    });

    fs::write(&path, serde_json::to_string_pretty(&project)?)?;
    println!("wrote default.project.json");
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
    println!("\n> rojo sourcemap --watch default.project.json -o sourcemap.json");
    let child = Command::new("rojo")
        .args(["sourcemap", "--watch", "default.project.json", "-o", "sourcemap.json"])
        .current_dir(project_dir)
        .stdin(Stdio::null())
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
