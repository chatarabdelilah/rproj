pub mod blender;
pub mod bootstrap;
pub mod git;
pub mod gitignore;
pub mod modules;
pub mod notify;
pub mod quality;
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

/// GETs `url` with a rproj User-Agent header and returns the body as text,
/// giving a clear explanation instead of a bare "http status: 403" if
/// GitHub's unauthenticated API rate limit (60 requests/hour per IP) was
/// hit - shared across every GitHub-touching step rproj *and* rokit itself
/// make, so it's easy to reach while iterating quickly during testing.
pub fn github_get_text(url: &str) -> Result<String> {
    match ureq::get(url).header("User-Agent", "rproj").call() {
        Ok(mut response) => response.body_mut().read_to_string().context("failed to read response body"),
        Err(ureq::Error::StatusCode(403)) => bail!(
            "GitHub's unauthenticated API rate limit (60 requests/hour per IP) was hit while \
             calling {url}. Wait for it to reset (up to an hour) and try again."
        ),
        Err(err) => Err(err).with_context(|| format!("failed to call {url}")),
    }
}
