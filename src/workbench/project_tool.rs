use crate::{commands::setup::checked, steps::execution::Reporter};
use anyhow::{Result, ensure};
use rproj_core::project_tool_setup::protocol::*;
use std::path::Path;

#[cfg(all(test, windows))]
mod tests;

pub(super) fn job(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Tool setup needs an existing absolute IPC directory"
    );
    let request: Request = read_json(&directory.join("request.json"))?;
    request.validate()?;
    let path = directory.join("report.json");
    let mut report = Report::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, &path, &mut report);
    if let Err(error) = &result {
        report.prepared = None;
        report.outcome = None;
        report.status = Some(if report.effects == WriteState::NotStarted {
            Status::Failed
        } else {
            Status::Uncertain
        });
        if report.effects == WriteState::Completed {
            report.effects = WriteState::MayHaveBegun;
        }
        report.message = Some(format!("{error:#}").chars().take(8192).collect());
    }
    report.check(&request)?;
    write_atomic(&path, &report)?;
    result?;
    ensure!(
        report.status == Some(Status::Succeeded),
        "Tool setup did not fully complete; inspect the reported steps. Partial effects may remain."
    );
    Ok(())
}

fn perform(request: &Request, path: &Path, report: &mut Report) -> Result<()> {
    ensure!(
        serde_json::to_vec(request)?.len() as u64 <= IPC_LIMIT,
        "Tool setup request exceeds the 2 MiB limit; no effects started"
    );
    match &request.action {
        Action::Prepare => {
            report.prepared = Some(checked::prepare(&request.project, &request.tool)?);
            report.status = Some(Status::Succeeded);
        }
        Action::Execute(plan) => {
            println!("Setting up {} in {}", plan.tool, plan.project.display());
            let reporter = Reporter(std::sync::Arc::new(|_, text| println!("{text}")));
            let outcome = checked::execute(plan, &reporter, &mut || {
                report.effects = WriteState::MayHaveBegun;
                write_atomic(path, report)
            })?;
            let success = outcome.succeeded();
            report.status = Some(if success {
                Status::Succeeded
            } else {
                Status::Failed
            });
            report.effects = if outcome.effects_may_have_begun {
                if success {
                    WriteState::Completed
                } else {
                    WriteState::MayHaveBegun
                }
            } else {
                WriteState::NotStarted
            };
            if !success {
                report.message = Some("Tool setup did not fully complete. Partial effects may remain; Refresh review before retrying.".into());
            }
            report.outcome = Some(outcome);
        }
    }
    ensure!(
        serde_json::to_vec(report)?.len() as u64 <= IPC_LIMIT,
        "Tool setup response exceeds the 2 MiB limit; no review was truncated. Refresh review to inspect any possible effects."
    );
    Ok(())
}
