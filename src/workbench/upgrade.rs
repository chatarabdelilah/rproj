use crate::commands::upgrade;
use anyhow::{Context, Result, ensure};
use rproj_core::upgrade::protocol::*;
use std::path::Path;

#[cfg(all(test, windows))]
mod tests;

pub(super) fn context(project: &Path) -> Result<UpgradeContext> {
    let project = project
        .canonicalize()
        .context("Cannot open the captured project")?;
    let identity = upgrade::project_identity(&project)?;
    Ok(UpgradeContext {
        protocol: UPGRADE_PROTOCOL,
        name: project
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        project,
        identity,
    })
}

pub(super) fn job(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Upgrade requires an existing absolute IPC directory"
    );
    let request: UpgradeRequest = read_json(&directory.join("request.json"))?;
    ensure!(
        request.protocol == UPGRADE_PROTOCOL
            && !request.id.is_empty()
            && request.id.len() <= 256
            && request.project.is_absolute(),
        "Unsupported Upgrade request"
    );
    crate::ui::set_verbose(true);
    let path = directory.join("report.json");
    let mut report = UpgradeReport::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, &path, &mut report);
    report.status = Some(match &result {
        Ok(()) => UpgradeStatus::Succeeded,
        Err(error) if crate::interrupt::is_cancelled(error) => UpgradeStatus::Cancelled,
        Err(_) => UpgradeStatus::Failed,
    });
    report.message = result
        .as_ref()
        .err()
        .map(|error| format!("{error:#}").chars().take(8192).collect());
    report.check(&request)?;
    write_atomic(&path, &report)?;
    result
}

fn perform(request: &UpgradeRequest, path: &Path, report: &mut UpgradeReport) -> Result<()> {
    crate::interrupt::check()?;
    match &request.action {
        UpgradeAction::Prepare(captured) => {
            ensure!(
                context(&request.project)? == *captured,
                "Captured project changed; explicitly Refresh review"
            );
            let prepared = upgrade::prepare(&request.project)?.review();
            ensure!(
                prepared.identity == captured.identity,
                "Captured project changed during preparation; explicitly Refresh review"
            );
            upgrade::verify_review(&prepared)?;
            let mut candidate = report.clone();
            candidate.prepared = Some(prepared);
            candidate.status = Some(UpgradeStatus::Succeeded);
            ensure!(
                serde_json::to_vec(&candidate)?.len() as u64 <= IPC_LIMIT,
                "Upgrade review exceeds the 2 MiB worker limit; use `rproj upgrade` in the terminal. No project changes made."
            );
            report.prepared = candidate.prepared;
        }
        UpgradeAction::Apply(review) => {
            ensure!(
                review.project == request.project,
                "Upgrade belongs to another project"
            );
            upgrade::execute_review(review, &mut || {
                report.writes = WriteState::MayHaveBegun;
                write_atomic(path, report)
            })?;
            report.writes = if review.files.is_empty() {
                WriteState::NotStarted
            } else {
                WriteState::Completed
            };
        }
    }
    Ok(())
}
