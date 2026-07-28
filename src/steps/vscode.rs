use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::config::PackageWorkflow;
use crate::steps::probe;
use crate::ui::{self, Tally};

enum CodeInvocation {
    OnPath,
    Cmd(PathBuf),
}

/// Finds the VS Code CLI even when it's not on PATH yet. winget's default
/// (user-scope) install doesn't reliably show up on PATH for an
/// already-running shell - the same PATH gotcha as Blender/cargo elsewhere
/// in this codebase. Falls back to the standard winget install location,
/// `%LocalAppData%\Programs\Microsoft VS Code\bin\code.cmd`.
fn locate_code() -> Result<CodeInvocation> {
    if probe("code", &["--version"]) {
        return Ok(CodeInvocation::OnPath);
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA").context("LOCALAPPDATA is not set")?;
    let candidate = PathBuf::from(local_app_data)
        .join("Programs")
        .join("Microsoft VS Code")
        .join("bin")
        .join("code.cmd");
    if candidate.is_file() {
        Ok(CodeInvocation::Cmd(candidate))
    } else {
        bail!(
            "code isn't on PATH and no install was found at {} - is VS Code installed?",
            candidate.display()
        )
    }
}

/// `code.cmd` is a batch file - Windows can't execute it directly via
/// CreateProcess (Rust's `Command::new` won't implicitly wrap it either),
/// it has to be run through `cmd.exe /C`. Plain `code` on PATH doesn't need
/// this since PATH resolution there already goes through the same
/// batch-file association machinery.
fn run_code(args: &[&str]) -> Result<std::process::Output> {
    match locate_code()? {
        CodeInvocation::OnPath => Command::new("code").args(args).output().context("failed to spawn `code`"),
        CodeInvocation::Cmd(path) => {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C").arg(&path).args(args);
            cmd.output().with_context(|| format!("failed to spawn `{}` via cmd.exe", path.display()))
        }
    }
}

fn installed_extensions() -> HashSet<String> {
    run_code(&["--list-extensions"])
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Installs any extension in `extension_ids` that isn't already present.
/// Fails fast if `code` can't be found at all (one clear message instead of
/// repeating that same failure for every extension), but a single
/// extension's install failing doesn't stop the rest from being attempted.
pub fn ensure_extensions(extension_ids: &[&str]) -> Result<()> {
    locate_code()?;

    let installed = installed_extensions();
    let mut tally = Tally::new();
    for id in extension_ids {
        if installed.contains(&id.to_lowercase()) {
            tally.already(id);
            continue;
        }
        match install_extension(id) {
            Ok(()) => tally.did(id),
            Err(err) => ui::warn(&format!("VS Code extension {id} skipped - {err}")),
        }
    }
    tally.finish("VS Code extensions");
    Ok(())
}

fn install_extension(id: &str) -> Result<()> {
    let output = run_code(&["--install-extension", id])
        .with_context(|| format!("failed to spawn install for {id}"))?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.status.success() {
        ui::passthrough(
            &String::from_utf8_lossy(&output.stdout),
            &String::from_utf8_lossy(&output.stderr),
        );
        bail!("exited with {}", output.status);
    }
    Ok(())
}

/// The project's `.vscode/settings.json`, or an empty object when there
/// isn't one.
///
/// Separate from `merge_settings` so `rproj configure` can hit the same
/// refusal *before* asking thirteen questions rather than after.
pub fn read_settings(project_dir: &Path) -> Result<serde_json::Map<String, Value>> {
    let path = project_dir.join(".vscode").join("settings.json");
    if !path.exists() {
        return Ok(serde_json::Map::new());
    }
    let text = fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        return Ok(serde_json::Map::new());
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => bail!("{} exists but isn't a JSON object", path.display()),
        // VS Code tolerates comments and trailing commas here; serde_json
        // doesn't. Refuse rather than silently discarding settings we
        // couldn't parse.
        Err(err) => bail!(
            "could not parse {} ({err}). It may contain comments or trailing commas, \
             which this writer can't preserve - fix or move the file, then re-run.",
            path.display()
        ),
    }
}

/// Merges `entries` into the project's `.vscode/settings.json`.
///
/// Merging rather than replacing matters: the file routinely holds
/// unrelated editor preferences, and more than one thing writes to it
/// (`rproj new`'s scaffold and every `rproj configure` run targeting a VS
/// Code tool), so each write has to leave the other keys alone.
pub fn merge_settings(project_dir: &Path, entries: &[(&str, Value)]) -> Result<()> {
    let mut settings = read_settings(project_dir)?;
    let dir = project_dir.join(".vscode");
    fs::create_dir_all(&dir)?;
    let path = dir.join("settings.json");

    for (key, value) in entries {
        settings.insert((*key).to_string(), value.clone());
    }

    fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&Value::Object(settings))?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    ui::ok("wrote .vscode/settings.json");
    Ok(())
}

