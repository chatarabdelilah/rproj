pub mod blender;
pub mod bootstrap;
pub mod git;
pub mod gitattributes;
pub mod gitignore;
pub mod modules;
pub mod notify;
pub mod quality;
pub mod rojo;
pub mod studio_plugin;
pub mod testez;
pub mod toolchain;
pub mod vscode;
pub mod wally;

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::ui;

/// Runs a command, capturing its output rather than letting it print.
///
/// Capturing is the point: these tools are chatty in ways that are noise
/// to someone running rproj (see `ui`). The output is surfaced only when
/// the command fails - where it's the whole explanation - or under
/// `--verbose`. Errors carry context instead of exiting the process
/// outright (the JS scripts this replaced called `process.exit(1)` on any
/// failure, which took the whole run down with them).
pub fn run(program: &str, args: &[&str]) -> Result<()> {
    run_in(program, args, None)
}

pub fn run_in(program: &str, args: &[&str], dir: Option<&Path>) -> Result<()> {
    let output = capture(program, args, dir)?;
    // Shown once: on failure it's the explanation, under --verbose it's
    // what was asked for. Checking both conditions separately printed it
    // twice for a failure during a verbose run.
    if !output.success || ui::is_verbose() {
        ui::passthrough(&output.stdout, &output.stderr);
    }
    if !output.success {
        bail!("`{program} {}` failed", args.join(" "));
    }
    Ok(())
}

/// `Debug` so a test can call `unwrap_err()` on a `Result<Captured, _>`,
/// which prints the `Ok` value when it panics and therefore requires it.
#[derive(Debug)]
pub struct Captured {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

impl Captured {
    /// Both streams together, for callers that need to classify a failure
    /// by matching on a message (some tools write theirs to stdout, some
    /// to stderr, and which one isn't always stable across versions).
    pub fn combined(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Runs a command and hands back its output for the caller to classify.
/// Only a genuine failure to *spawn* is an error here - a non-zero exit is
/// data, since for several of these tools it's an expected, benign outcome
/// (a tool already installed, for instance).
pub fn capture(program: &str, args: &[&str], dir: Option<&Path>) -> Result<Captured> {
    ui::command(program, args);
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = dir {
        cmd.current_dir(dir);
    }
    let output = cmd.output().with_context(|| format!("failed to spawn `{program}`"))?;
    Ok(Captured {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
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
