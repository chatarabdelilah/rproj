use anyhow::{Context, Result, bail, ensure};
use rproj_core::desktop_state::DesktopState;
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub fn run(project: Option<&std::path::Path>) -> Result<()> {
    launch(project, false)
}

pub(super) fn open_project_template() -> Result<()> {
    launch(None, true)
}

fn launch(project: Option<&std::path::Path>, project_template: bool) -> Result<()> {
    let cli = std::env::current_exe().context("Could not locate rproj")?;
    let sibling = cli.with_file_name(if cfg!(windows) {
        "rproj-desktop.exe"
    } else {
        "rproj-desktop"
    });
    let desktop = locate(sibling)?;
    let mut command = desktop_command(&desktop);
    if project_template {
        check_project_template(&desktop)?;
        command.arg("--project-template");
    }
    if let Some(project) = project {
        command.arg(
            project
                .canonicalize()
                .context("Project folder is unavailable")?,
        );
    }
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("Could not open {}", desktop.display()))?;
    println!("Started rproj desktop.");
    Ok(())
}

fn desktop_command(desktop: &std::path::Path) -> Command {
    let mut command = Command::new(desktop);
    command.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}

fn check_project_template(desktop: &std::path::Path) -> Result<()> {
    let output = desktop_command(desktop)
        .arg("--self-check")
        .output()
        .with_context(|| format!("Could not check {}", desktop.display()))?;
    let status = serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap_or_default();
    ensure!(
        output.status.success()
            && status["desktop_version"].as_str() == Some(env!("CARGO_PKG_VERSION"))
            && status["matching_cli"].as_bool() == Some(true)
            && status["project_template"].as_bool() == Some(true),
        "Project Template requires a matching desktop package. Extract the current Windows portable archive and run rproj-desktop.exe once, or keep the matching executables together."
    );
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
