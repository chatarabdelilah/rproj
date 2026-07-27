use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::catalog::wally_packages::{self, Realm};
use crate::steps::{rojo, run_in};
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

    let mut specs = Vec::new();
    for key in selected {
        specs.push(
            wally_packages::find(key)
                .with_context(|| format!("unknown package key `{key}` in selection"))?,
        );
    }

    let mut body = format!(
        "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nregistry = \"https://github.com/UpliftGames/wally-index\"\nrealm = \"shared\"\n\n[dependencies]\n"
    );
    for spec in specs.iter().filter(|s| s.realm == Realm::Shared) {
        body.push_str(&format!("{} = \"{}\"\n", spec.key, spec.source));
    }

    // A server-realm package under `[dependencies]` doesn't merely land in
    // the wrong folder - wally refuses to resolve it and the install fails
    // outright, taking the whole scaffold with it. The section is only
    // emitted when something needs it, so the common project keeps a
    // one-section manifest.
    let server: Vec<_> = specs.iter().filter(|s| s.realm == Realm::Server).collect();
    if !server.is_empty() {
        body.push_str("\n[server-dependencies]\n");
        for spec in server {
            body.push_str(&format!("{} = \"{}\"\n", spec.key, spec.source));
        }
    }

    fs::write(&path, body)?;
    ui::ok("wrote wally.toml");
    Ok(())
}

pub fn wally_install(project_dir: &Path) -> Result<()> {
    run_in("wally", &["install"], Some(project_dir))
}

/// `Packages/` with a capital P - the name wally hardcodes for the shared
/// realm. Every other spelling only works by accident on Windows.
pub const PACKAGES_DIR: &str = "Packages";

/// Where wally puts `[server-dependencies]`. A separate folder, not a
/// subfolder of `Packages/`, so everything that touches vendored package
/// code has to know about both.
pub const SERVER_PACKAGES_DIR: &str = "ServerPackages";

/// Retypes the link files in whichever package folders exist.
///
/// Both folders are passed in one invocation rather than two runs: the tool
/// accepts multiple paths, and a project with no server-realm package has no
/// `ServerPackages/` at all - naming a missing directory is an error, so the
/// argument list is built from what's on disk.
pub fn wally_package_types(project_dir: &Path) -> Result<()> {
    let mut args = vec!["-s", "sourcemap.json"];
    for dir in [PACKAGES_DIR, SERVER_PACKAGES_DIR] {
        if project_dir.join(dir).is_dir() {
            args.push(dir);
        }
    }
    run_in("wally-package-types", &args, Some(project_dir))
}

/// Installs this project's Wally dependencies and restores the types on the
/// generated link files. Always use this rather than `wally_install` alone.
///
/// `wally install` rewrites every link file in `packages/` from scratch, and
/// what it writes is a bare `return require(script.Parent._Index[...])` with
/// no `export type` lines at all. So an install doesn't just *fail to add*
/// types - it actively strips the ones `wally-package-types` put there on
/// the previous run, and every package silently degrades to `any`.
///
/// That made `rproj watch` destructive: it installed and went straight to
/// watching, so the documented first thing to run after `rproj new` undid
/// the scaffold's own retyping, and the only way back was typing
/// `wally-package-types -s sourcemap.json packages/` by hand.
///
/// The order is fixed by how wally-package-types works: it resolves each
/// link file's require through the sourcemap to find the real module, so
/// the packages have to be on disk before the sourcemap is generated, and
/// the sourcemap has to exist before the retyping runs.
pub fn sync(project_dir: &Path) -> Result<()> {
    wally_install(project_dir)?;
    // With zero dependencies `wally install` doesn't just skip creating
    // Packages/ - it *removes* an existing empty one (verified). The
    // project file maps that path, and rojo refuses to generate a sourcemap
    // at all when a mapped $path is missing, so recreating it here is what
    // keeps a no-dependency project working.
    fs::create_dir_all(project_dir.join(PACKAGES_DIR))?;
    rojo::generate_sourcemap(project_dir)?;
    // Safe with no packages: an empty directory is a successful no-op.
    wally_package_types(project_dir)
}
