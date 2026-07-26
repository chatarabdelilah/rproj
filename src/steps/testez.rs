//! TestEZ wiring for a scaffolded project.
//!
//! Selecting `testez` as a package only puts the *library* in the tree.
//! Two more things are needed before tests actually run:
//!
//! - `selene.toml` must use `std = "roblox+testez"`, or every `describe`/
//!   `it`/`expect` is reported as an undefined global. (Handled by
//!   `steps::toolchain::ensure_selene_config`.)
//! - The TestEZ Companion Studio plugin needs a `testez-companion.toml`
//!   telling it which DataModel locations hold `.spec` files.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::ui;

/// DataModel locations holding test files, as TestEZ Companion wants them:
/// service-rooted, slash-separated, and searched recursively (the plugin
/// finds `.spec` files as descendants).
///
/// These mirror the three source folders `steps::rojo` maps, so tests can
/// live next to the code they cover. Derived from the same constant the
/// project file is built from rather than written out twice, since a root
/// that doesn't match the tree silently finds no tests - the plugin just
/// reports nothing to run, which reads like "all tests pass".
pub const TEST_ROOTS: &[&str] = &[
    "ReplicatedStorage/shared",
    "ServerScriptService/server",
    "StarterPlayer/StarterPlayerScripts/client",
];

pub fn companion_config() -> String {
    let roots = TEST_ROOTS
        .iter()
        .map(|r| format!("\t\"{r}\",\n"))
        .collect::<String>();
    format!(
        "# Where TestEZ Companion looks for .spec files. Descendants are\n\
         # included, so these are the three source roots rproj maps in\n\
         # default.project.json - put a foo.spec.luau next to foo.luau.\n\
         roots = [\n{roots}]\n"
    )
}

pub fn ensure_companion_config(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("testez-companion.toml");
    if path.exists() {
        ui::ok("testez-companion.toml already exists");
        return Ok(());
    }
    fs::write(&path, companion_config())
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote testez-companion.toml");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A root that doesn't match the generated tree finds nothing, and
    /// "no tests found" is indistinguishable from "all tests passed".
    #[test]
    fn roots_are_service_rooted_and_match_the_scaffolded_tree() {
        let config = companion_config();
        for root in TEST_ROOTS {
            assert!(config.contains(root), "{root} missing from:\n{config}");
            assert!(!root.starts_with('/'), "{root} should not start with a slash");
            assert!(!root.starts_with("game/"), "{root} should be service-rooted, not game-rooted");
        }
        assert!(config.starts_with('#'), "config should explain itself");
        assert!(config.contains("roots = ["));
    }

    /// The roots have to name the same instances default.project.json
    /// creates; these are the lowercase folder names it maps.
    #[test]
    fn roots_use_the_lowercase_instance_names_the_project_file_creates() {
        assert!(TEST_ROOTS.contains(&"ReplicatedStorage/shared"));
        assert!(TEST_ROOTS.contains(&"ServerScriptService/server"));
        assert!(TEST_ROOTS.contains(&"StarterPlayer/StarterPlayerScripts/client"));
    }
}
