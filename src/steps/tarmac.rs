//! Writes `tarmac.toml`.
//!
//! Tarmac's config is a project name plus one or more `[[inputs]]` blocks
//! saying where assets live and where to generate the Luau module holding
//! their uploaded ids. Keys verified against Tarmac's own README rather
//! than recalled: `glob`, `codegen`, `codegen-path`, `codegen-base-path`.
//!
//! The asset folder is chosen from what the project actually has. A project
//! with `figma/exports/` gets that as its input glob, because that is where
//! the Figma artifact tells people to put exports - so selecting both makes
//! one pipeline rather than two folders that ignore each other.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::ui;

/// Where a project's source images live, and the base path for generated
/// code.
///
/// Returned rather than assumed so the caller can report which was chosen -
/// a user who selected Figma should be told their exports folder is wired
/// up, not left to discover it.
pub fn asset_source(project_dir: &Path) -> (&'static str, &'static str) {
    if project_dir.join("figma").join("exports").is_dir() {
        ("figma/exports/**/*.png", "figma/exports")
    } else {
        ("assets/**/*.png", "assets")
    }
}

pub fn render(project_name: &str, glob: &str, base_path: &str) -> String {
    format!(
        "\
# Tarmac uploads the images under `glob` to Roblox and generates a Luau
# module holding the resulting asset ids, so you write `Assets.logo`
# instead of pasting rbxassetid numbers.
#
#   tarmac sync --target roblox    upload changed assets, update ids
#   tarmac sync --target none      dry run, see what would upload
name = \"{project_name}\"

[[inputs]]
glob = \"{glob}\"
codegen = true
# Generated, and committed: the ids have to be in the tree for a teammate
# who has not run `tarmac sync` to build the project.
codegen-path = \"src/shared/assets.luau\"
codegen-base-path = \"{base_path}\"
"
    )
}

/// Creates `tarmac.toml` if it is not already there, and the asset folder
/// it points at.
///
/// Never overwrites: the config depends on a project's asset layout, and
/// someone who has edited theirs has said more about their layout than this
/// function knows.
pub fn ensure_config(project_dir: &Path, project_name: &str) -> Result<()> {
    let path = project_dir.join("tarmac.toml");
    if path.exists() {
        ui::ok("tarmac.toml already exists");
        return Ok(());
    }

    let (glob, base_path) = asset_source(project_dir);
    fs::write(&path, render(project_name, glob, base_path))
        .with_context(|| format!("failed to write {}", path.display()))?;

    // The glob's folder, so there is somewhere to put images. Tarmac errors
    // on a glob matching nothing rather than treating it as empty.
    let folder = project_dir.join(base_path);
    fs::create_dir_all(&folder)
        .with_context(|| format!("failed to create {}", folder.display()))?;
    let keep = folder.join(".gitkeep");
    if !keep.exists() {
        let _ = fs::write(&keep, b"");
    }

    ui::ok(&format!("wrote tarmac.toml, reading assets from {base_path}/"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rendered_config_parses_and_carries_the_verified_keys() {
        let text = render("my-game", "assets/**/*.png", "assets");
        let parsed: toml::Table = toml::from_str(&text).expect("valid TOML");

        assert_eq!(parsed["name"].as_str(), Some("my-game"));
        let input = &parsed["inputs"].as_array().expect("inputs array")[0];
        assert_eq!(input["glob"].as_str(), Some("assets/**/*.png"));
        assert_eq!(input["codegen"].as_bool(), Some(true));
        // Hyphenated, not snake_case - Tarmac's own spelling.
        assert!(input.get("codegen-path").is_some(), "{text}");
        assert!(input.get("codegen-base-path").is_some(), "{text}");
    }

    /// Selecting Figma and Tarmac together should produce one pipeline, not
    /// two asset folders that ignore each other.
    #[test]
    fn a_project_with_figma_exports_reads_from_there() {
        let dir = std::env::temp_dir().join(format!("rproj-tarmac-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("figma").join("exports")).expect("create");

        assert_eq!(asset_source(&dir), ("figma/exports/**/*.png", "figma/exports"));

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");
        assert_eq!(asset_source(&dir), ("assets/**/*.png", "assets"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_existing_config_is_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("rproj-tarmac-keep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");
        fs::write(dir.join("tarmac.toml"), "name = \"mine\"\n").expect("write");

        ensure_config(&dir, "generated").expect("ensure");
        assert_eq!(
            fs::read_to_string(dir.join("tarmac.toml")).expect("read"),
            "name = \"mine\"\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_asset_folder_is_created_so_the_glob_matches_something() {
        let dir = std::env::temp_dir().join(format!("rproj-tarmac-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");

        ensure_config(&dir, "my-game").expect("ensure");
        assert!(dir.join("tarmac.toml").is_file());
        assert!(dir.join("assets").is_dir(), "the glob's folder must exist");
        let _ = fs::remove_dir_all(&dir);
    }
}
