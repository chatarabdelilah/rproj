use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::catalog::tool_catalog::{Detect, ToolEntry, ToolKind};
use crate::steps::execution::{MessageKind, Reporter};
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
                Detect::ExeUnder {
                    env_var,
                    subdir,
                    exe,
                } => exe_exists_under(env_var, subdir, exe) || by_winget(),
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

/// Runs `winget install` with output captured (not just inherited) so a
/// hash-mismatch failure - a known, ongoing upstream winget-pkgs issue for
/// installers like Roblox's that self-update behind a static download URL,
/// leaving the pinned manifest hash stale - can be called out with an
/// actionable message instead of a bare exit-code error.
pub(crate) fn install_winget(winget_id: &str, reporter: Option<&Reporter>) -> Result<()> {
    let args = [
        "install",
        "--id",
        winget_id,
        "-e",
        "--accept-source-agreements",
        "--accept-package-agreements",
    ];
    let output = if let Some(reporter) = reporter {
        reporter.capture(std::process::Command::new("winget").args(args))
    } else {
        capture("winget", &args, None)
    };
    let output = missing_tool_result(output, WINGET_MISSING_HELP)?;
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

pub(crate) fn ensure_rokit_with(reporter: Option<&Reporter>) -> Result<()> {
    if probe("rokit", &["--version"]) {
        if let Some(reporter) = reporter {
            reporter.message(MessageKind::Already, "rokit already installed");
        } else {
            ui::ok("rokit already installed");
        }
        return Ok(());
    }
    if let Some(reporter) = reporter {
        let mut command = std::process::Command::new("cargo");
        command.args(["install", "rokit", "--locked"]);
        cargo_bootstrap_result(reporter.run(&mut command))?;
        return reporter.run(std::process::Command::new("rokit").arg("self-install"));
    }
    cargo_bootstrap_result(run("cargo", &["install", "rokit", "--locked"]))?;
    run("rokit", &["self-install"])
}

const WINGET_MISSING_HELP: &str = "WinGet was not found on PATH; it is required to install applications. \
    Install or repair App Installer using https://learn.microsoft.com/en-us/windows/package-manager/winget/, \
    ensure WinGet is on PATH, open a new terminal, check `winget --version`, and rerun `rproj setup`";

fn missing_tool_result<T>(result: Result<T>, help: &'static str) -> Result<T> {
    result.map_err(|error| {
        if error
            .downcast_ref::<std::io::Error>()
            .is_some_and(|cause| cause.kind() == std::io::ErrorKind::NotFound)
        {
            error.context(help)
        } else {
            error
        }
    })
}

fn cargo_bootstrap_result(result: Result<()>) -> Result<()> {
    missing_tool_result(
        result,
        "Cargo was not found on PATH; it is required to install Rokit. \
         Install Rust from https://rustup.rs, open a new terminal, \
         check `cargo --version`, and rerun `rproj setup`",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::tool_catalog::SYSTEM_APPS;

    #[test]
    fn missing_winget_recovery_covers_plain_and_reported_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("missing-winget");
        for reported in [false, true] {
            let result = if reported {
                let reporter = Reporter(std::sync::Arc::new(|_, _| {}));
                reporter.capture(&mut std::process::Command::new(&absent))
            } else {
                capture(absent.to_str().unwrap(), &[], None)
            };
            let error = missing_tool_result(result, WINGET_MISSING_HELP).unwrap_err();
            let message = format!("{error:#}");
            for expected in [
                "App Installer",
                "https://learn.microsoft.com/en-us/windows/package-manager/winget/",
                "new terminal",
                "winget --version",
                "rproj setup",
            ] {
                assert!(message.contains(expected), "{message}");
            }
            assert!(message.contains("missing-winget"), "{message}");
            assert_eq!(
                error.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::NotFound
            );
        }
    }

    #[test]
    fn winget_help_preserves_permission_errors_and_installer_output() {
        let denied = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            .context("failed to spawn `winget`");
        let before = format!("{denied:#}");
        let error = missing_tool_result::<crate::steps::Captured>(Err(denied), WINGET_MISSING_HELP)
            .unwrap_err();
        assert_eq!(format!("{error:#}"), before);
        for success in [false, true] {
            let output = crate::steps::Captured {
                stdout: "installer output".into(),
                stderr: "Installer hash does not match".into(),
                success,
            };
            let preserved = missing_tool_result(Ok(output), WINGET_MISSING_HELP).unwrap();
            assert_eq!(preserved.stdout, "installer output");
            assert_eq!(preserved.stderr, "Installer hash does not match");
            assert_eq!(preserved.success, success);
        }
    }

    #[test]
    fn missing_cargo_bootstrap_explains_rust_and_path_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("missing-cargo");
        let result = run(absent.to_str().unwrap(), &[]);
        let error = cargo_bootstrap_result(result).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("https://rustup.rs"), "{message}");
        assert!(message.contains("new terminal"), "{message}");
        assert!(message.contains("rproj setup"), "{message}");
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[test]
    fn cargo_bootstrap_preserves_permission_and_build_failures() {
        let denied = anyhow::Error::new(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            .context("failed to spawn `cargo`");
        let build = anyhow::anyhow!("cargo install rokit failed with exit code 1");
        for original in [denied, build] {
            let before = format!("{original:#}");
            let after = cargo_bootstrap_result(Err(original)).unwrap_err();
            assert_eq!(format!("{after:#}"), before);
        }
        assert!(cargo_bootstrap_result(Ok(())).is_ok());
    }

    /// Studio must not be detected through winget.
    ///
    /// `winget list --id Roblox.RobloxStudio -e` exits 20 on a machine with
    /// Studio installed, because Studio installs per-user through its own
    /// bootstrapper. Reverting this entry to `Detect::Winget` makes setup
    /// try to reinstall Studio on every run and skip `rojo plugin install`
    /// as "Studio isn't installed".
    #[test]
    fn studio_is_not_detected_through_winget() {
        let studio = SYSTEM_APPS
            .iter()
            .find(|e| e.key == "studio")
            .expect("a studio entry");
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
            let ToolKind::SystemApp { detect, .. } = entry.kind else {
                continue;
            };
            assert_eq!(
                detect,
                Detect::Winget,
                "{} should detect via winget",
                entry.key
            );
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
        assert!(!exe_exists_under(
            "RPROJ_TEST_DETECT_UNSET_VAR",
            "App",
            "Thing.exe"
        ));
    }
}
