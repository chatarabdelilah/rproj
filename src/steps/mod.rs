pub mod blender;
pub mod bootstrap;
pub mod gitignore;
pub mod notify;
pub mod rojo;
pub mod studio_plugin;
pub mod toolchain;
pub mod vscode;
pub mod wally;

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Run a command with inherited stdio, printing what's being run and
/// returning an error with context instead of silently exiting the whole
/// process (the old JS scripts called `process.exit(1)` on any failure).
pub fn run(program: &str, args: &[&str]) -> Result<()> {
    run_in(program, args, None)
}

pub fn run_in(program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
    println!("\n> {program} {}", args.join(" "));
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to spawn `{program}`"))?;
    if !status.success() {
        bail!("`{program} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

/// Like `run`, but a non-zero exit is reported by returning `Ok(false)`
/// instead of an error - useful for detection probes where "not found" is
/// an expected, non-fatal outcome.
pub fn probe(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
