//! Writes `asphalt.toml`.
//!
//! Asphalt's config is a Roblox creator, optional codegen settings, and one
//! or more named inputs. The starter config uses PNG exports because that
//! composes with the `figma/exports/` artifact and avoids matching
//! placeholder files as assets.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::ui;

pub fn asset_source(project_dir: &Path) -> (&'static str, &'static str) {
    if project_dir.join("figma").join("exports").is_dir() {
        ("figma/exports/**/*.png", "figma/exports")
    } else {
        ("assets/**/*.png", "assets")
    }
}

pub fn render(glob: &str) -> String {
    format!(
        "\
#:schema https://raw.githubusercontent.com/jackTabsCode/asphalt/refs/heads/main/schema.json
# Asphalt uploads matching assets to Roblox and generates Luau references.
# Replace creator.id before running `asphalt sync cloud`.
#
#   asphalt sync studio             sync locally to Studio
#   asphalt sync cloud --dry-run    check what would upload
#   asphalt sync                    upload to Roblox, write asphalt.lock.toml

[creator]
type = \"user\"
id = 0

[codegen]
style = \"nested\"
strip_extensions = true
typescript = false
content = false

[inputs.assets]
path = \"{glob}\"
output_path = \"src/shared\"
output_basename = \"assets\"
"
    )
}

pub fn ensure_config(project_dir: &Path) -> Result<()> {
    let path = project_dir.join("asphalt.toml");
    if path.exists() {
        ui::ok("asphalt.toml already exists");
        return Ok(());
    }

    let (glob, base_path) = asset_source(project_dir);
    fs::write(&path, render(glob))
        .with_context(|| format!("failed to write {}", path.display()))?;

    let folder = project_dir.join(base_path);
    fs::create_dir_all(&folder)
        .with_context(|| format!("failed to create {}", folder.display()))?;
    let keep = folder.join(".gitkeep");
    if !keep.exists() {
        let _ = fs::write(&keep, b"");
    }

    ui::ok(&format!(
        "wrote asphalt.toml, reading assets from {base_path}/"
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rendered_config_parses_and_uses_asphalts_shape() {
        let text = render("assets/**/*.png");
        let parsed: toml::Table = toml::from_str(&text).expect("valid TOML");

        assert_eq!(parsed["creator"]["type"].as_str(), Some("user"));
        assert_eq!(parsed["creator"]["id"].as_integer(), Some(0));
        assert_eq!(parsed["codegen"]["style"].as_str(), Some("nested"));
        assert_eq!(parsed["codegen"]["strip_extensions"].as_bool(), Some(true));
        assert_eq!(
            parsed["inputs"]["assets"]["output_path"].as_str(),
            Some("src/shared")
        );
        assert_eq!(
            parsed["inputs"]["assets"]["output_basename"].as_str(),
            Some("assets")
        );
    }

    #[test]
    fn a_project_with_figma_exports_reads_from_there() {
        let dir = std::env::temp_dir().join(format!("rproj-asphalt-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("figma").join("exports")).expect("create");

        assert_eq!(
            asset_source(&dir),
            ("figma/exports/**/*.png", "figma/exports")
        );

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");
        assert_eq!(asset_source(&dir), ("assets/**/*.png", "assets"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_existing_config_is_never_overwritten() {
        let dir = std::env::temp_dir().join(format!("rproj-asphalt-keep-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");
        fs::write(dir.join("asphalt.toml"), "[creator]\nid = 1\n").expect("write");

        ensure_config(&dir).expect("ensure");
        assert_eq!(
            fs::read_to_string(dir.join("asphalt.toml")).expect("read"),
            "[creator]\nid = 1\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_asset_folder_is_created() {
        let dir = std::env::temp_dir().join(format!("rproj-asphalt-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create");

        ensure_config(&dir).expect("ensure");
        assert!(dir.join("asphalt.toml").is_file());
        assert!(dir.join("assets").is_dir());
        let _ = fs::remove_dir_all(&dir);
    }
}
