use std::path::Path;
use std::process::Command;
use std::{error::Error, fmt};

use anyhow::{Context, Result, bail};

use crate::catalog::tool_catalog::SYSTEM_APPS;
use crate::config::{PackageWorkflow, project_file};
use crate::graph::TestRunner;
use crate::steps::{bootstrap, git, jest, studio_plugin, toolchain, wally};
use crate::ui;

#[derive(Debug)]
pub(crate) struct RunnerFailure {
    program: String,
    code: i32,
}

impl RunnerFailure {
    pub(crate) fn exit_code(&self) -> u8 {
        u8::try_from(self.code).unwrap_or(1)
    }
}

impl fmt::Display for RunnerFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} reported test failures (exit code {})",
            self.program, self.code
        )
    }
}

impl Error for RunnerFailure {}

pub fn run(arguments: &[String]) -> Result<()> {
    let project_dir = std::env::current_dir().context("failed to read current directory")?;
    run_in(&project_dir, arguments)
}

pub(super) fn run_in(project_dir: &Path, arguments: &[String]) -> Result<()> {
    let Some(project) = project_file::load_from(project_dir)? else {
        bail!("no rproj.toml here - `rproj test` needs the recorded Testing capability");
    };
    let Some(runner) = project.test_runner() else {
        bail!("Testing is not enabled for this project; select it when creating a project");
    };
    if !project.testing_is_compatible() {
        bail!("Jest Roblox requires the Wally dependency workflow");
    }

    if project_dir.join("rokit.toml").exists() {
        toolchain::sync_installed_tools(project_dir)?;
    }
    git::sync_submodules(project_dir)?;

    match runner {
        TestRunner::TestEz => {
            if project.package_workflow == PackageWorkflow::Wally
                && project_dir.join("wally.toml").exists()
            {
                wally::sync(project_dir)?;
            }
            let (program, args) = invocation(runner, arguments);
            run_runner(program, &args, project_dir)
        }
        TestRunner::JestRoblox => {
            if uses_studio_cli(arguments) {
                ensure_jest_runtime()?;
            }
            if !project_dir.join("wally.toml").exists() {
                bail!("Jest Roblox project is missing wally.toml; run `rproj upgrade`");
            }
            jest::refresh_project(project_dir)?;
            jest::ensure_config(project_dir)?;
            wally::sync_for_project(project_dir, jest::PROJECT_FILE)?;
            let (program, args) = invocation(runner, arguments);
            run_runner(program, &args, project_dir)
        }
    }
}

fn uses_studio_cli(arguments: &[String]) -> bool {
    for (index, argument) in arguments.iter().enumerate() {
        if argument == "--backend" {
            return arguments
                .get(index + 1)
                .is_none_or(|backend| backend == "studio-cli");
        }
        if let Some(backend) = argument.strip_prefix("--backend=") {
            return backend == "studio-cli";
        }
    }
    true
}

fn ensure_jest_runtime() -> Result<()> {
    let studio = SYSTEM_APPS
        .iter()
        .find(|entry| entry.key == "studio")
        .context("Roblox Studio is missing from rproj's tool catalog")?;
    if !bootstrap::is_installed(studio) {
        bail!(
            "Roblox Studio is required for local Jest Roblox tests; install it with `rproj setup`"
        );
    }
    let plugin = studio_plugin::studio_plugins_dir()?.join("JestRobloxRunner.rbxm");
    if !plugin.is_file() {
        bail!(
            "JestRobloxRunner.rbxm is missing from Roblox Studio plugins; select it in `rproj setup`"
        );
    }
    Ok(())
}

fn run_runner(program: &str, arguments: &[String], project_dir: &Path) -> Result<()> {
    crate::interrupt::check()?;
    let shown: Vec<&str> = arguments.iter().map(String::as_str).collect();
    ui::command(program, &shown);
    let status = Command::new(program)
        .args(arguments)
        .current_dir(project_dir)
        .status()
        .with_context(|| {
            format!("failed to start `{program}`; run `rproj watch` to restore pinned tools")
        })?;
    crate::diagnostics::tool_exit(program, status);
    crate::interrupt::check()?;
    if !status.success() {
        return Err(RunnerFailure {
            program: program.to_string(),
            code: status.code().unwrap_or(1),
        }
        .into());
    }
    Ok(())
}

fn invocation(runner: TestRunner, arguments: &[String]) -> (&'static str, Vec<String>) {
    match runner {
        TestRunner::TestEz => (
            "lute",
            std::iter::once("test".to_string())
                .chain(arguments.iter().cloned())
                .collect(),
        ),
        TestRunner::JestRoblox => (
            "jest-roblox-cli",
            std::iter::once("--passWithNoTests".to_string())
                .chain(arguments.iter().cloned())
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::ProjectGraph;
    use tempfile::TempDir;

    #[test]
    fn missing_project_manifest_is_clear() {
        let dir = TempDir::new().unwrap();
        let error = run_in(dir.path(), &[]).unwrap_err().to_string();
        assert!(error.contains("rproj.toml"), "{error}");
    }

    #[test]
    fn project_without_testing_is_rejected_before_running_tools() {
        let dir = TempDir::new().unwrap();
        project_file::save_to(&ProjectGraph::default(), dir.path()).unwrap();
        let error = run_in(dir.path(), &[]).unwrap_err().to_string();
        assert!(error.contains("Testing is not enabled"), "{error}");
    }

    #[test]
    fn jest_without_wally_is_rejected_before_running_tools() {
        let dir = TempDir::new().unwrap();
        let mut project = ProjectGraph {
            package_workflow: PackageWorkflow::None,
            ..Default::default()
        };
        project.choose("test", Some("jest-roblox"));
        project_file::save_to(&project, dir.path()).unwrap();
        let error = run_in(dir.path(), &[]).unwrap_err().to_string();
        assert!(error.contains("requires the Wally"), "{error}");
    }

    #[test]
    fn runner_arguments_are_forwarded_after_required_defaults() {
        let supplied = vec!["--headed".into(), "-t".into(), "inventory".into()];
        let (program, args) = invocation(TestRunner::JestRoblox, &supplied);
        assert_eq!(program, "jest-roblox-cli");
        assert_eq!(args, ["--passWithNoTests", "--headed", "-t", "inventory"]);
        let (program, args) = invocation(TestRunner::TestEz, &supplied);
        assert_eq!(program, "lute");
        assert_eq!(args[0], "test");
        assert_eq!(&args[1..], supplied);
    }

    #[cfg(windows)]
    #[test]
    fn runner_exit_code_is_preserved() {
        let dir = TempDir::new().unwrap();
        let error =
            run_runner("cmd", &["/C".into(), "exit".into(), "7".into()], dir.path()).unwrap_err();
        let failure = error.downcast_ref::<RunnerFailure>().unwrap();
        assert_eq!(failure.exit_code(), 7);
    }

    #[test]
    fn only_studio_cli_runs_require_the_local_studio_preflight() {
        assert!(uses_studio_cli(&[]));
        assert!(uses_studio_cli(&["--backend=studio-cli".into()]));
        assert!(!uses_studio_cli(&["--backend".into(), "open-cloud".into()]));
        assert!(!uses_studio_cli(&["--backend=workspace".into()]));
    }
}
