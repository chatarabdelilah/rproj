//! Scaffolds the project's quality gate: `.luaurc`, `.lute/check.luau`
//! and the CI workflow that runs them. See `catalog::quality_checks` for
//! what goes into the check script.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use crate::catalog::quality_checks::{render_check, CI_WORKFLOW};
use crate::steps::{probe, run_in};

/// Writes `.luaurc` with `languageMode: "strict"`.
///
/// Deliberately does *not* write the `~/.lute/typedefs/<version>` aliases
/// the reference projects carry: that version is whatever lute is
/// installed, so hardcoding one would be wrong on any other machine.
/// `lute setup --with-luaurc` fills them in and merges into this file
/// rather than replacing it, preserving `languageMode`.
///
/// Merges into an existing `.luaurc` instead of overwriting, since it may
/// already carry aliases (from a previous `lute setup`) worth keeping.
pub fn ensure_luaurc(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".luaurc");

    let mut config = if path.exists() {
        let text = fs::read_to_string(&path)?;
        match serde_json::from_str::<Value>(&text) {
            Ok(Value::Object(map)) => map,
            // Don't discard a file we can't parse - .luaurc allows
            // comments that serde_json rejects.
            _ => {
                println!("check: .luaurc exists but couldn't be parsed, leaving it alone");
                return Ok(());
            }
        }
    } else {
        serde_json::Map::new()
    };

    if config.contains_key("languageMode") {
        println!("check: .luaurc already configured");
        return Ok(());
    }
    config.insert("languageMode".to_string(), json!("strict"));

    fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&Value::Object(config))?))
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote .luaurc");
    Ok(())
}

/// Writes `.lute/check.luau` from the selected tools. No-op when the
/// project selected nothing the gate can run.
pub fn ensure_check_script(project_dir: &Path, selected_tools: &[String]) -> Result<bool> {
    let Some(contents) = render_check(selected_tools) else {
        return Ok(false);
    };

    let path = project_dir.join(".lute").join("check.luau");
    if path.exists() {
        println!("check: .lute/check.luau already exists");
        return Ok(true);
    }
    fs::create_dir_all(path.parent().expect("joined path has a parent"))?;
    fs::write(&path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote .lute/check.luau");
    Ok(true)
}

pub fn ensure_ci_workflow(project_dir: &Path) -> Result<()> {
    let path = project_dir.join(".github").join("workflows").join("ci.yml");
    if path.exists() {
        println!("check: .github/workflows/ci.yml already exists");
        return Ok(());
    }
    fs::create_dir_all(path.parent().expect("joined path has a parent"))?;
    fs::write(&path, CI_WORKFLOW).with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote .github/workflows/ci.yml");
    Ok(())
}

/// Generates lute's type definitions and points `.luaurc` at them, so the
/// check script's `@std`/`@lute` requires resolve for the language server.
/// Skipped (not an error) when lute isn't callable yet - a fresh install
/// isn't always on PATH in the shell that installed it, and CI runs this
/// same command anyway.
pub fn lute_setup(project_dir: &Path) -> Result<()> {
    if !probe("lute", &["--version"]) {
        println!("skip: lute isn't on PATH yet - run `lute setup --with-luaurc` in the project once it is");
        return Ok(());
    }
    if let Err(err) = run_in("lute", &["setup", "--with-luaurc"], Some(project_dir)) {
        eprintln!("warning: `lute setup --with-luaurc` failed, continuing - {err}");
    }
    Ok(())
}

