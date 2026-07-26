use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::catalog::wally_packages;
use crate::steps::run_in;
use crate::ui;

pub fn ensure_wally_init(project_dir: &Path) -> Result<()> {
    if project_dir.join("wally.toml").exists() {
        ui::ok("wally.toml already exists");
        return Ok(());
    }
    run_in("wally", &["init"], Some(project_dir))
}

/// Writes wally.toml from the catalog for the given selected package keys.
/// Idempotent: if the file already lists exactly this set of dependency
/// keys, it's left untouched.
pub fn write_wally_toml(project_dir: &Path, package_name: &str, selected: &[String]) -> Result<()> {
    let path = project_dir.join("wally.toml");

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        let has_all = selected
            .iter()
            .all(|key| content.lines().any(|l| l.trim_start().starts_with(&format!("{key} ="))));
        if has_all {
            ui::ok("wally.toml already configured");
            return Ok(());
        }
    }

    let mut body = format!(
        "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nregistry = \"https://github.com/UpliftGames/wally-index\"\nrealm = \"shared\"\n\n[dependencies]\n"
    );
    for key in selected {
        let spec = wally_packages::find(key)
            .with_context(|| format!("unknown package key `{key}` in selection"))?;
        body.push_str(&format!("{} = \"{}\"\n", spec.key, spec.source));
    }

    fs::write(&path, body)?;
    ui::ok("wrote wally.toml");
    Ok(())
}

pub fn wally_install(project_dir: &Path) -> Result<()> {
    run_in("wally", &["install"], Some(project_dir))
}

pub fn wally_package_types(project_dir: &Path) -> Result<()> {
    run_in(
        "wally-package-types",
        &["-s", "sourcemap.json", "packages/"],
        Some(project_dir),
    )
}
