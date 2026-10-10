use anyhow::{Context, Result, bail};
use rproj_core::desktop_state::DesktopState;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn run(project: Option<&std::path::Path>) -> Result<()> {
    let cli = std::env::current_exe().context("Could not locate rproj")?;
    let sibling = cli.with_file_name(if cfg!(windows) {
        "rproj-desktop.exe"
    } else {
        "rproj-desktop"
    });
    let desktop = locate(sibling)?;
    let mut command = Command::new(&desktop);
    if let Some(project) = project {
        command.arg(
            project
                .canonicalize()
                .context("Project folder is unavailable")?,
        );
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .with_context(|| format!("Could not open {}", desktop.display()))?;
    println!("Started rproj desktop.");
    Ok(())
}

fn locate(sibling: PathBuf) -> Result<PathBuf> {
    if sibling.is_file() {
        return Ok(sibling);
    }
    let preferences = DesktopState::load(&DesktopState::path()?)?;
    if let Some(path) = preferences
        .desktop_executable
        .filter(|path| path.is_absolute() && path.is_file())
    {
        return Ok(path);
    }
    bail!(
        "rproj desktop is not installed. Extract the Windows portable archive and run rproj-desktop.exe once, or keep it beside rproj.exe. Cargo installation supplies the CLI; it does not download the desktop."
    )
}
