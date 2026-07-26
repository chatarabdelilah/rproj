use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

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
/// A single tool failing (e.g. a GitHub rate limit) doesn't stop the rest
/// from being attempted, or the project scaffold that follows.
pub fn add_selected_tools(project_dir: &Path, selected: &[String]) -> Result<()> {
    for key in selected {
        let Some(entry) = ROKIT_TOOLS.iter().find(|t| t.key == key) else {
            continue;
        };
        let ToolKind::RokitTool { rokit_source } = entry.kind else {
            continue;
        };
        run_rokit_add(Some(project_dir), rokit_source, key, "")?;
    }
    Ok(())
}

/// Installs every rokit tool key in `selected` into rokit's *global* manifest
/// (`rokit add --global`, which lives at `~/.rokit/rokit.toml`) so each tool
/// is resolvable from any directory, not just inside a project that has
/// already run `rokit add` for it locally. This has to happen before
/// anything tries to invoke one of these tools outside a project context -
/// e.g. `rojo plugin install`, which otherwise fails with "Failed to find
/// tool 'rojo' in any project manifest file" the first time, before any
/// project's own rokit.toml exists yet. Per-project `add_selected_tools`
/// still runs separately so each project also pins its own exact versions.
pub fn add_global_tools(selected: &[String]) -> Result<()> {
    for key in selected {
        let Some(entry) = ROKIT_TOOLS.iter().find(|t| t.key == key) else {
            continue;
        };
        let ToolKind::RokitTool { rokit_source } = entry.kind else {
            continue;
        };
        run_rokit_add(None, rokit_source, key, " globally")?;
    }
    Ok(())
}

/// Runs `rokit add [--global] <rokit_source>`, printing output and
/// classifying the outcome so one tool's hiccup doesn't take the rest down
/// with it: re-adding an already-present tool ("Tool already exists", which
/// a project-local `rokit add` treats as a silent no-op but `--global`
/// doesn't) and GitHub's unauthenticated rate limit (60 requests/hour per
/// IP, shared across every GitHub-touching step rproj *and* rokit itself
/// make - easy to hit while iterating quickly during testing) are both
/// printed as informative, non-fatal outcomes rather than propagated.
/// Only a genuine inability to spawn `rokit` at all bails, since every
/// other call in the batch would fail identically anyway.
fn run_rokit_add(project_dir: Option<&Path>, rokit_source: &str, key: &str, scope_desc: &str) -> Result<()> {
    let mut args = vec!["add"];
    if project_dir.is_none() {
        // `rokit add --global` refuses a tool rokit hasn't seen before
        // ("The following tool has not been marked as trusted"), unlike a
        // project-local `rokit add`, which trusts implicitly as it
        // installs. Since rproj does the global adds *first*, every one of
        // them fails this way on a machine that has never installed the
        // tool - i.e. exactly the fresh-PC case this tool exists for,
        // which is invisible on a developer machine where everything is
        // already trusted. Trusting up front is not extra exposure: these
        // are catalog tools the user explicitly selected, and the
        // project-local add that follows would trust them anyway. Output
        // is captured rather than printed because this is plumbing, and
        // failures here are left for the add below to report properly.
        let _ = Command::new("rokit").args(["trust", rokit_source]).output();
        args.push("--global");
    }
    args.push(rokit_source);

    println!("\n> rokit {}", args.join(" "));
    let mut cmd = Command::new("rokit");
    cmd.args(&args);
    if let Some(dir) = project_dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().context("failed to spawn `rokit`")?;

    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if output.status.success() {
        return Ok(());
    }

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if combined.contains("already exists") {
        println!("check: {key} already added{scope_desc}");
    } else if combined.contains("not been marked as trusted") {
        // The pre-emptive `rokit trust` above should make this
        // unreachable; kept so the message explains itself rather than
        // surfacing as a bare non-zero exit if rokit changes behaviour.
        eprintln!(
            "warning: {key} isn't trusted by rokit and trusting it automatically failed. \
             Run `rokit trust {rokit_source}` once, then re-run.\n"
        );
    } else if combined.contains("403 Forbidden") || combined.to_lowercase().contains("rate limit") {
        eprintln!(
            "warning: {key} failed - GitHub's unauthenticated API rate limit (60 requests/hour \
             per IP) was likely hit. That's shared across every GitHub-touching step rproj and \
             rokit make, so it's easy to reach while iterating quickly - wait for it to reset \
             (up to an hour) and try again.\n"
        );
    } else {
        eprintln!(
            "warning: failed to add {key}{scope_desc}, continuing - `rokit {}` exited with {}\n",
            args.join(" "),
            output.status
        );
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
