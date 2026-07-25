use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::steps::{run, run_in};

pub fn ensure_rojo_init(project_dir: &Path) -> Result<()> {
    if project_dir.join("default.project.json").exists() {
        println!("check: default.project.json already exists");
        return Ok(());
    }
    run_in("rojo", &["init"], Some(project_dir))
}

/// Installs/updates the Rojo Studio plugin via Rojo's own CLI command -
/// no generic file-copy logic needed for this one. This targets Studio's
/// plugin folder directly, not any particular project, so it only needs
/// to run once from `rproj setup`, not per-project.
pub fn install_studio_plugin() -> Result<()> {
    run("rojo", &["plugin", "install"])
}

/// Adds a `packages` entry under `ReplicatedStorage` in default.project.json
/// so Wally-installed packages get synced into Studio.
pub fn ensure_packages_in_project_json(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("default.project.json");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut project: Value = serde_json::from_str(&text)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;

    let already_present = project
        .pointer("/tree/ReplicatedStorage/packages/$path")
        .and_then(Value::as_str)
        == Some("packages");
    if already_present {
        println!("check: packages already in ReplicatedStorage");
        return Ok(());
    }

    let tree = project
        .as_object_mut()
        .context("default.project.json root is not an object")?
        .entry("tree")
        .or_insert_with(|| Value::Object(Default::default()));
    let replicated_storage = tree
        .as_object_mut()
        .context("`tree` is not an object")?
        .entry("ReplicatedStorage")
        .or_insert_with(|| Value::Object(Default::default()));
    replicated_storage
        .as_object_mut()
        .context("`tree.ReplicatedStorage` is not an object")?
        .insert(
            "packages".to_string(),
            serde_json::json!({ "$path": "packages" }),
        );

    let pretty = serde_json::to_string_pretty(&project)?;
    fs::write(&path, pretty)?;
    println!("added packages to ReplicatedStorage");
    Ok(())
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
