use std::fs;
use std::path::Path;

use anyhow::{bail, Result};

use crate::catalog::tool_catalog::{Detect, ToolEntry, ToolKind};
use crate::steps::{capture, probe, run};
use crate::ui;

pub fn is_installed(entry: &ToolEntry) -> bool {
    match entry.kind {
        ToolKind::SystemApp { winget_id, detect } => {
            let by_winget = || probe("winget", &["list", "--id", winget_id, "-e"]);
            match detect {
                Detect::Winget => by_winget(),
                // Either kind of evidence counts: the app may also have
                // been installed some way winget *does* track.
                Detect::ExeUnder { env_var, subdir, exe } => {
                    exe_exists_under(env_var, subdir, exe) || by_winget()
                }
            }
        }
        _ => false,
    }
}

/// Whether `exe` exists at `%env_var%\subdir\` or one level below it.
///
/// One level, not a recursive walk, because the layout this exists for is
/// exactly `Versions\version-<hash>\<exe>` - and a recursive search of a
/// directory that large is slow enough to be noticeable in a detection
/// pass that runs for every app.
fn exe_exists_under(env_var: &str, subdir: &str, exe: &str) -> bool {
    let Some(base) = std::env::var_os(env_var) else {
        return false;
    };
    exe_exists_in(Path::new(&base), subdir, exe)
}

/// The pure half, taking the base directory rather than reading it.
///
/// Split out so the tests point it at a scratch directory instead of
/// setting an environment variable - which is process-global, shared with
/// every other test in the binary, and `unsafe` in edition 2024.
fn exe_exists_in(base: &Path, subdir: &str, exe: &str) -> bool {
    let mut root = base.to_path_buf();
    for part in subdir.split('/') {
        root.push(part);
    }
    if root.join(exe).is_file() {
        return true;
    }
    let Ok(entries) = fs::read_dir(&root) else {
        return false;
    };
    entries
        .filter_map(|e| e.ok())
        .any(|e| e.path().join(exe).is_file())
}

pub fn install(entry: &ToolEntry) -> Result<()> {
    match entry.kind {
        ToolKind::SystemApp { winget_id, .. } => install_winget(winget_id),
        _ => Ok(()),
    }
}

/// Runs `winget install` with output captured (not just inherited) so a
/// hash-mismatch failure - a known, ongoing upstream winget-pkgs issue for
/// installers like Roblox's that self-update behind a static download URL,
/// leaving the pinned manifest hash stale - can be called out with an
/// actionable message instead of a bare exit-code error.
fn install_winget(winget_id: &str) -> Result<()> {
    let args = [
        "install",
        "--id",
        winget_id,
        "-e",
        "--accept-source-agreements",
        "--accept-package-agreements",
    ];
    let output = capture("winget", &args, None)?;
    if !output.success || ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if output.success {
        return Ok(());
    }

    // A hash mismatch is a known, ongoing upstream winget-pkgs issue, not
    // anything the user did. Keep the headline short and put the recovery
    // options in `detail` so they're available without dominating the run.
    if output.combined().contains("Installer hash does not match") {
        bail!("winget's pinned installer hash is stale (an upstream winget-pkgs issue, not rproj)");
    }
    bail!("winget install failed");
}

/// Recovery options for the hash-mismatch case, printed by the caller
/// alongside its warning so the failure is actionable without every other
/// install path carrying the same wall of text.
pub const WINGET_HASH_HELP: &str = "A vendor updated their installer behind a static URL faster than winget's manifest.\n\
     - Try again in a few days; the winget-pkgs bots usually catch up\n\
     - Or install it directly (for Studio: https://www.roblox.com/create)\n\
     - Or, as admin, `winget settings --enable InstallerHashOverride` once, then retry";

/// Ensures Rokit itself is present. Rokit isn't on winget - its own docs
/// specify `cargo install rokit --locked` then `rokit self-install`.
pub fn ensure_rokit() -> Result<()> {
    if probe("rokit", &["--version"]) {
        ui::ok("rokit already installed");
        return Ok(());
    }
    run("cargo", &["install", "rokit", "--locked"])?;
    run("rokit", &["self-install"])
}

pub fn ensure_projects_folder(root: &Path) -> Result<()> {
    if root.exists() {
        ui::ok(&format!("{} already exists", root.display()));
        return Ok(());
    }
    fs::create_dir_all(root)?;
    ui::ok(&format!("created {}", root.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::tool_catalog::SYSTEM_APPS;

    /// Studio must not be detected through winget.
    ///
    /// `winget list --id Roblox.RobloxStudio -e` exits 20 on a machine with
    /// Studio installed, because Studio installs per-user through its own
    /// bootstrapper. Reverting this entry to `Detect::Winget` makes setup
    /// try to reinstall Studio on every run and skip `rojo plugin install`
    /// as "Studio isn't installed".
    #[test]
    fn studio_is_not_detected_through_winget() {
        let studio = SYSTEM_APPS.iter().find(|e| e.key == "studio").expect("a studio entry");
        let ToolKind::SystemApp { detect, .. } = studio.kind else {
            panic!("studio must be a system app");
        };
        assert!(
            matches!(detect, Detect::ExeUnder { .. }),
            "studio must not use winget detection: {detect:?}"
        );
    }

    /// Every other system app does use winget, so the exception stays an
    /// exception rather than spreading by copy-paste.
    #[test]
    fn every_other_system_app_uses_winget_detection() {
        for entry in SYSTEM_APPS.iter().filter(|e| e.key != "studio") {
            let ToolKind::SystemApp { detect, .. } = entry.kind else { continue };
            assert_eq!(detect, Detect::Winget, "{} should detect via winget", entry.key);
        }
    }

    /// A scratch directory, so no test here has to set an environment
    /// variable to steer the lookup.
    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("rproj-detect-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    /// The layout the exception exists for: `<root>/version-<hash>/<exe>`.
    #[test]
    fn an_exe_one_level_below_the_root_is_found() {
        let base = scratch("versioned");
        let versioned = base.join("App").join("Versions").join("version-abc123");
        fs::create_dir_all(&versioned).expect("create");
        fs::write(versioned.join("Thing.exe"), b"").expect("write");

        assert!(exe_exists_in(&base, "App/Versions", "Thing.exe"));
        assert!(!exe_exists_in(&base, "App/Versions", "Missing.exe"));
        assert!(!exe_exists_in(&base, "App/Nope", "Thing.exe"));

        let _ = fs::remove_dir_all(&base);
    }

    /// And directly in the root, for an app that is not versioned.
    #[test]
    fn an_exe_directly_in_the_root_is_found() {
        let base = scratch("flat");
        fs::create_dir_all(base.join("App")).expect("create");
        fs::write(base.join("App").join("Thing.exe"), b"").expect("write");

        assert!(exe_exists_in(&base, "App", "Thing.exe"));
        let _ = fs::remove_dir_all(&base);
    }

    /// A directory where the executable should be is not a match - the
    /// `is_file` check, which `exists()` would have got wrong.
    #[test]
    fn a_directory_named_like_the_exe_is_not_a_match() {
        let base = scratch("dir-named-exe");
        fs::create_dir_all(base.join("App").join("Thing.exe")).expect("create");
        assert!(!exe_exists_in(&base, "App", "Thing.exe"));
        let _ = fs::remove_dir_all(&base);
    }

    /// An unset variable is "not installed", not a panic.
    #[test]
    fn an_unset_environment_variable_is_not_installed() {
        assert!(!exe_exists_under("RPROJ_TEST_DETECT_UNSET_VAR", "App", "Thing.exe"));
    }
}
