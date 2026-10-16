use super::*;
use crate::creation::protocol::*;

impl WorkbenchClient {
    pub fn creation_context(&self) -> Result<CreationContext> {
        let text = query(&self.worker, &[WORKER_FLAG, "creation-context"], None).context("Creation is unavailable; use the matching desktop and CLI from one portable distribution")?;
        let context: CreationContext =
            serde_json::from_str(&text).context("Invalid creation context")?;
        ensure!(
            context.protocol == CREATION_PROTOCOL,
            "Unsupported creation protocol; use a matching portable distribution"
        );
        ensure!(
            context.environment.projects_root.is_absolute(),
            "Creation worker returned a relative projects root"
        );
        Ok(context)
    }

    pub fn prepare_creation(
        &self,
        request: PrepareCreation,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<PreparationOutcome> {
        let (result, report) =
            self.creation_operation(CreationAction::Prepare(request), cancellation, sink)?;
        if result == OperationOutcome::Cancelled || report.status == Some(CreationStatus::Cancelled)
        {
            return Ok(PreparationOutcome::Cancelled);
        }
        ensure!(
            result == OperationOutcome::Succeeded
                && report.status == Some(CreationStatus::Succeeded),
            "{}",
            report.error.as_deref().unwrap_or(
                "Creation preparation was cancelled or failed; nothing was created by preparation"
            )
        );
        ensure!(
            report.claim == ClaimState::NotClaimed,
            "Preparation returned an invalid directory claim"
        );
        Ok(PreparationOutcome::Prepared(Box::new(
            report
                .prepared
                .context("Preparation returned no reviewed plan")?,
        )))
    }

    pub fn create_project(
        &self,
        prepared: PreparedCreation,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<CreationOutcome> {
        let destination = prepared.plan.destination.clone();
        let (result, report) =
            self.creation_operation(CreationAction::Execute(prepared), cancellation, sink)?;
        let status = match result {
            OperationOutcome::Cancelled => CreationStatus::Cancelled,
            _ if report.status == Some(CreationStatus::Cancelled) => CreationStatus::Cancelled,
            OperationOutcome::Succeeded
                if report.status == Some(CreationStatus::Succeeded)
                    && report.claim == ClaimState::Claimed =>
            {
                CreationStatus::Succeeded
            }
            _ => CreationStatus::Failed,
        };
        Ok(CreationOutcome {
            destination,
            status,
            claim: report.claim,
            message: report.error,
        })
    }

    fn creation_operation(
        &self,
        action: CreationAction,
        cancellation: &Cancellation,
        sink: impl Fn(OperationEvent) + Send + Sync + 'static,
    ) -> Result<(OperationOutcome, CreationReport)> {
        let _guard = self
            .busy
            .try_lock()
            .map_err(|_| anyhow::anyhow!("Another operation is running"))?;
        let directory = tempfile::Builder::new()
            .prefix("rproj-creation-")
            .tempdir()?;
        let request = CreationRequest {
            protocol: CREATION_PROTOCOL,
            id: directory
                .path()
                .file_name()
                .context("Missing operation identity")?
                .to_string_lossy()
                .into_owned(),
            action,
        };
        write_atomic(&directory.path().join("request.json"), &request)?;
        if cancellation.is_cancelled() {
            return Ok((
                OperationOutcome::Cancelled,
                CreationReport::initial(request.id),
            ));
        }
        sink(OperationEvent::Started);
        let sink: Arc<dyn Fn(OperationEvent) + Send + Sync> = Arc::new(sink);
        let outcome =
            run_owned_creation(&self.worker, directory.path(), cancellation, sink.clone())?;
        // A killed worker may not have written a terminal report. An absent or
        // mismatched report never proves that a destination was left untouched.
        let checked =
            read_json::<CreationReport>(&directory.path().join("report.json")).and_then(|report| {
                report.check(&request)?;
                ensure!(
                    outcome == OperationOutcome::Cancelled || report.status.is_some(),
                    "Creation worker exited without a terminal report"
                );
                if outcome == OperationOutcome::Succeeded {
                    let valid = report.status == Some(CreationStatus::Succeeded)
                        && match &request.action {
                            CreationAction::Prepare(_) => {
                                report.claim == ClaimState::NotClaimed && report.prepared.is_some()
                            }
                            CreationAction::Execute(_) => {
                                report.claim == ClaimState::Claimed && report.prepared.is_none()
                            }
                        };
                    ensure!(
                        valid,
                        "Creation worker returned an invalid completion report"
                    );
                }
                Ok(report)
            });
        let report = match checked {
            Ok(report) => report,
            Err(_) if outcome == OperationOutcome::Cancelled => {
                let mut report = CreationReport::initial(request.id.clone());
                let preparing = matches!(&request.action, CreationAction::Prepare(_));
                report.claim = if preparing { ClaimState::NotClaimed } else { ClaimState::Unknown };
                report.status = Some(CreationStatus::Cancelled);
                report.error = Some(if preparing { "Preparation was cancelled; project creation was not started" } else { "Cancellation completed, but directory ownership could not be verified. Files may remain at the destination; inspect it before retrying" }.into());
                report
            }
            Err(error) => return Err(error).context("Creation worker returned no trustworthy report; inspect the destination before retrying because files may remain"),
        };
        sink(OperationEvent::Finished {
            outcome: outcome.clone(),
        });
        Ok((outcome, report))
    }
}

#[cfg(windows)]
fn run_owned_creation(
    worker: &Path,
    directory: &Path,
    cancellation: &Cancellation,
    sink: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    let args = [
        std::ffi::OsString::from(WORKER_FLAG),
        "creation-job".into(),
        directory.as_os_str().to_owned(),
    ];
    execute_owned_os(worker, &args, directory, cancellation, sink)
}

#[cfg(not(windows))]
fn run_owned_creation(
    _: &Path,
    _: &Path,
    _: &Cancellation,
    _: Arc<dyn Fn(OperationEvent) + Send + Sync>,
) -> Result<OperationOutcome> {
    bail!(
        "Desktop creation is currently supported on Windows; direct CLI commands remain available"
    )
}
