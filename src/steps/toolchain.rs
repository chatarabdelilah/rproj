use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::catalog::tool_catalog::{ToolKind, ROKIT_TOOLS};
use crate::steps::{capture, run_in};
use crate::ui::{self, Tally};

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
        ui::ok("rokit.toml already exists");
        return Ok(());
    }
    run_in("rokit", &["init"], Some(project_dir))
}

/// Adds every rokit tool key in `selected` to the project via `rokit add`.
/// Keys must match `ToolEntry::key` in `catalog::tool_catalog::ROKIT_TOOLS`.
/// A single tool failing (e.g. a GitHub rate limit) doesn't stop the rest
/// from being attempted, or the project scaffold that follows.
pub fn add_selected_tools(project_dir: &Path, selected: &[String]) -> Result<()> {
    add_all(Some(project_dir), selected, "rokit tools")
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
    add_all(None, selected, "rokit tools (global)")
}

fn add_all(project_dir: Option<&Path>, selected: &[String], noun: &str) -> Result<()> {
    let mut tally = Tally::new();
    for key in selected {
        let Some(entry) = ROKIT_TOOLS.iter().find(|t| t.key == key) else {
            continue;
        };
        let ToolKind::RokitTool { rokit_source } = entry.kind else {
            continue;
        };
        run_rokit_add(project_dir, rokit_source, key, &mut tally)?;
    }
    tally.finish(noun);
    Ok(())
}

/// Runs `rokit add [--global] <rokit_source>` and classifies the outcome so
/// one tool's hiccup doesn't take the rest down with it.
///
/// rokit's own output is captured rather than printed: re-adding an
/// already-present tool makes `--global` emit a five-line ERROR block
/// ("Tool already exists and can't be added", plus update/remove hints)
/// even though nothing is wrong, and across nine tools that buries every
/// line that actually matters. The outcome goes to the tally instead, and
/// the captured text is only surfaced for failures we can't explain.
///
/// Only a genuine inability to spawn `rokit` bails, since every other call
/// in the batch would fail identically anyway.
fn run_rokit_add(
    project_dir: Option<&Path>,
    rokit_source: &str,
    key: &str,
    tally: &mut Tally,
) -> Result<()> {
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
        // project-local add that follows would trust them anyway.
        let _ = capture("rokit", &["trust", rokit_source], None);
        args.push("--global");
    }
    args.push(rokit_source);

    let output = capture("rokit", &args, project_dir)?;
    if ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if output.success {
        tally.did(key);
        return Ok(());
    }

    let combined = output.combined();
    if combined.contains("already exists") {
        tally.already(key);
    } else if combined.contains("not been marked as trusted") {
        // The pre-emptive `rokit trust` above should make this
        // unreachable; kept so the message explains itself rather than
        // surfacing as a bare non-zero exit if rokit changes behaviour.
        ui::warn(&format!(
            "{key} isn't trusted by rokit - run `rokit trust {rokit_source}` once, then re-run"
        ));
    } else if combined.contains("403 Forbidden") || combined.to_lowercase().contains("rate limit") {
        ui::warn(&format!("{key} skipped - GitHub API rate limit reached"));
        ui::detail(
            "GitHub allows 60 unauthenticated API requests per hour per IP, shared across
             every call rproj and rokit make. Wait for it to reset, or run
             `rokit authenticate github` with a token to raise the limit.",
        );
    } else {
        ui::warn(&format!("{key} could not be added, continuing"));
        ui::passthrough(&output.stdout, &output.stderr);
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
            ui::ok(&format!("{filename} already configured"));
            return Ok(());
        }
    }
    fs::write(&path, default_content)?;
    ui::ok(&format!("wrote {filename}"));
    Ok(())
}

