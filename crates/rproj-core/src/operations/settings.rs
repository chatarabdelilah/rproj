use super::*;
use crate::tool_configuration::{SettingsContext, SettingsDraft, SettingsReview, protocol::*};

impl WorkbenchClient {
    pub fn settings_support(&self) -> Result<()> {
        let text = query(&self.worker, &[WORKER_FLAG, "settings-support"], None)
            .context("Configure Tools is unavailable; use a matching desktop/CLI distribution")?;
        ensure!(
            serde_json::from_str::<u32>(&text)? == SETTINGS_PROTOCOL,
            "Unsupported settings worker; use a matching distribution"
        );
        Ok(())
    }

    pub fn settings_context(
        &self,
        project: &Path,
        tool: &str,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SettingsLoaded> {
        // Check the selected leaf before canonicalizing, as the CLI does.
        let metadata = std::fs::symlink_metadata(project)?;
        ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "Choose a regular project directory"
        );
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            ensure!(
                metadata.file_attributes() & 0x400 == 0,
                "Project is a reparse point; choose a regular project directory"
            );
        }
        let project = project.canonicalize()?;
        let report = self.settings_operation(
            project,
            tool.into(),
            SettingsAction::Context,
            cancellation,
            sink,
        )?;
        ensure!(
            report.status == Some(SettingsStatus::Succeeded),
            "{}",
            report
                .message
                .unwrap_or_else(|| "Could not load settings; retry explicitly".into())
        );
        report
            .loaded
            .context("Settings worker returned no baseline")
    }

    pub fn prepare_settings(
        &self,
        context: SettingsContext,
        draft: SettingsDraft,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SettingsReview> {
        let report = self.settings_operation(
            context.project.clone(),
            context.tool.clone(),
            SettingsAction::Prepare { context, draft },
            cancellation,
            sink,
        )?;
        ensure!(
            report.status == Some(SettingsStatus::Succeeded),
            "{}",
            report
                .message
                .unwrap_or_else(|| "Could not prepare settings; review again".into())
        );
        report
            .prepared
            .context("Settings worker returned no review")
    }

    /// Saving has no public cancellation token: normal close waits for completion.
    pub fn save_settings(
        &self,
        review: SettingsReview,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SettingsOutcome> {
        let project = review.context.project.clone();
        let tool = review.context.tool.clone();
        let report = match self.settings_operation(
            project.clone(),
            tool.clone(),
            SettingsAction::Save(review),
            &Cancellation::default(),
            sink,
        ) {
            Ok(report) => report,
            Err(e) if e.is::<OwnedTeardownUnconfirmed>() => return Err(e),
            Err(e) => {
                return Ok(SettingsOutcome {
                    project,
                    tool,
                    status: SettingsStatus::Failed,
                    writes: WriteState::NotStarted,
                    saved: None,
                    message: Some(format!(
                        "Save could not start; no settings writes were made: {e:#}"
                    )),
                });
            }
        };
        Ok(SettingsOutcome {
            project,
            tool,
            status: report.status.unwrap_or(SettingsStatus::Uncertain),
            writes: report.writes,
            saved: report.loaded,
            message: report.message,
        })
    }

    fn settings_operation(
        &self,
        project: PathBuf,
        tool: String,
        action: SettingsAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<SettingsReport> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-settings-")
            .tempdir()?;
        let request = SettingsRequest {
            protocol: SETTINGS_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing settings operation identity")?
                .to_string_lossy()
                .into_owned(),
            project,
            tool,
            action,
        };
        request.validate()?;
        ensure!(
            serde_json::to_vec(&request)?.len() as u64 <= IPC_LIMIT,
            "Settings request exceeds the 2 MiB worker limit; use terminal configuration. Nothing written."
        );
        write_atomic(&directory.path().join("request.json"), &request)?;
        let mut report = SettingsReport::initial(&request);
        if cancellation.is_cancelled() {
            report.status = Some(SettingsStatus::Cancelled);
            report.message = Some("Settings preparation cancelled; nothing written".into());
            return Ok(report);
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        // After spawn, runner errors carry OwnedTeardownUnconfirmed. Only earlier
        // errors may be interpreted as a recoverable, unchanged-file failure.
        let outcome =
            run_owned_settings(&self.worker, directory.path(), cancellation, sink.clone())?;
        let checked =
            read_json::<SettingsReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    report.status.is_some(),
                    "Worker exited without a terminal settings report"
                );
                ensure!(
                    (outcome == OperationOutcome::Succeeded)
                        == (report.status == Some(SettingsStatus::Succeeded)),
                    "Worker exit and settings report disagree"
                );
                Ok(report)
            });
        let report = match checked {
            Ok(report) => report,
            Err(error) => {
                let saving = matches!(request.action, SettingsAction::Save(_));
                report.status = Some(if !saving && outcome == OperationOutcome::Cancelled {
                    SettingsStatus::Cancelled
                } else {
                    SettingsStatus::Uncertain
                });
                report.writes = if saving {
                    WriteState::Unknown
                } else {
                    WriteState::NotStarted
                };
                report.message = Some(format!(
                    "No trustworthy settings completion report. {}: {error:#}",
                    if saving {
                        "Inspect the captured file and explicitly Reload from file before retrying"
                    } else {
                        "No settings writes were requested"
                    }
                ));
                report
            }
        };
        sink(OperationEvent::Finished { outcome });
        Ok(report)
    }
}

#[cfg(windows)]
fn run_owned_settings(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    execute_owned_os(
        worker,
        &[
            WORKER_FLAG.into(),
            "settings-job".into(),
            directory.as_os_str().to_owned(),
        ],
        directory,
        cancellation,
        sink,
    )
}
#[cfg(not(windows))]
fn run_owned_settings(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop settings workers are currently supported on Windows; terminal configuration remains available"
    )
}