/// Globs the editor should treat as vendored third-party code.
///
/// luau-lsp already defaults both of these to `["**/_Index/**"]`, which is
/// where *Wally* puts vendored packages - so the Wally workflow gets this
/// behaviour for free and the git-submodule workflow got none of it. That
/// asymmetry is the whole reason a submodule project showed hundreds of
/// diagnostics from code it doesn't own, and offered every package twice
/// in auto-import (`modules.Charm` and `modules.submodules.Charm`).
/// Extending the defaults rather than replacing them keeps `_Index`
/// covered for projects that use both.
const VENDORED_GLOBS: &[&str] = &["**/_Index/**", "**/submodules/**"];

/// Writes the editor settings a scaffolded project needs on top of the
/// extension defaults.
pub fn ensure_project_settings(project_dir: &Path, workflow: PackageWorkflow) -> Result<()> {
    merge_settings(project_dir, &project_settings(workflow))
}

/// The editor settings a scaffolded project needs on top of the extension
/// defaults. Split from the writer so the list itself is testable.
fn project_settings(workflow: PackageWorkflow) -> Vec<(&'static str, Value)> {
    let mut entries: Vec<(&str, Value)> = vec![
        // The extension defaults this to *false*, which is the single
        // setting standing between a scaffolded project and knowing about
        // anything you make in Studio: without it the editor never opens
        // the port the companion plugin posts the DataModel to, so a Part
        // you name `testPart` is invisible to `workspace.testPart`, while
        // the Studio side still looks connected. Nothing says why.
        ("luau-lsp.studioPlugin.enabled", json!(true)),
        ("luau-lsp.sourcemap.enabled", json!(true)),
        // `rproj watch` runs `rojo sourcemap --watch` itself. Leaving the
        // extension to autogenerate as well puts two watchers on one file.
        ("luau-lsp.sourcemap.autogenerate", json!(false)),
        ("luau-lsp.sourcemap.rojoProjectFile", json!("default.project.json")),
        ("luau-lsp.sourcemap.sourcemapFile", json!("sourcemap.json")),
        // `luau-lsp.types.roblox` is the deprecated spelling of this and
        // setting both earns a deprecation squiggle in the file rproj just
        // wrote, which is a poor first impression.
        ("luau-lsp.platform.type", json!("roblox")),
        // The editor half of `.gitattributes`. That file governs what git
        // checks out; this governs what the editor *creates*, and on
        // Windows VS Code defaults to CRLF. A new file made in the editor
        // would otherwise be CRLF while `stylua.toml` says `line_endings =
        // "Unix"`, so `stylua --check` reports a diff for the whole file
        // with every line looking identical on both sides - and CI fails on
        // code that looks perfect.
        ("files.eol", json!("\n")),
    ];

    // Format with the same binary CI checks with. The extension otherwise
    // falls back to its own bundled StyLua, and two StyLua versions
    // disagreeing about formatting is how a file formatted on save fails
    // `stylua --check` on the runner.
    if let Some(path) = rokit_tool_path("stylua") {
        entries.push(("stylua.styluaPath", json!(path)));
    }

    // Empty means "find this project's own stylua.toml the normal way".
    // Written explicitly because a *user-level* `stylua.configPath` is an
    // absolute path that overrides every workspace, and one left pointing
    // at a deleted project breaks formatting in every project on the
    // machine with only `Failed to read config file: The system cannot
    // find the path specified` to go on. A workspace setting outranks it.
    // Verified against the extension's own code, which appends
    // `--config-path` only when the value is non-empty after trimming.
    entries.push(("stylua.configPath", json!("")));

    if workflow == PackageWorkflow::GitSubmodules {
        entries.push(("luau-lsp.ignoreGlobs", json!(VENDORED_GLOBS)));
        entries.push(("luau-lsp.completion.imports.ignoreGlobs", json!(VENDORED_GLOBS)));
    }
    entries
}

