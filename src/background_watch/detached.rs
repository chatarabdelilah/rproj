use std::path::Path;

use anyhow::{Context, Result, ensure};
use winsafe::{STARTUPINFO, co, guard::CloseHandlePiGuard};

// Stdio::null() redirects standard streams but Command::spawn still inherits
// the launcher's original pipes on Windows. Use a safe wrapper that can disable
// all handle inheritance, so desktop control readers reach EOF after startup.
pub(super) fn spawn(
    exe: &Path,
    root: &Path,
    project: &Path,
    nonce: &str,
) -> Result<CloseHandlePiGuard> {
    let application = exe
        .to_str()
        .context("Watch executable path is not Unicode")?;
    let mut command = String::new();
    for arg in [
        application,
        super::INTERNAL,
        "supervisor",
        root.to_str().context("Watch session path is not Unicode")?,
        project
            .to_str()
            .context("Watch project path is not Unicode")?,
        nonce,
    ] {
        if !command.is_empty() {
            command.push(' ');
        }
        quote_argument(arg, &mut command)?;
    }
    ensure!(
        command.encode_utf16().count() < 32_767,
        "Watch command line is too long"
    );
    // Inherit the environment unchanged, including process-local tool PATHs.
    // The private startup nonce travels as an argument; it is not a control token.
    Ok(winsafe::CreateProcess(
        Some(application),
        Some(&command),
        None,
        None,
        false,
        co::CREATE::DETACHED_PROCESS | co::CREATE::BREAKAWAY_FROM_JOB,
        &[],
        None,
        &mut STARTUPINFO::default(),
    )?)
}

pub(super) fn exit_code(process: &CloseHandlePiGuard) -> Result<Option<u32>> {
    Ok(
        if process.hProcess.WaitForSingleObject(Some(0))? == co::WAIT::OBJECT_0 {
            Some(process.hProcess.GetExitCodeProcess()?)
        } else {
            None
        },
    )
}

fn quote_argument(value: &str, command: &mut String) -> Result<()> {
    ensure!(
        !value.contains('\0'),
        "Watch startup value contains a null character"
    );
    command.push('"');
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        command.extend(std::iter::repeat_n('\\', slashes));
        if character == '"' {
            command.extend(std::iter::repeat_n('\\', slashes + 1));
        }
        slashes = 0;
        command.push(character);
    }
    command.extend(std::iter::repeat_n('\\', slashes * 2));
    command.push('"');
    Ok(())
}
