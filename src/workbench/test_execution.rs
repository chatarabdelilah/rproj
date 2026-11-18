use crate::commands::test::{RunnerFailure, checked};
use anyhow::{Result, ensure};
use rproj_core::test_execution::protocol::*;
use std::path::Path;

pub(super) fn job(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Tests require an existing absolute IPC directory"
    );
    let request: Request = read_json(&directory.join("request.json"))?;
    request.validate()?;
    let path = directory.join("report.json");
    let mut report = Report::initial(&request)?;
    write_atomic(&path, &report)?;
    let result = match &request.action {
        Action::Context => checked::prepare(&request.project).map(|context| {
            report.context = Some(context);
        }),
        Action::Run { context, arguments } => checked::execute(context, arguments, &mut || {
            let mut accepted = report.clone();
            accepted.accepted = true;
            accepted.effects = WriteState::MayHaveBegun;
            write_atomic(&path, &accepted)?;
            report = accepted;
            Ok(())
        }),
    };
    match &result {
        Ok(()) => {
            report.status = Some(Status::Succeeded);
            if matches!(request.action, Action::Run { .. }) {
                report.effects = WriteState::Completed;
                report.runner_exit = Some(0);
            }
        }
        Err(error) => {
            report.status = Some(Status::Failed);
            report.context = None;
            report.runner_exit = error
                .downcast_ref::<RunnerFailure>()
                .map(RunnerFailure::runner_exit_code);
            report.message = Some(format!("{error:#}").chars().take(8192).collect());
        }
    }
    report.check(&request)?;
    write_atomic(&path, &report)?;
    result
}

#[cfg(test)]
mod tests;
