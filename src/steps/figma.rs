//! Scaffolds the `figma/` folder.
//!
//! Unlike Blender, there is nothing to generate. A Figma file lives in
//! Figma's cloud, has no local format worth writing, and cannot be created
//! from a CLI - so a `figma/scene.fig` equivalent does not exist.
//!
//! What *is* worth scaffolding is the place exported assets land, because
//! that is the part with a pipeline attached: design in Figma, export to
//! `figma/exports/`, let Tarmac upload them and generate the Luau asset
//! references. The folder plus a README documenting that is honest and
//! useful; a placeholder file pretending to be a design would not be.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::ui;

const README: &str = "\
# Design assets

The Figma file itself lives in Figma, not here - it has no local format
worth committing. This folder is for what comes *out* of it.

## exports/

Export images from Figma into `exports/`. PNG at 2x or 3x for anything
scaled in-game; SVG is not usable by Roblox.

If this project has Tarmac (`tarmac.toml` in the root), it will upload
them and generate Luau modules holding the resulting asset ids, so you
write `Assets.button` instead of pasting `rbxassetid://` numbers:

    tarmac sync --target roblox     # upload changed assets, update ids
    tarmac sync --target none       # dry run, see what would upload

Uploads go through Roblox moderation and are not usable until approved.

## Roblox UI kits

Roblox does not publish an official Figma kit. Community kits exist and
vary in quality - search the Figma community for \"Roblox UI\" and check
when the file was last updated before building on it.

## Scale

Figma works in pixels and so does Roblox's `UDim2` offset, so a frame
designed at 1920x1080 maps 1:1 to a 1920x1080 viewport. Anything meant to
adapt needs `UDim2` scale rather than offset, which is a decision to make
in Roblox rather than something a Figma export carries.
";

/// Creates `figma/exports/` and its README, if not already there.
pub fn scaffold_design_folder(project_dir: &Path) -> Result<()> {
    let root = project_dir.join("figma");
    let exports = root.join("exports");
    fs::create_dir_all(&exports)
        .with_context(|| format!("failed to create {}", exports.display()))?;

    // `.gitkeep` because an empty directory is not a thing git tracks, and
    // a teammate cloning the project should get somewhere to put exports
    // rather than having to guess the folder name.
    let keep = exports.join(".gitkeep");
    if !keep.exists() {
        fs::write(&keep, b"").with_context(|| format!("failed to write {}", keep.display()))?;
    }

    let readme = root.join("README.md");
    if readme.exists() {
        ui::ok("figma/ already scaffolded");
        return Ok(());
    }
    fs::write(&readme, README).with_context(|| format!("failed to write {}", readme.display()))?;
    ui::ok("scaffolded figma/ for exported design assets");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The README is the whole artifact, so the two things it exists to say
    /// have to be in it.
    #[test]
    fn the_readme_explains_the_export_pipeline() {
        assert!(README.contains("exports/"), "names the folder");
        assert!(README.contains("tarmac sync"), "names the command");
        assert!(
            README.contains("moderation"),
            "warns that uploads are not immediately usable"
        );
        assert!(
            README.contains("SVG is not usable"),
            "SVG is the mistake a designer will make first"
        );
    }

    #[test]
    fn scaffolding_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("rproj-figma-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");

        scaffold_design_folder(&dir).expect("first run");
        let first = fs::read_to_string(dir.join("figma/README.md")).expect("read");

        scaffold_design_folder(&dir).expect("second run");
        assert_eq!(
            fs::read_to_string(dir.join("figma/README.md")).expect("read"),
            first,
            "a second run must not rewrite it"
        );
        assert!(dir.join("figma/exports/.gitkeep").is_file());

        let _ = fs::remove_dir_all(&dir);
    }
}
