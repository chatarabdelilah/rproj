use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::catalog::tool_catalog::{ToolKind, ROKIT_TOOLS};
use crate::steps::run_in;

const STYLUA_CONFIG: &str = r#"column_width = 120
indent_type = "Spaces"
indent_width = 4
quote_style = "AutoPreferDouble"
call_parentheses = "Always"
"#;

/// Syncs installed tool binaries to match an existing rokit.toml - the
/// idempotent operation for a project whose manifest already exists
/// (freshly cloned from a teammate, or created by an earlier `rproj new`).
pub fn sync_installed_tools(project_dir: &Path) -> Result<()> {
    run_in("rokit", &["install"], Some(project_dir))
}

pub fn ensure_rokit_init(project_dir: &Path) -> Result<()> {
    if project_dir.join("rokit.toml").exists() {
        println!("check: rokit.toml already exists");
        return Ok(());
    }
    run_in("rokit", &["init"], Some(project_dir))
}

/// Adds every rokit tool key in `selected` to the project via `rokit add`.
/// Keys must match `ToolEntry::key` in `catalog::tool_catalog::ROKIT_TOOLS`.
pub fn add_selected_tools(project_dir: &Path, selected: &[String]) -> Result<()> {
    for key in selected {
        let Some(entry) = ROKIT_TOOLS.iter().find(|t| t.key == key) else {
            continue;
        };
        let ToolKind::RokitTool { rokit_source } = entry.kind else {
            continue;
        };
        run_in("rokit", &["add", rokit_source], Some(project_dir))?;
    }
    Ok(())
}

/// `std` needs "roblox+testez" instead of plain "roblox" when the project
/// includes TestEZ, so selene understands its `describe`/`it` globals
/// instead of flagging them as unknown globals.
pub fn ensure_selene_config(project_dir: &Path, testez_selected: bool) -> Result<()> {
    let std_value = if testez_selected { "roblox+testez" } else { "roblox" };
    let config = format!("std = \"{std_value}\"\n");
    ensure_config_file(project_dir, "selene.toml", &config, |content| {
        content.lines().any(|l| l.trim() == format!(r#"std = "{std_value}""#))
    })
}

pub fn ensure_stylua_config(project_dir: &Path) -> Result<()> {
    ensure_config_file(project_dir, "stylua.toml", STYLUA_CONFIG, |_| true)
}

fn ensure_config_file(
    project_dir: &Path,
    filename: &str,
    default_content: &str,
    already_valid: impl Fn(&str) -> bool,
) -> Result<()> {
    let path = project_dir.join(filename);
    if path.exists() {
        let content = fs::read_to_string(&path)?;
        if already_valid(&content) {
            println!("check: {filename} already configured");
            return Ok(());
        }
    }
    fs::write(&path, default_content)?;
    println!("wrote {filename}");
    Ok(())
}
