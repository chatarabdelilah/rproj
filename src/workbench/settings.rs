use crate::commands::configure::checked;
use anyhow::{Result, ensure};
use rproj_core::tool_configuration::{SettingsDraft, protocol::*};
use std::path::Path;

#[cfg(all(test, windows))]
mod tests;

pub(super) fn job(directory: &Path) -> Result<()> {
    ensure!(
        directory.is_absolute() && directory.is_dir(),
        "Settings require an existing absolute IPC directory"
    );
    let request: SettingsRequest = read_json(&directory.join("request.json"))?;
    request.validate()?;
    let path = directory.join("report.json");
    let mut report = SettingsReport::initial(&request);
    write_atomic(&path, &report)?;
    let result = perform(&request, &path, &mut report);
    report.status = Some(match &result {
        Ok(()) => SettingsStatus::Succeeded,
        Err(_) if report.writes != WriteState::NotStarted => SettingsStatus::Uncertain,
        Err(_) => SettingsStatus::Failed,
    });
    if let Err(error) = &result {
        report.loaded = None;
        report.prepared = None;
        report.message = Some(format!("{error:#}").chars().take(8192).collect());
    }
    report.check(&request)?;
    write_atomic(&path, &report)?;
    result
}

fn perform(request: &SettingsRequest, path: &Path, report: &mut SettingsReport) -> Result<()> {
    match &request.action {
        SettingsAction::Context => {
            let context = checked::capture(&request.project, &request.tool)?;
            let draft = SettingsDraft::new(checked::values(&context)?);
            // Revalidate the exact read, including missing files, before returning it.
            checked::prepare(&context, &draft)?;
            let blocked = checked::field_problems(&context)?;
            report.loaded = Some(SettingsLoaded {
                context,
                draft,
                blocked,
            });
        }
        SettingsAction::Prepare { context, draft } => {
            report.prepared = Some(checked::prepare(context, draft)?);
        }
        SettingsAction::Save(review) => {
            // The submitted review is bounded before the mutation callback can run.
            ensure!(
                serde_json::to_vec(request)?.len() as u64 <= IPC_LIMIT,
                "Settings review exceeds the 2 MiB worker limit; nothing written"
            );
            println!(
                "Saving {} for {}",
                review.context.target()?.target.relative(),
                request.project.display()
            );
            let context = checked::execute(review, &mut || {
                report.writes = WriteState::MayHaveBegun;
                write_atomic(path, report)
            })?;
            let draft = SettingsDraft::new(checked::values(&context)?);
            let blocked = checked::field_problems(&context)?;
            report.loaded = Some(SettingsLoaded {
                context,
                draft,
                blocked,
            });
            report.writes = if review.contents.is_some() {
                WriteState::Completed
            } else {
                WriteState::NotStarted
            };
        }
    }
    let mut candidate = report.clone();
    candidate.status = Some(SettingsStatus::Succeeded);
    if serde_json::to_vec(&candidate)?.len() as u64 > IPC_LIMIT {
        // Do not truncate a review or pretend a completed replacement did not happen.
        if report.writes == WriteState::Completed {
            report.writes = WriteState::MayHaveBegun;
        }
        anyhow::bail!(
            "Settings response exceeds the 2 MiB worker limit; use terminal configuration. Reload from file if Save was started."
        );
    }
    Ok(())
}