/// Absolute path to a rokit-managed tool's shim, or `None` if it isn't
/// there. Absolute because the editor doesn't resolve through rokit's
/// shims the way a shell on `PATH` does — and machine-specific paths are
/// fine here only because `.vscode/` is gitignored in every scaffolded
/// project.
fn rokit_tool_path(tool: &str) -> Option<String> {
    let home = directories::UserDirs::new()?.home_dir().to_path_buf();
    let shim = home.join(".rokit").join("bin").join(format!("{tool}.exe"));
    shim.is_file().then(|| shim.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// luau-lsp's own defaults only cover Wally's `_Index`; dropping it
    /// while adding `submodules` would regress projects using both.
    #[test]
    fn vendored_globs_extend_rather_than_replace_the_defaults() {
        assert!(VENDORED_GLOBS.contains(&"**/_Index/**"), "must keep Wally's vendored dir");
        assert!(VENDORED_GLOBS.contains(&"**/submodules/**"), "must add the submodule dir");
    }

    fn setting(workflow: PackageWorkflow, key: &str) -> Option<Value> {
        project_settings(workflow).into_iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// This used to write nothing at all unless the project used git
    /// submodules, so every Wally project - the common case - got whatever
    /// the machine's global settings happened to say.
    #[test]
    fn both_workflows_get_editor_settings() {
        for workflow in [PackageWorkflow::Wally, PackageWorkflow::GitSubmodules] {
            assert!(!project_settings(workflow).is_empty(), "{workflow:?} got nothing");
        }
    }

    /// The extension defaults this to false, and with it off the editor
    /// never opens the port the Studio companion plugin posts to - so
    /// nothing you create in Studio is ever known to autocomplete, with no
    /// error anywhere to say so.
    #[test]
    fn the_studio_plugin_bridge_is_turned_on() {
        for workflow in [PackageWorkflow::Wally, PackageWorkflow::GitSubmodules] {
            // The non-deprecated spelling: `luau-lsp.plugin.enabled` is
            // deprecated in favour of this one.
            assert_eq!(setting(workflow, "luau-lsp.studioPlugin.enabled"), Some(json!(true)));
            assert!(setting(workflow, "luau-lsp.plugin.enabled").is_none(), "deprecated spelling");
        }
    }

    /// A user-level `stylua.configPath` is absolute and outranks every
    /// project's own config; left pointing at a deleted project it breaks
    /// formatting everywhere, reporting only "cannot find the path
    /// specified". An empty workspace value restores the normal lookup.
    #[test]
    fn a_stale_global_stylua_config_path_is_neutralised() {
        assert_eq!(setting(PackageWorkflow::Wally, "stylua.configPath"), Some(json!("")));
    }

    /// `.gitattributes` governs what git checks out; this governs what the
    /// editor creates. Without it a file made on Windows is CRLF while
    /// stylua.toml asks for Unix endings, and `stylua --check` reports a
    /// whole-file diff in which every line looks identical.
    #[test]
    fn new_files_are_created_with_unix_line_endings() {
        assert_eq!(setting(PackageWorkflow::Wally, "files.eol"), Some(json!("
")));
    }

    /// Deprecated in favour of `luau-lsp.platform.type`, which is written
    /// instead; setting both squiggles the file rproj just wrote.
    #[test]
    fn the_deprecated_roblox_types_switch_is_not_written() {
        assert!(setting(PackageWorkflow::Wally, "luau-lsp.types.roblox").is_none());
        assert_eq!(setting(PackageWorkflow::Wally, "luau-lsp.platform.type"), Some(json!("roblox")));
    }

    /// `rproj watch` runs `rojo sourcemap --watch`; letting the extension
    /// autogenerate too puts two writers on one file.
    #[test]
    fn sourcemap_autogeneration_is_left_to_rproj_watch() {
        assert_eq!(
            setting(PackageWorkflow::Wally, "luau-lsp.sourcemap.autogenerate"),
            Some(json!(false))
        );
    }

    /// The vendored globs are the one part that really is submodule-only:
    /// luau-lsp already ignores Wally's `_Index` by default.
    #[test]
    fn vendored_globs_stay_submodule_only() {
        assert!(setting(PackageWorkflow::Wally, "luau-lsp.ignoreGlobs").is_none());
        assert!(setting(PackageWorkflow::GitSubmodules, "luau-lsp.ignoreGlobs").is_some());
    }
}
