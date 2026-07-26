use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::steps::probe;

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
    for id in extension_ids {
        if installed.contains(&id.to_lowercase()) {
            println!("check: VS Code extension {id} already installed");
            continue;
        }
        if let Err(err) = install_extension(id) {
            eprintln!("warning: failed to install VS Code extension {id}, continuing - {err:#}\n");
        }
    }
    Ok(())
}

fn install_extension(id: &str) -> Result<()> {
    let output = run_code(&["--install-extension", id])
        .with_context(|| format!("failed to spawn install for {id}"))?;
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        bail!("exited with {}", output.status);
    }
    Ok(())
}
