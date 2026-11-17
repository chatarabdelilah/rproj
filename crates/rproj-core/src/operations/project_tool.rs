use super::*;
use crate::project_tool_setup::{Plan, protocol::*};

impl WorkbenchClient {
    pub fn project_tool_support(&self) -> Result<()> {
        let text = query(&self.worker, &[WORKER_FLAG, "project-tool-support"], None).context(
            "Project Tool Setup is unavailable; use a matching desktop/CLI distribution",
        )?;
        ensure!(
            serde_json::from_str::<u32>(&text)? == PROJECT_TOOL_PROTOCOL,
            "Unsupported Project Tool Setup worker; use a matching distribution"
        );
        Ok(())
    }

    pub fn prepare_project_tool(
        &self,
        project: &Path,
        tool: &str,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<Plan> {
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
                "Project is a reparse point; choose a regular directory"
            );
        }
        let report = self.project_tool_operation(
            project.canonicalize()?,
            tool.into(),
            Action::Prepare,
            cancellation,
            sink,
        )?;
        ensure!(
            report.status == Some(Status::Succeeded),
            "{}",
            report
                .message
                .unwrap_or_else(|| "Could not prepare tool setup; Refresh review".into())
        );
        report.prepared.context("Tool setup returned no review")
    }

    /// No cancellation token: callers must wait for confirmed owned teardown.
    pub fn setup_project_tool(
        &self,
        plan: Plan,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<Completion> {
        let project = plan.project.clone();
        let tool = plan.tool.clone();
        let report = match self.project_tool_operation(
            project.clone(),
            tool.clone(),
            Action::Execute(plan),
            &Cancellation::default(),
            sink,
        ) {
            Ok(report) => report,
            Err(error) if error.is::<OwnedTeardownUnconfirmed>() => return Err(error),
            Err(error) => {
                return Ok(Completion {
                    project,
                    tool,
                    status: Status::Failed,
                    effects: WriteState::NotStarted,
                    outcome: None,
                    message: Some(format!(
                        "Tool setup could not start; no effects started: {error:#}"
                    )),
                });
            }
        };
        Ok(Completion {
            project,
            tool,
            status: report.status.unwrap_or(Status::Uncertain),
            effects: report.effects,
            outcome: report.outcome,
            message: report.message,
        })
    }

    fn project_tool_operation(
        &self,
        project: PathBuf,
        tool: String,
        action: Action,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<Report> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-project-tool-")
            .tempdir()?;
        let request = Request {
            protocol: PROJECT_TOOL_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing tool operation identity")?
                .to_string_lossy()
                .into_owned(),
            project,
            tool,
            action,
        };
        request.validate()?;
        ensure!(
            serde_json::to_vec(&request)?.len() as u64 <= IPC_LIMIT,
            "Tool setup request exceeds the 2 MiB worker limit; no effects started"
        );
        write_atomic(&directory.path().join("request.json"), &request)?;
        let mut initial = Report::initial(&request);
        if cancellation.is_cancelled() {
            initial.status = Some(Status::Cancelled);
            initial.message = Some("Tool setup preparation cancelled; no effects started".into());
            return Ok(initial);
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let exit = run_project_tool(&self.worker, directory.path(), cancellation, sink.clone())?;
        let checked =
            read_json::<Report>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    report.status.is_some(),
                    "Worker exited without a terminal tool setup report"
                );
                ensure!(
                    (exit == OperationOutcome::Succeeded)
                        == (report.status == Some(Status::Succeeded)),
                    "Tool setup report and worker exit disagree"
                );
                Ok(report)
            });
        let report = match checked {
            Ok(report) => report,
            Err(error) => {
                let executing = matches!(request.action, Action::Execute(_));
                initial.status = Some(if !executing && exit == OperationOutcome::Cancelled {
                    Status::Cancelled
                } else {
                    Status::Uncertain
                });
                initial.effects = if executing {
                    WriteState::Unknown
                } else {
                    WriteState::NotStarted
                };
                initial.message = Some(format!(
                    "No trustworthy tool setup completion report. {}: {error:#}",
                    if executing {
                        "Partial effects may remain; explicitly Refresh review before retrying"
                    } else {
                        "No effects were requested"
                    }
                ));
                initial
            }
        };
        sink(OperationEvent::Finished { outcome: exit });
        Ok(report)
    }
}

#[cfg(windows)]
fn run_project_tool(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    execute_owned_os(
        worker,
        &[
            WORKER_FLAG.into(),
            "project-tool-job".into(),
            directory.as_os_str().to_owned(),
        ],
        directory,
        cancellation,
        sink,
    )
}
#[cfg(not(windows))]
fn run_project_tool(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop Project Tool Setup workers are currently supported on Windows; terminal setup remains available"
    )
}
